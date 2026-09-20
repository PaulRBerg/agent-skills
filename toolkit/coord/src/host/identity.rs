use std::{env, ffi::OsString};

use crate::{
    domain::{Client, Identity, ProcessFingerprint},
    error::Result,
};

use super::NativeProcessProbe;

pub(crate) fn identity_key(identity: &Identity) -> String {
    let client = match identity.client {
        Client::Codex => "codex",
        Client::Claude => "claude",
    };
    format!("{client}/{}", identity.session_id)
}

/// Resolve direct host identity. Explicit test/integration overrides win only
/// when both fields form a valid identity; Codex then precedes Claude.
pub(crate) fn from_environment() -> Option<Identity> {
    from_environment_with(|name| env::var_os(name))
}

fn from_environment_with(mut get: impl FnMut(&str) -> Option<OsString>) -> Option<Identity> {
    let override_client = get("AI_COORD_CLIENT").and_then(|value| value.into_string().ok());
    let override_session = get("AI_COORD_SESSION_ID").and_then(nonempty_utf8);
    if let (Some(client), Some(session_id)) = (override_client.as_deref(), override_session) {
        let client = match client {
            "codex" => Client::Codex,
            "claude" => Client::Claude,
            _ => return host_environment(&mut get),
        };
        return Some(Identity { client, session_id });
    }
    host_environment(&mut get)
}

fn host_environment(get: &mut impl FnMut(&str) -> Option<OsString>) -> Option<Identity> {
    if let Some(session_id) = get("CODEX_SESSION_ID").and_then(nonempty_utf8) {
        return Some(Identity { client: Client::Codex, session_id });
    }
    if let Some(session_id) = get("CODEX_THREAD_ID").and_then(nonempty_utf8) {
        return Some(Identity { client: Client::Codex, session_id });
    }
    get("CLAUDE_CODE_SESSION_ID")
        .and_then(nonempty_utf8)
        .map(|session_id| Identity { client: Client::Claude, session_id })
}

fn nonempty_utf8(value: OsString) -> Option<String> {
    value.into_string().ok().filter(|value| !value.is_empty())
}

/// Raw signal that the caller's environment looks like a delegate process
/// rather than the session that should hold lifecycle claims. Rule (a)
/// (`Override`) is always a rejection; rule (b) (`Thread`) is only confirmed
/// as a delegate by the coordinator once the ledger shows an active delegate
/// row for the resolved root session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DelegateSignal {
    /// The `AI_COORD_CLIENT`/`AI_COORD_SESSION_ID` override is in effect, and
    /// the process's own host identity resolves to a different session.
    Override { host: Identity },
    /// No override is in effect; `CODEX_SESSION_ID` and `CODEX_THREAD_ID` are
    /// both present and disagree, as they do for a native Codex subagent.
    Thread { thread_id: String },
}

/// Detect whether the current process environment looks like a delegate.
/// See [`DelegateSignal`] for what each variant means.
pub(crate) fn delegate_signal() -> Option<DelegateSignal> {
    delegate_signal_with(|name| env::var_os(name))
}

fn delegate_signal_with(mut get: impl FnMut(&str) -> Option<OsString>) -> Option<DelegateSignal> {
    let override_client = get("AI_COORD_CLIENT").and_then(|value| value.into_string().ok());
    let override_session = get("AI_COORD_SESSION_ID").and_then(nonempty_utf8);
    let override_identity = match (override_client.as_deref(), override_session) {
        (Some("codex"), Some(session_id)) => Some(Identity { client: Client::Codex, session_id }),
        (Some("claude"), Some(session_id)) => Some(Identity { client: Client::Claude, session_id }),
        _ => None,
    };

    if let Some(override_identity) = override_identity {
        return host_environment(&mut get)
            .filter(|host| *host != override_identity)
            .map(|host| DelegateSignal::Override { host });
    }

    let codex_session = get("CODEX_SESSION_ID").and_then(nonempty_utf8);
    let codex_thread = get("CODEX_THREAD_ID").and_then(nonempty_utf8);
    match (codex_session, codex_thread) {
        (Some(session), Some(thread)) if session != thread => Some(DelegateSignal::Thread { thread_id: thread }),
        _ => None,
    }
}

/// Return the starting process and at most fifteen ancestors.
pub(crate) fn process_ancestors(start_pid: Option<u32>) -> Vec<ProcessFingerprint> {
    let pid = start_pid.unwrap_or_else(parent_process_id);
    NativeProcessProbe::new().ancestors(pid)
}

/// Find the actual host process above any transient hook runner or shell.
pub(crate) fn host_process_reference(client: Client, start_pid: Option<u32>) -> Result<Option<ProcessFingerprint>> {
    NativeProcessProbe::new().host_ancestor(client, start_pid.unwrap_or_else(parent_process_id))
}

fn parent_process_id() -> u32 {
    #[cfg(unix)]
    {
        // SAFETY: getppid has no preconditions.
        unsafe { libc::getppid() as u32 }
    }
    #[cfg(not(unix))]
    {
        0
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn resolve(values: &[(&str, &str)]) -> Option<Identity> {
        let values: HashMap<&str, &str> = values.iter().copied().collect();
        from_environment_with(|name| values.get(name).map(OsString::from))
    }

    #[test]
    fn explicit_identity_override_precedes_host_variables() {
        assert_eq!(
            resolve(&[
                ("AI_COORD_CLIENT", "claude"),
                ("AI_COORD_SESSION_ID", "override"),
                ("CODEX_SESSION_ID", "codex-root"),
                ("CODEX_THREAD_ID", "codex"),
                ("CLAUDE_CODE_SESSION_ID", "claude"),
            ]),
            Some(Identity { client: Client::Claude, session_id: "override".to_owned() })
        );
    }

    #[test]
    fn incomplete_or_invalid_override_falls_through_to_codex_root_then_legacy_thread_then_claude() {
        assert_eq!(
            resolve(&[
                ("AI_COORD_CLIENT", "invalid"),
                ("AI_COORD_SESSION_ID", "override"),
                ("CODEX_SESSION_ID", "codex-root"),
                ("CODEX_THREAD_ID", "codex-thread"),
                ("CLAUDE_CODE_SESSION_ID", "claude"),
            ]),
            Some(Identity { client: Client::Codex, session_id: "codex-root".to_owned() })
        );
        assert_eq!(
            resolve(&[("CODEX_THREAD_ID", "codex-thread"), ("CLAUDE_CODE_SESSION_ID", "claude")]),
            Some(Identity { client: Client::Codex, session_id: "codex-thread".to_owned() })
        );
        assert_eq!(
            resolve(&[("AI_COORD_CLIENT", "codex"), ("CLAUDE_CODE_SESSION_ID", "claude")]),
            Some(Identity { client: Client::Claude, session_id: "claude".to_owned() })
        );
    }

    #[test]
    fn identity_key_uses_stable_provider_prefix() {
        assert_eq!(identity_key(&Identity { client: Client::Codex, session_id: "abc".to_owned() }), "codex/abc");
    }

    fn signal(values: &[(&str, &str)]) -> Option<DelegateSignal> {
        let values: HashMap<&str, &str> = values.iter().copied().collect();
        delegate_signal_with(|name| values.get(name).map(OsString::from))
    }

    #[test]
    fn override_matching_host_identity_reports_no_delegate_signal() {
        assert_eq!(
            signal(&[("AI_COORD_CLIENT", "codex"), ("AI_COORD_SESSION_ID", "parent"), ("CODEX_SESSION_ID", "parent")]),
            None
        );
    }

    #[test]
    fn override_with_differing_host_identity_reports_rule_a() {
        assert_eq!(
            signal(&[("AI_COORD_CLIENT", "codex"), ("AI_COORD_SESSION_ID", "parent"), ("CODEX_SESSION_ID", "child")]),
            Some(DelegateSignal::Override { host: Identity { client: Client::Codex, session_id: "child".to_owned() } })
        );
    }

    #[test]
    fn override_with_no_host_environment_reports_no_delegate_signal() {
        assert_eq!(signal(&[("AI_COORD_CLIENT", "codex"), ("AI_COORD_SESSION_ID", "parent")]), None);
    }

    #[test]
    fn equal_codex_session_and_thread_report_no_delegate_signal() {
        assert_eq!(signal(&[("CODEX_SESSION_ID", "root"), ("CODEX_THREAD_ID", "root")]), None);
    }

    #[test]
    fn differing_codex_session_and_thread_report_rule_b() {
        assert_eq!(
            signal(&[("CODEX_SESSION_ID", "root"), ("CODEX_THREAD_ID", "child")]),
            Some(DelegateSignal::Thread { thread_id: "child".to_owned() })
        );
    }

    #[test]
    fn claude_host_only_reports_no_delegate_signal() {
        assert_eq!(signal(&[("CLAUDE_CODE_SESSION_ID", "claude")]), None);
    }
}

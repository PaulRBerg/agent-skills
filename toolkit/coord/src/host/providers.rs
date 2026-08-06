use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::domain::{
    Client, Identity, InventoryResult, ProcessFingerprint, ProcessProbe, ProviderReport, SessionState,
};

use super::{git_root, run_output_timeout};

pub(crate) const INVENTORY_CACHE_SECONDS: f64 = 2.0;
pub(crate) const CLAUDE_INVENTORY_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const CLAUDE_PROVIDER_SOURCE: &str = "claude-agents-json";
pub(crate) const CODEX_PROVIDER_SOURCE: &str = "hook-ledger";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderContext {
    pub(crate) codex_executable: Option<PathBuf>,
    pub(crate) claude_executable: Option<PathBuf>,
    pub(crate) codex_home: PathBuf,
    pub(crate) claude_config_dir: PathBuf,
    pub(crate) cache_key: String,
}

impl ProviderContext {
    pub(crate) fn discover() -> Self {
        let codex_executable = discover_executable("codex");
        let claude_executable = discover_executable("claude");
        let home = home_dir().unwrap_or_else(|| PathBuf::from("."));
        let codex_home = env::var_os("CODEX_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".codex"));
        let claude_config_dir = env::var_os("CLAUDE_CONFIG_DIR")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".claude"));
        Self::new(codex_executable, claude_executable, codex_home, claude_config_dir)
    }

    pub(crate) fn new(
        codex_executable: Option<PathBuf>,
        claude_executable: Option<PathBuf>,
        codex_home: PathBuf,
        claude_config_dir: PathBuf,
    ) -> Self {
        let codex_executable = codex_executable.map(|path| resolved_context_path(&path));
        let claude_executable = claude_executable.map(|path| resolved_context_path(&path));
        let codex_home = resolved_context_path(&codex_home);
        let claude_config_dir = resolved_context_path(&claude_config_dir);
        let cache_key = provider_context_key(
            codex_executable.as_deref(),
            claude_executable.as_deref(),
            &codex_home,
            &claude_config_dir,
        );
        Self { codex_executable, claude_executable, codex_home, claude_config_dir, cache_key }
    }

    pub(crate) fn codex_hooks_path(&self) -> PathBuf {
        self.codex_home.join("hooks.json")
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct CodexHookLedgerEvidence {
    pub(crate) hooks_ok: bool,
    pub(crate) hooks_error: Option<String>,
    pub(crate) missing_hooks: Vec<String>,
    pub(crate) last_hook_error_code: Option<String>,
    pub(crate) trust_ok: bool,
    pub(crate) trust_error: Option<String>,
}

/// Convert owned-hook, hook-ledger, and trust evidence into provider health.
/// App-server thread status is deliberately absent: it is not cross-process
/// liveness evidence for Codex sessions.
pub(crate) fn codex_provider_report(executable: Option<&Path>, evidence: &CodexHookLedgerEvidence) -> ProviderReport {
    if executable.is_none() {
        return provider_report(Client::Codex, true, CODEX_PROVIDER_SOURCE, false, 0, None);
    }
    if !evidence.hooks_ok {
        let mut details = Vec::new();
        if let Some(error) = evidence.hooks_error.as_deref().filter(|value| !value.is_empty()) {
            details.push(error.to_owned());
        }
        if !evidence.missing_hooks.is_empty() {
            let mut missing = evidence.missing_hooks.clone();
            missing.sort();
            details.push(format!("missing or invalid hooks: {}", missing.join(", ")));
        }
        let error =
            if details.is_empty() { "hook configuration could not be verified".to_owned() } else { details.join("; ") };
        return provider_report(Client::Codex, false, CODEX_PROVIDER_SOURCE, true, 0, Some(error));
    }
    if let Some(code) = evidence.last_hook_error_code.as_deref().filter(|value| !value.is_empty()) {
        return provider_report(
            Client::Codex,
            false,
            CODEX_PROVIDER_SOURCE,
            true,
            0,
            Some(format!("last hook error: {code}")),
        );
    }
    if !evidence.trust_ok {
        let detail = evidence
            .trust_error
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or("app-server hook trust could not be verified");
        return provider_report(
            Client::Codex,
            false,
            CODEX_PROVIDER_SOURCE,
            true,
            0,
            Some(format!("hook trust: {detail}")),
        );
    }
    provider_report(Client::Codex, true, CODEX_PROVIDER_SOURCE, true, 0, None)
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ClaudeSessionObservation {
    pub(crate) identity: Identity,
    pub(crate) cwd: PathBuf,
    pub(crate) repo_root: Option<PathBuf>,
    pub(crate) state: SessionState,
    pub(crate) name: Option<String>,
    pub(crate) waiting_for: Option<String>,
    pub(crate) pid: Option<u32>,
    pub(crate) fingerprint: Option<ProcessFingerprint>,
    pub(crate) started_at: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ClaudeNormalization {
    pub(crate) sessions: Vec<ClaudeSessionObservation>,
    pub(crate) dropped: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ClaudeInventoryObservation {
    pub(crate) report: ProviderReport,
    pub(crate) sessions: Vec<ClaudeSessionObservation>,
    /// Only an authoritative observation may replace the ledger's Claude rows.
    pub(crate) authoritative: bool,
}

pub(crate) fn collect_claude_inventory(
    executable: Option<&Path>,
    probe: &dyn ProcessProbe,
) -> ClaudeInventoryObservation {
    let Some(executable) = executable else {
        return ClaudeInventoryObservation {
            report: provider_report(Client::Claude, true, CLAUDE_PROVIDER_SOURCE, false, 0, None),
            sessions: Vec::new(),
            authoritative: false,
        };
    };
    let output = match run_output_timeout(Command::new(executable).args(["agents", "--json"]), CLAUDE_INVENTORY_TIMEOUT)
    {
        Ok(output) => output,
        Err(error) => {
            return ClaudeInventoryObservation {
                report: provider_report(
                    Client::Claude,
                    false,
                    CLAUDE_PROVIDER_SOURCE,
                    true,
                    0,
                    Some(error.to_string()),
                ),
                sessions: Vec::new(),
                authoritative: false,
            };
        }
    };
    if !output.status.success() {
        let stderr = bounded_detail(&String::from_utf8_lossy(&output.stderr));
        let detail = if stderr.is_empty() { format!("exit {}", output.status.code().unwrap_or(-1)) } else { stderr };
        return ClaudeInventoryObservation {
            report: provider_report(Client::Claude, false, CLAUDE_PROVIDER_SOURCE, true, 0, Some(detail)),
            sessions: Vec::new(),
            authoritative: false,
        };
    }
    let payload: Value = match serde_json::from_slice(&output.stdout) {
        Ok(payload) => payload,
        Err(error) => {
            return ClaudeInventoryObservation {
                report: provider_report(
                    Client::Claude,
                    false,
                    CLAUDE_PROVIDER_SOURCE,
                    true,
                    0,
                    Some(format!("invalid JSON: {error}")),
                ),
                sessions: Vec::new(),
                authoritative: false,
            };
        }
    };
    let normalized = normalize_claude_sessions(&payload, probe);
    let authoritative = normalized.dropped == 0;
    ClaudeInventoryObservation {
        report: provider_report(Client::Claude, true, CLAUDE_PROVIDER_SOURCE, true, normalized.dropped, None),
        sessions: normalized.sessions,
        authoritative,
    }
}

pub(crate) fn normalize_claude_sessions(payload: &Value, probe: &dyn ProcessProbe) -> ClaudeNormalization {
    let Some(values) = payload.as_array() else {
        return ClaudeNormalization { sessions: Vec::new(), dropped: 1 };
    };
    let mut sessions = Vec::new();
    let mut dropped = 0;
    for raw in values {
        let Some(raw) = raw.as_object() else {
            dropped += 1;
            continue;
        };
        let state_value = raw
            .get("state")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .or_else(|| raw.get("status").and_then(Value::as_str));
        let Some(state_value) = state_value else {
            dropped += 1;
            continue;
        };
        let state_value = state_value.to_ascii_lowercase();
        if matches!(state_value.as_str(), "completed" | "done" | "failed" | "stopped") {
            continue;
        }
        let session_id = raw
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .or_else(|| raw.get("id").and_then(Value::as_str).filter(|value| !value.is_empty()));
        let cwd = raw.get("cwd").and_then(Value::as_str).filter(|value| !value.is_empty());
        let started_at = raw.get("startedAt").and_then(timestamp);
        let (Some(session_id), Some(cwd), Some(started_at)) = (session_id, cwd, started_at) else {
            dropped += 1;
            continue;
        };
        if cwd.contains('\0') {
            dropped += 1;
            continue;
        }
        let cwd = PathBuf::from(cwd);
        let pid = raw
            .get("pid")
            .and_then(Value::as_u64)
            .filter(|pid| *pid > 0 && *pid <= u32::MAX as u64)
            .map(|pid| pid as u32);
        let fingerprint = pid.and_then(|pid| probe.fingerprint(pid).ok());
        sessions.push(ClaudeSessionObservation {
            identity: Identity { client: Client::Claude, session_id: session_id.to_owned() },
            repo_root: git_root(&cwd),
            cwd,
            state: match state_value.as_str() {
                "busy" | "working" => SessionState::Working,
                "blocked" | "waiting" => SessionState::Waiting,
                "idle" => SessionState::Idle,
                _ => SessionState::Unknown,
            },
            name: raw.get("name").and_then(Value::as_str).map(str::to_owned),
            waiting_for: raw.get("waitingFor").and_then(Value::as_str).map(str::to_owned),
            pid,
            fingerprint,
            started_at,
        });
    }
    ClaudeNormalization { sessions, dropped }
}

pub(crate) fn inventory_result(providers: Vec<ProviderReport>) -> InventoryResult {
    let complete = providers.iter().all(|report| !report.enabled || (report.ok && report.dropped == 0));
    InventoryResult { complete, providers }
}

pub(crate) fn discover_executable(name: &str) -> Option<PathBuf> {
    if name.contains(std::path::MAIN_SEPARATOR) {
        let path = PathBuf::from(name);
        return is_executable(&path).then(|| resolved_context_path(&path));
    }
    let search_path = env::var_os("PATH")?;
    env::split_paths(&search_path)
        .map(|directory| directory.join(name))
        .find(|candidate| is_executable(candidate))
        .map(|candidate| resolved_context_path(&candidate))
}

fn provider_report(
    client: Client,
    ok: bool,
    source: &str,
    enabled: bool,
    dropped: usize,
    error: Option<String>,
) -> ProviderReport {
    ProviderReport { client, ok, source: source.to_owned(), enabled, dropped, error }
}

fn provider_context_key(
    codex_executable: Option<&Path>,
    claude_executable: Option<&Path>,
    codex_home: &Path,
    claude_config_dir: &Path,
) -> String {
    let values = [
        ("claude_config_dir", Some(claude_config_dir)),
        ("claude_executable", claude_executable),
        ("codex_executable", codex_executable),
        ("codex_home", Some(codex_home)),
    ];
    let mut digest = Sha256::new();
    digest.update(b"ai-coord-provider-context-v1\0");
    for (key, value) in values {
        digest.update(key.as_bytes());
        digest.update(b"=");
        if let Some(value) = value {
            digest.update(value.as_os_str().as_encoded_bytes());
        }
        digest.update(b"\0");
    }
    hex_bytes(&digest.finalize())
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn resolved_context_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_owned()
        } else {
            env::current_dir().map_or_else(|_| path.to_owned(), |cwd| cwd.join(path))
        }
    })
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").filter(|value| !value.is_empty()).map(PathBuf::from)
}

fn bounded_detail(value: &str) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= 240 {
        return collapsed;
    }
    let mut result: String = collapsed.chars().take(239).collect();
    result.push('…');
    result
}

fn timestamp(value: &Value) -> Option<f64> {
    if let Some(value) = value.as_f64() {
        let result = if value > 10_000_000_000.0 { value / 1000.0 } else { value };
        return result.is_finite().then_some(result);
    }
    value.as_str().and_then(parse_iso_timestamp)
}

fn parse_iso_timestamp(value: &str) -> Option<f64> {
    let (date, rest) = value.split_once(['T', ' '])?;
    let mut date_parts = date.split('-');
    let year = date_parts.next()?;
    if year.len() != 4 || !year.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let year = year.parse::<i32>().ok()?;
    let month = date_parts.next()?.parse::<u32>().ok()?;
    let day = date_parts.next()?.parse::<u32>().ok()?;
    if date_parts.next().is_some() || month == 0 || month > 12 || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    let (time, offset_seconds) = if let Some(time) = rest.strip_suffix(['Z', 'z']) {
        (time, 0_i64)
    } else if let Some(index) =
        rest.char_indices().skip(1).find_map(|(index, value)| matches!(value, '+' | '-').then_some(index))
    {
        let sign = if rest.as_bytes()[index] == b'+' { 1_i64 } else { -1_i64 };
        let offset = &rest[index + 1..];
        let (hours, minutes) = offset.split_once(':')?;
        let hours = hours.parse::<i64>().ok()?;
        let minutes = minutes.parse::<i64>().ok()?;
        if hours > 23 || minutes > 59 {
            return None;
        }
        (&rest[..index], sign * (hours * 3600 + minutes * 60))
    } else {
        (rest, 0_i64)
    };
    let mut time_parts = time.split(':');
    let hour = time_parts.next()?.parse::<u32>().ok()?;
    let minute = time_parts.next()?.parse::<u32>().ok()?;
    let second = time_parts.next()?.parse::<f64>().ok()?;
    if time_parts.next().is_some() || hour > 23 || minute > 59 || !(0.0..60.0).contains(&second) || !second.is_finite()
    {
        return None;
    }
    let days = days_from_civil(year, month, day);
    Some(days as f64 * 86_400.0 + hour as f64 * 3600.0 + minute as f64 * 60.0 + second - offset_seconds as f64)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

// Howard Hinnant's civil-date conversion, returning days since 1970-01-01.
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = month as i32;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    (era * 146_097 + day_of_era - 719_468) as i64
}

#[cfg(test)]
mod tests {
    use std::{fs, os::unix::fs::PermissionsExt};

    use tempfile::TempDir;

    use super::*;
    use crate::{
        domain::{ProcessLiveness, ProcessProbe},
        error::{AppError, Result},
    };

    #[derive(Debug)]
    struct FakeProbe;

    impl ProcessProbe for FakeProbe {
        fn fingerprint(&self, pid: u32) -> Result<ProcessFingerprint> {
            Ok(ProcessFingerprint { pid, start_token: Some(format!("token-{pid}")) })
        }

        fn liveness(&self, _fingerprint: &ProcessFingerprint) -> ProcessLiveness {
            ProcessLiveness::Alive
        }
    }

    #[test]
    fn codex_report_uses_hook_ledger_health_in_failure_order() {
        let executable = Path::new("/bin/codex");
        assert_eq!(
            codex_provider_report(None, &CodexHookLedgerEvidence::default()),
            provider_report(Client::Codex, true, CODEX_PROVIDER_SOURCE, false, 0, None)
        );
        let invalid_hooks = CodexHookLedgerEvidence {
            hooks_error: Some("invalid hook configuration".to_owned()),
            missing_hooks: vec!["Stop".to_owned(), "SessionStart".to_owned()],
            ..CodexHookLedgerEvidence::default()
        };
        assert_eq!(
            codex_provider_report(Some(executable), &invalid_hooks).error.as_deref(),
            Some("invalid hook configuration; missing or invalid hooks: SessionStart, Stop")
        );
        let hook_failure = CodexHookLedgerEvidence {
            hooks_ok: true,
            last_hook_error_code: Some("hook_error".to_owned()),
            ..CodexHookLedgerEvidence::default()
        };
        assert_eq!(
            codex_provider_report(Some(executable), &hook_failure).error.as_deref(),
            Some("last hook error: hook_error")
        );
        let trusted = CodexHookLedgerEvidence { hooks_ok: true, trust_ok: true, ..Default::default() };
        assert!(codex_provider_report(Some(executable), &trusted).ok);
    }

    #[test]
    fn normalizes_live_terminal_unknown_and_malformed_claude_rows() {
        let payload = serde_json::json!([
            {
                "sessionId": "one",
                "cwd": "/tmp",
                "state": "busy",
                "startedAt": 1_700_000_000_000_u64,
                "pid": 42,
                "name": "worker"
            },
            {
                "id": "two",
                "cwd": "/tmp",
                "status": "novel",
                "startedAt": "2026-08-02T00:00:00Z"
            },
            {"id": "done", "cwd": "/tmp", "status": "completed", "startedAt": 1},
            {"id": "bad", "cwd": "/tmp", "status": "working", "startedAt": "not-a-time"},
            {"bad": true}
        ]);
        let normalized = normalize_claude_sessions(&payload, &FakeProbe);

        assert_eq!(normalized.dropped, 2);
        assert_eq!(normalized.sessions.len(), 2);
        assert_eq!(normalized.sessions[0].state, SessionState::Working);
        assert_eq!(
            normalized.sessions[0].fingerprint,
            Some(ProcessFingerprint { pid: 42, start_token: Some("token-42".to_owned()) })
        );
        assert_eq!(normalized.sessions[1].state, SessionState::Unknown);
        assert_eq!(normalized.sessions[1].started_at, 1_785_628_800.0);
    }

    #[test]
    fn complete_claude_payload_is_authoritative_but_dropped_rows_are_not() {
        let temp = TempDir::new().unwrap();
        let executable = temp.path().join("claude");
        fs::write(
            &executable,
            "#!/bin/sh\nprintf '%s' '[{\"id\":\"done\",\"cwd\":\"/tmp\",\"state\":\"done\",\"startedAt\":1}]'\n",
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        let complete = collect_claude_inventory(Some(&executable), &FakeProbe);
        assert!(complete.authoritative);
        assert!(complete.sessions.is_empty());
        assert_eq!(complete.report.dropped, 0);

        fs::write(&executable, "#!/bin/sh\nprintf '%s' '[{\"bad\":true}]'\n").unwrap();
        let malformed = collect_claude_inventory(Some(&executable), &FakeProbe);
        assert!(!malformed.authoritative);
        assert_eq!(malformed.report.dropped, 1);
        assert!(!inventory_result(vec![malformed.report]).complete);
    }

    #[test]
    fn invalid_top_level_claude_payload_is_incomplete() {
        let normalized = normalize_claude_sessions(&serde_json::json!({"agents": []}), &FakeProbe);
        assert_eq!(normalized, ClaudeNormalization { sessions: Vec::new(), dropped: 1 });
    }

    #[test]
    fn provider_context_key_changes_with_executable_or_config_context() {
        let first = ProviderContext::new(
            Some(PathBuf::from("/bin/codex")),
            None,
            PathBuf::from("/tmp/codex"),
            PathBuf::from("/tmp/claude"),
        );
        let same = ProviderContext::new(
            Some(PathBuf::from("/bin/codex")),
            None,
            PathBuf::from("/tmp/codex"),
            PathBuf::from("/tmp/claude"),
        );
        let changed = ProviderContext::new(None, None, PathBuf::from("/tmp/codex"), PathBuf::from("/tmp/claude"));
        assert_eq!(first.cache_key, same.cache_key);
        assert_ne!(first.cache_key, changed.cache_key);
    }

    #[test]
    fn iso_timestamp_honors_offsets_and_leap_days() {
        assert_eq!(parse_iso_timestamp("1970-01-01T01:00:00+01:00"), Some(0.0));
        assert_eq!(parse_iso_timestamp("2024-02-29T00:00:00Z"), Some(1_709_164_800.0));
        assert_eq!(parse_iso_timestamp("2023-02-29T00:00:00Z"), None);
        assert_eq!(parse_iso_timestamp("2147483647-01-01T00:00:00Z"), None);
    }

    #[test]
    fn fake_probe_error_does_not_drop_otherwise_valid_row() {
        #[derive(Debug)]
        struct DeniedProbe;
        impl ProcessProbe for DeniedProbe {
            fn fingerprint(&self, _pid: u32) -> Result<ProcessFingerprint> {
                Err(AppError::operational("denied"))
            }
            fn liveness(&self, _fingerprint: &ProcessFingerprint) -> ProcessLiveness {
                ProcessLiveness::Unknown
            }
        }
        let payload = serde_json::json!([
            {"id": "one", "cwd": "/tmp", "state": "idle", "startedAt": 1, "pid": 42}
        ]);
        let normalized = normalize_claude_sessions(&payload, &DeniedProbe);
        assert_eq!(normalized.dropped, 0);
        assert_eq!(normalized.sessions[0].pid, Some(42));
        assert_eq!(normalized.sessions[0].fingerprint, None);
    }
}

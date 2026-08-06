use std::{fs, sync::Arc, time::Duration};

use serde_json::json;
use tempfile::TempDir;

use super::*;
use crate::{
    coordinator::{Clock, InventoryObservation, ProviderInventory},
    domain::{InventoryResult, ProcessFingerprint, ProcessLiveness, ProcessProbe, ProviderReport},
    state::Store,
};

struct AliveProbe;
impl ProcessProbe for AliveProbe {
    fn fingerprint(&self, pid: u32) -> Result<ProcessFingerprint> {
        Ok(ProcessFingerprint { pid, start_token: Some(format!("{pid}")) })
    }
    fn liveness(&self, _fingerprint: &ProcessFingerprint) -> ProcessLiveness {
        ProcessLiveness::Alive
    }
}
struct FixedClock;
impl Clock for FixedClock {
    fn wall(&self) -> f64 {
        100.0
    }
    fn monotonic(&self) -> f64 {
        100.0
    }
    fn sleep(&self, _duration: Duration) {}
}
struct Static(bool);
impl ProviderInventory for Static {
    fn cache_key(&self) -> &str {
        "hooks-static"
    }
    fn refresh(&mut self, _store: &Store, _probe: &dyn ProcessProbe) -> Result<InventoryObservation> {
        let providers = [Client::Codex, Client::Claude]
            .into_iter()
            .map(|client| ProviderReport {
                client,
                ok: self.0,
                source: "static".into(),
                enabled: true,
                dropped: 0,
                error: (!self.0).then(|| "incomplete".to_owned()),
            })
            .collect();
        Ok(InventoryObservation {
            result: InventoryResult { complete: self.0, providers },
            claude_sessions: vec![],
            claude_authoritative: false,
        })
    }
}

fn runtime_with_coverage(temp: &TempDir, complete: bool) -> (Coordinator, PathBuf) {
    let repo = temp.path().join("repo");
    fs::create_dir(&repo).unwrap();
    assert!(std::process::Command::new("git").args(["init", "-q"]).current_dir(&repo).status().unwrap().success());
    let coordinator = Coordinator::with_components(
        Store::open(temp.path().join("state.db")).unwrap(),
        Box::new(Static(complete)),
        Arc::new(AliveProbe),
        Arc::new(FixedClock),
    );
    (coordinator, repo)
}

fn runtime(temp: &TempDir) -> (Coordinator, PathBuf) {
    runtime_with_coverage(temp, true)
}

#[test]
fn malformed_supported_hook_fails_open_without_payload_leak() {
    let temp = TempDir::new().unwrap();
    let (coordinator, _) = runtime(&temp);
    let output = HookRuntime::new(&coordinator).ingest(
        "codex",
        &json!({
            "hook_event_name": "Stop", "prompt": "SECRET", "tool_input": {"token": "SECRET"}
        }),
    );
    assert_eq!(output, "{}");
    assert!(!output.contains("SECRET"));
    assert_eq!(coordinator.store().unwrap().hook_health().unwrap()[0].last_error_code.as_deref(), Some("hook_error"));
}

#[test]
fn unknown_event_creates_no_session_or_health_row() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    assert_eq!(
        HookRuntime::new(&coordinator).ingest(
            "codex",
            &json!({
                "session_id": "phantom", "cwd": repo, "hook_event_name": "UnexpectedEvent"
            })
        ),
        ""
    );
    let store = coordinator.store().unwrap();
    assert!(store.sessions().unwrap().is_empty());
    assert!(store.hook_health().unwrap().is_empty());
}

#[test]
fn permission_mode_is_whitelisted_and_unknown_clears_it() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    runtime.ingest(
        "claude",
        &json!({"session_id":"one", "cwd":repo, "hook_event_name":"UserPromptSubmit", "permission_mode":"plan"}),
    );
    assert_eq!(
        coordinator
            .store()
            .unwrap()
            .session(&Identity { client: Client::Claude, session_id: "one".into() })
            .unwrap()
            .unwrap()
            .permission_mode
            .as_deref(),
        Some("plan")
    );
    runtime.ingest(
        "claude",
        &json!({"session_id":"one", "cwd":repo, "hook_event_name":"Stop", "permission_mode":"private-secret"}),
    );
    assert_eq!(
        coordinator
            .store()
            .unwrap()
            .session(&Identity { client: Client::Claude, session_id: "one".into() })
            .unwrap()
            .unwrap()
            .permission_mode,
        None
    );
}

#[test]
fn prompt_presence_is_counts_only() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    runtime.ingest("codex", &json!({"session_id":"peer-secret-id", "cwd":repo, "hook_event_name":"SessionStart"}));
    let output = runtime.ingest(
        "claude",
        &json!({"session_id":"self", "cwd":repo, "hook_event_name":"UserPromptSubmit", "prompt":"PRIVATE"}),
    );
    assert!(output.contains("Peers: 1"));
    assert!(!output.contains("peer-secret-id"));
    assert!(!output.contains("PRIVATE"));
}

#[test]
fn claude_exit_plan_hook_is_obsolete_and_creates_no_work() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    HookRuntime::new(&coordinator).ingest(
        "claude",
        &json!({
            "session_id":"planner", "cwd":repo, "hook_event_name":"PostToolUse", "tool_name":"ExitPlanMode",
            "tool_response":{"plan":"private preface\n# Ship safe coordinator\nsecret body"}
        }),
    );
    let identity = Identity { client: Client::Claude, session_id: "planner".into() };
    assert!(coordinator.store().unwrap().work(&identity).unwrap().is_none());
    assert!(coordinator.store().unwrap().session(&identity).unwrap().is_none());
}

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use serde_json::json;
use tempfile::TempDir;

use super::*;
use crate::{
    coordinator::{Clock, InventoryObservation, ProviderInventory},
    domain::{InventoryResult, ProcessFingerprint, ProcessLiveness, ProcessProbe, ProviderReport},
    state::{FindingAdd, SessionUpdate, Store},
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

#[derive(Default)]
struct RecordingScheduler(Mutex<Vec<(PathBuf, Identity)>>);

impl LifecycleTriageScheduler for RecordingScheduler {
    fn schedule(&self, _: &Coordinator, cwd: &Path, identity: &Identity) {
        self.0.lock().unwrap().push((cwd.to_owned(), identity.clone()));
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

fn begin_turn(runtime: &HookRuntime<'_>, client: &str, repo: &Path, session_id: &str) {
    let mut payload = json!({
        "session_id": session_id,
        "cwd": repo,
        "hook_event_name": "UserPromptSubmit",
        "prompt": "work"
    });
    if client == "codex" {
        payload["turn_id"] = json!("provider-turn");
    }
    runtime.ingest(client, &payload);
}

fn record_finding(coordinator: &Coordinator, repo: &Path, identity: &Identity, summary: &str) -> String {
    coordinator
        .store()
        .unwrap()
        .add_finding(&FindingAdd {
            repo_root: fs::canonicalize(repo).unwrap().to_string_lossy().into_owned(),
            summary: summary.into(),
            normalized_summary: summary.into(),
            kind: None,
            paths: vec![],
            head_oid: None,
            observations: vec![],
            author: identity.clone(),
            turn_id: None,
            current: 100.0,
        })
        .unwrap()
        .finding
        .id
}

#[test]
fn lifecycle_schedules_only_after_an_allowed_main_stop_or_session_end() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let scheduler = RecordingScheduler::default();
    let runtime = HookRuntime::with_scheduler(&coordinator, &scheduler);
    let identity = Identity { client: Client::Codex, session_id: "main".into() };
    let canonical_repo = fs::canonicalize(&repo).unwrap();

    begin_turn(&runtime, "codex", &repo, "main");
    let finding_id = record_finding(&coordinator, &repo, &identity, "must report first");
    let reporting = runtime.ingest(
        "codex",
        &json!({"session_id":"main", "cwd":repo, "hook_event_name":"Stop", "last_assistant_message":""}),
    );
    assert_eq!(serde_json::from_str::<Value>(&reporting).unwrap()["decision"], "block");
    assert!(scheduler.0.lock().unwrap().is_empty(), "reporting continuation must precede triage scheduling");

    assert_eq!(
        runtime.ingest(
            "codex",
            &json!({
                "session_id":"main", "cwd":repo, "hook_event_name":"Stop", "stop_hook_active":true,
                "last_assistant_message": format!("Findings recorded: {finding_id}")
            }),
        ),
        "{}"
    );
    assert_eq!(scheduler.0.lock().unwrap().as_slice(), &[(canonical_repo.clone(), identity.clone())]);

    assert_eq!(
        runtime.ingest(
            "codex",
            &json!({"session_id":"main", "cwd":repo, "hook_event_name":"SubagentStop", "agent_id":"child"}),
        ),
        "{}"
    );
    assert_eq!(scheduler.0.lock().unwrap().len(), 1, "subagent stops never schedule a repository triage");

    runtime.ingest("codex", &json!({"session_id":"ending", "cwd":repo, "hook_event_name":"SessionStart"}));
    runtime.ingest("codex", &json!({"session_id":"ending", "cwd":repo, "hook_event_name":"SessionEnd"}));
    assert_eq!(
        scheduler.0.lock().unwrap().last(),
        Some(&(canonical_repo, Identity { client: Client::Codex, session_id: "ending".into() }))
    );
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
fn finding_id_matching_requires_token_boundaries() {
    assert!(contains_exact_id("Findings recorded: `deadbeef`", "deadbeef"));
    assert!(!contains_exact_id("not-deadbeef0", "deadbeef"));
    assert!(!contains_exact_id("0deadbeef", "deadbeef"));
}

#[test]
fn finding_continuation_keeps_all_ids_within_the_reason_bound() {
    let findings = (0..100)
        .map(|index| crate::state::CurrentTurnFinding {
            id: format!("{index:08x}"),
            summary: "long summary ".repeat(100),
        })
        .collect::<Vec<_>>();
    let output: Value = serde_json::from_str(&finding_continuation("Stop", &findings)).unwrap();
    let reason = output["reason"].as_str().unwrap();
    assert!(reason.chars().count() <= MAX_FINDING_REASON_CHARS);
    for finding in findings {
        assert!(contains_exact_id(reason, &finding.id));
    }
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
fn session_start_assigns_unique_normalized_callsigns() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    let first = Identity { client: Client::Codex, session_id: "first".into() };
    let callsign = generated_callsign(&first, 0);
    let second = (0..4_096)
        .map(|index| Identity { client: Client::Codex, session_id: format!("collision-{index}") })
        .find(|identity| generated_callsign(identity, 0) == callsign)
        .expect("wordlist combinations collide for a bounded set of session IDs");

    let canonical_repo = fs::canonicalize(&repo).unwrap().to_string_lossy().into_owned();
    let mut store = coordinator.store().unwrap();
    store
        .upsert_session(&SessionUpdate {
            identity: first.clone(),
            cwd: canonical_repo.clone(),
            repo_root: Some(canonical_repo),
            state: SessionState::Idle,
            source: "test".into(),
            name: None,
            waiting_for: None,
            permission_mode: None,
            update_permission_mode: false,
            fingerprint: None,
            started_at: None,
            current: 100.0,
        })
        .unwrap();
    store.set_session_callsign(&first, &callsign).unwrap();
    drop(store);
    runtime.ingest("codex", &json!({"session_id":second.session_id, "cwd":repo, "hook_event_name":"SessionStart"}));

    let store = coordinator.store().unwrap();
    let first_callsign = store.session(&first).unwrap().unwrap().callsign.unwrap();
    let second_callsign = store.session(&second).unwrap().unwrap().callsign.unwrap();
    assert_eq!(first_callsign, normalize_callsign(&first_callsign).unwrap());
    assert_eq!(second_callsign, normalize_callsign(&second_callsign).unwrap());
    assert_ne!(first_callsign, second_callsign);
    assert_eq!(second_callsign, generated_callsign(&second, 1));
}

#[test]
fn prompt_presence_adds_compact_finding_counts_without_backlog_content() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    let identity = Identity { client: Client::Claude, session_id: "self".into() };
    begin_turn(&runtime, "claude", &repo, "self");
    let id = record_finding(&coordinator, &repo, &identity, "private finding summary");

    let output = runtime.ingest(
        "claude",
        &json!({"session_id":"self", "cwd":repo, "hook_event_name":"UserPromptSubmit", "prompt":"next"}),
    );
    assert!(output.contains("Findings: pending=1; triaging=0; handed-off=0"), "{output:?}");
    assert!(!output.contains(&id));
    assert!(!output.contains("private finding summary"));
}

#[test]
fn stop_without_current_turn_findings_is_a_normal_noop_for_both_clients() {
    for client in ["codex", "claude"] {
        let temp = TempDir::new().unwrap();
        let (coordinator, repo) = runtime(&temp);
        let runtime = HookRuntime::new(&coordinator);
        begin_turn(&runtime, client, &repo, "self");
        let output = runtime.ingest(
            client,
            &json!({
                "session_id":"self", "cwd":repo, "hook_event_name":"Stop",
                "stop_hook_active":false, "last_assistant_message":"done"
            }),
        );
        assert_eq!(output, if client == "codex" { "{}" } else { "" });
    }
}

#[test]
fn main_stop_blocks_once_with_exact_ids_and_summaries_when_missing() {
    for (client, client_kind) in [("codex", Client::Codex), ("claude", Client::Claude)] {
        let temp = TempDir::new().unwrap();
        let (coordinator, repo) = runtime(&temp);
        let runtime = HookRuntime::new(&coordinator);
        let identity = Identity { client: client_kind, session_id: "self".into() };
        begin_turn(&runtime, client, &repo, "self");
        let id = record_finding(&coordinator, &repo, &identity, "review the boundary");
        let output = runtime.ingest(
            client,
            &json!({
                "session_id":"self", "cwd":repo, "hook_event_name":"Stop",
                "stop_hook_active":false, "last_assistant_message":"done"
            }),
        );
        let output: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(output["decision"], "block");
        assert!(output["reason"].as_str().unwrap().contains("Findings recorded"));
        assert!(output["reason"].as_str().unwrap().contains(&id));
        assert!(output["reason"].as_str().unwrap().contains("review the boundary"));
    }
}

#[test]
fn main_stop_marks_ids_user_surfaced_when_the_final_message_contains_them() {
    for (client, client_kind) in [("codex", Client::Codex), ("claude", Client::Claude)] {
        let temp = TempDir::new().unwrap();
        let (coordinator, repo) = runtime(&temp);
        let runtime = HookRuntime::new(&coordinator);
        let identity = Identity { client: client_kind, session_id: "self".into() };
        begin_turn(&runtime, client, &repo, "self");
        let id = record_finding(&coordinator, &repo, &identity, "review the boundary");
        let output = runtime.ingest(
            client,
            &json!({
                "session_id":"self", "cwd":repo, "hook_event_name":"Stop",
                "stop_hook_active":false, "last_assistant_message":format!("Findings recorded: {id}")
            }),
        );
        assert_eq!(output, if client == "codex" { "{}" } else { "" });
        assert!(coordinator.store().unwrap().current_turn_findings(&identity).unwrap().is_empty());
    }
}

#[test]
fn active_stop_guard_fails_open_without_marking_missing_findings_surfaced() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    let identity = Identity { client: Client::Codex, session_id: "self".into() };
    begin_turn(&runtime, "codex", &repo, "self");
    record_finding(&coordinator, &repo, &identity, "still missing");
    let output = runtime.ingest(
        "codex",
        &json!({
            "session_id":"self", "cwd":repo, "hook_event_name":"Stop",
            "stop_hook_active":true, "last_assistant_message":"done"
        }),
    );
    assert_eq!(output, "{}");
    assert_eq!(coordinator.store().unwrap().current_turn_findings(&identity).unwrap().len(), 1);
}

#[test]
fn duplicate_sightings_require_one_exact_id_and_surface_together() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    let identity = Identity { client: Client::Codex, session_id: "self".into() };
    begin_turn(&runtime, "codex", &repo, "self");
    let id = record_finding(&coordinator, &repo, &identity, "same report");
    assert_eq!(record_finding(&coordinator, &repo, &identity, "same report"), id);
    let blocked = runtime.ingest(
        "codex",
        &json!({
            "session_id":"self", "cwd":repo, "hook_event_name":"Stop",
            "stop_hook_active":false, "last_assistant_message":"done"
        }),
    );
    assert_eq!(blocked.matches(&id).count(), 1);
    assert_eq!(
        runtime.ingest(
            "codex",
            &json!({
                "session_id":"self", "cwd":repo, "hook_event_name":"Stop",
                "stop_hook_active":false, "last_assistant_message":format!("Findings recorded: {id}")
            })
        ),
        "{}"
    );
    assert!(coordinator.store().unwrap().current_turn_findings(&identity).unwrap().is_empty());
}

#[test]
fn subagent_stop_requires_ids_but_only_main_stop_marks_them_user_surfaced() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    let identity = Identity { client: Client::Claude, session_id: "self".into() };
    begin_turn(&runtime, "claude", &repo, "self");
    let id = record_finding(&coordinator, &repo, &identity, "subagent finding");
    let blocked = runtime.ingest(
        "claude",
        &json!({
            "session_id":"self", "cwd":repo, "hook_event_name":"SubagentStop", "agent_id":"child",
            "stop_hook_active":false, "last_assistant_message":"done"
        }),
    );
    let blocked: Value = serde_json::from_str(&blocked).unwrap();
    assert_eq!(blocked["decision"], "block");
    assert!(blocked["reason"].as_str().unwrap().contains("subagent final result"));
    assert!(blocked["reason"].as_str().unwrap().contains(&id));

    assert_eq!(
        runtime.ingest(
            "claude",
            &json!({
                "session_id":"self", "cwd":repo, "hook_event_name":"SubagentStop", "agent_id":"child",
                "stop_hook_active":false, "last_assistant_message":format!("Findings recorded: {id}")
            })
        ),
        ""
    );
    assert_eq!(coordinator.store().unwrap().current_turn_findings(&identity).unwrap().len(), 1);

    let main = runtime.ingest(
        "claude",
        &json!({
            "session_id":"self", "cwd":repo, "hook_event_name":"Stop",
            "stop_hook_active":false, "last_assistant_message":"done"
        }),
    );
    assert_eq!(serde_json::from_str::<Value>(&main).unwrap()["decision"], "block");
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

#[test]
fn post_tool_payloads_record_normalized_deduplicated_touched_paths() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    runtime.ingest("claude", &json!({"session_id":"self", "cwd":repo, "hook_event_name":"SessionStart"}));
    runtime.ingest(
        "claude",
        &json!({
            "session_id":"self", "cwd":repo, "hook_event_name":"PostToolBatch",
            "tool_uses":[
                {"tool_name":"Write", "tool_input":{"file_path":repo.join("src/lib.rs")}},
                {"tool_name":"Edit", "tool_input":{"file_path":repo.join("src/lib.rs")}},
                {"tool_name":"NotebookEdit", "tool_input":{"notebook_path":repo.join("notes.ipynb")}},
                {"tool_name":"Write", "tool_input":{"file_path":temp.path().join("outside")}}
            ]
        }),
    );
    runtime.ingest(
        "codex",
        &json!({
            "session_id":"codex-self", "cwd":repo, "hook_event_name":"SessionStart"
        }),
    );
    runtime.ingest(
        "codex",
        &json!({
            "session_id":"codex-self", "cwd":repo, "hook_event_name":"PostToolUse",
            "tool_name":"apply_patch", "tool_input":{"command":"*** Begin Patch\n*** Update File: README.md\n*** End Patch"}
        }),
    );

    let root = fs::canonicalize(&repo).unwrap().to_string_lossy().into_owned();
    assert_eq!(
        coordinator
            .store()
            .unwrap()
            .touched(&Identity { client: Client::Claude, session_id: "self".into() }, &root)
            .unwrap()
            .paths,
        vec!["notes.ipynb", "src/lib.rs"]
    );
    assert_eq!(
        coordinator
            .store()
            .unwrap()
            .touched(&Identity { client: Client::Codex, session_id: "codex-self".into() }, &root)
            .unwrap()
            .paths,
        vec!["README.md"]
    );
}

#[test]
fn touched_cap_drops_oldest_and_discloses_truncation() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    runtime.ingest("codex", &json!({"session_id":"self", "cwd":repo, "hook_event_name":"SessionStart"}));
    let identity = Identity { client: Client::Codex, session_id: "self".into() };
    let root = fs::canonicalize(repo).unwrap().to_string_lossy().into_owned();
    let paths = (0..=1_000).map(|index| format!("path-{index:04}")).collect::<Vec<_>>();
    coordinator.store().unwrap().record_touched(&identity, &root, &paths, 100.0).unwrap();
    let touched = coordinator.store().unwrap().touched(&identity, &root).unwrap();
    assert!(touched.truncated);
    assert_eq!(touched.paths.len(), 1_000);
    assert!(!touched.paths.contains(&"path-0000".to_owned()));
}

#[test]
fn clean_scope_release_nudge_emits_once_per_transition() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    fs::write(repo.join("tracked.txt"), "clean\n").unwrap();
    for arguments in [
        vec!["config", "user.email", "smoke@example.invalid"],
        vec!["config", "user.name", "Smoke"],
        vec!["add", "tracked.txt"],
        vec!["commit", "-q", "-m", "init"],
    ] {
        assert!(std::process::Command::new("git").args(arguments).current_dir(&repo).status().unwrap().success());
    }
    let runtime = HookRuntime::new(&coordinator);
    runtime.ingest("codex", &json!({"session_id":"self", "cwd":repo, "hook_event_name":"SessionStart"}));
    let identity = Identity { client: Client::Codex, session_id: "self".into() };
    assert_eq!(
        coordinator.start_for(identity, "work", &[repo.join("tracked.txt")], &[], &repo).unwrap().kind,
        crate::domain::OutcomeKind::Ready
    );
    let payload = json!({
        "session_id":"self", "cwd":repo, "hook_event_name":"PostToolUse", "tool_name":"Read", "tool_input":{}
    });
    let first = runtime.ingest("codex", &payload);
    assert!(first.contains("Owned scopes are clean"), "{first}");
    assert_eq!(runtime.ingest("codex", &payload), "");

    fs::write(repo.join("tracked.txt"), "dirty\n").unwrap();
    assert_eq!(runtime.ingest("codex", &payload), "");
    fs::write(repo.join("tracked.txt"), "clean\n").unwrap();
    assert!(runtime.ingest("codex", &payload).contains("Owned scopes are clean"));
}

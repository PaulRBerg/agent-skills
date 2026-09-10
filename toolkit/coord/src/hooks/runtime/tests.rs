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

fn additional_repo(temp: &TempDir, name: &str) -> PathBuf {
    let repo = temp.path().join(name);
    fs::create_dir(&repo).unwrap();
    assert!(std::process::Command::new("git").args(["init", "-q"]).current_dir(&repo).status().unwrap().success());
    repo
}

fn register(coordinator: &Coordinator, identity: &Identity, repo: &Path, pid: u32) {
    let root = fs::canonicalize(repo).unwrap().to_string_lossy().into_owned();
    coordinator
        .store()
        .unwrap()
        .upsert_session(&SessionUpdate {
            identity: identity.clone(),
            cwd: root.clone(),
            repo_root: Some(root),
            state: SessionState::Idle,
            source: "test".into(),
            name: None,
            waiting_for: None,
            permission_mode: None,
            update_permission_mode: false,
            coordination_waived: None,
            fingerprint: Some(ProcessFingerprint { pid, start_token: Some(format!("test-{pid}")) }),
            transcript_path: None,
            started_at: Some(100.0),
            current: 100.0,
        })
        .unwrap();
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
fn lifecycle_schedules_after_main_stop_without_a_finding_report_or_session_end() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let scheduler = RecordingScheduler::default();
    let runtime = HookRuntime::with_scheduler(&coordinator, &scheduler);
    let identity = Identity { client: Client::Codex, session_id: "main".into() };
    let canonical_repo = fs::canonicalize(&repo).unwrap();

    begin_turn(&runtime, "codex", &repo, "main");
    record_finding(&coordinator, &repo, &identity, "tracked internally");
    assert_eq!(
        runtime.ingest(
            "codex",
            &json!({
                "session_id":"main", "cwd":repo, "hook_event_name":"Stop", "stop_hook_active":false,
                "last_assistant_message":"done"
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

    for event in ["SessionStart", "SessionEnd"] {
        runtime.ingest(
            "codex",
            &json!({"session_id":"ending", "cwd":repo, "hook_event_name":event, "transcript_path":"opaque:ending"}),
        );
    }
    assert_eq!(
        scheduler.0.lock().unwrap().last(),
        Some(&(canonical_repo, Identity { client: Client::Codex, session_id: "ending".into() }))
    );
}

#[test]
fn codex_child_fork_and_delayed_hooks_preserve_parent_work_and_evidence_without_waking_waiters() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    let holder = Identity { client: Client::Codex, session_id: "root".into() };
    let waiter = Identity { client: Client::Claude, session_id: "waiter".into() };
    register(&coordinator, &waiter, &repo, 302);
    let first = json!({
        "session_id":"root", "cwd":repo, "hook_event_name":"SessionStart",
        "transcript_path":"opaque:first"
    });
    runtime.ingest("codex", &first);
    register(&coordinator, &holder, &repo, 301);
    let scope = repo.join("src/lib.rs");
    fs::create_dir(repo.join("src")).unwrap();
    fs::write(&scope, "existing dirt").unwrap();
    let root = fs::canonicalize(&repo).unwrap().to_string_lossy().into_owned();
    coordinator
        .store()
        .unwrap()
        .observe_dirt(&root, &crate::host::git_blob_hashes(&repo, &["src/lib.rs".to_owned()], false), 0.0)
        .unwrap();
    assert_eq!(
        coordinator.start_for(holder.clone(), "holder", std::slice::from_ref(&scope), &[], &repo).unwrap().kind,
        crate::domain::OutcomeKind::Ready
    );
    assert_eq!(
        coordinator.start_for(waiter.clone(), "waiter", std::slice::from_ref(&scope), &[], &repo).unwrap().kind,
        crate::domain::OutcomeKind::Blocked
    );
    begin_turn(&runtime, "codex", &repo, "root");
    let finding = record_finding(&coordinator, &repo, &holder, "parent finding");
    let mut store = coordinator.store().unwrap();
    store.set_session_callsign(&holder, "🦀 Custom Root").unwrap();
    store.set_coordination_waived(&holder, true).unwrap();
    store.record_touched(&holder, &root, &["src/lib.rs".to_owned()], 100.0).unwrap();
    store.update_delegate(&holder, "existing-child", Some("worker"), "active", 100.0).unwrap();
    let session = store.session(&holder).unwrap().unwrap();
    let work = store.work(&holder).unwrap();
    let queued = store.work(&waiter).unwrap();
    let baselines = store.baselines_in_repo(&holder, &root).unwrap();
    assert_eq!(baselines.len(), 1);
    let findings = store.current_turn_findings(&holder).unwrap();
    assert_eq!(findings.len(), 1);
    drop(store);

    let events = [
        first.clone(),
        json!({"hook_event_name":"SubagentStart", "transcript_path":"opaque:child", "agent_id":"child",
            "permission_mode":"plan", "prompt":"child prompt", "turn_id":"child-turn"}),
        json!({"hook_event_name":"PostToolUse", "transcript_path":"opaque:child", "tool_name":"Edit",
            "tool_input":{"file_path":repo.join("src/child.rs"), "new_string":"private"}}),
        json!({"hook_event_name":"SubagentStop", "transcript_path":"opaque:child", "agent_id":"child",
            "agent_transcript_path":"opaque:agent-child", "last_assistant_message":finding}),
        json!({"hook_event_name":"PostToolUse", "transcript_path":"opaque:first", "tool_name":"Read"}),
        json!({"hook_event_name":"SessionStart", "transcript_path":"opaque:fork", "source":"resume"}),
        json!({"hook_event_name":"SessionStart", "transcript_path":"opaque:compact", "source":"compact"}),
        json!({"hook_event_name":"PostToolUse", "transcript_path":"opaque:fork", "tool_name":"Read"}),
        json!({"hook_event_name":"PostToolUse", "transcript_path":null, "tool_name":"Read"}),
        json!({"hook_event_name":"PostToolUse", "tool_name":"Read"}),
        first,
    ];
    for mut payload in events {
        payload["session_id"] = json!("root");
        payload["cwd"] = json!(repo);
        runtime.ingest("codex", &payload);
        let store = coordinator.store().unwrap();
        let observed = store.session(&holder).unwrap().unwrap();
        assert_eq!(observed.transcript_path, session.transcript_path, "{payload}");
        assert_eq!(observed.callsign, session.callsign, "{payload}");
        assert_eq!(observed.started_at, session.started_at, "{payload}");
        assert!(observed.coordination_waived, "{payload}");
        assert_eq!(store.work(&holder).unwrap(), work, "{payload}");
        assert_eq!(store.work(&waiter).unwrap(), queued, "{payload}");
        assert_eq!(store.baselines_in_repo(&holder, &root).unwrap(), baselines, "{payload}");
        assert_eq!(store.current_turn_findings(&holder).unwrap(), findings, "{payload}");
        assert!(store.delegates().unwrap().iter().any(|row| row.agent_id == "existing-child"));
        assert!(store.inbox(&waiter, true).unwrap().is_empty(), "{payload}");
    }
    let store = coordinator.store().unwrap();
    assert_eq!(store.touched(&holder, &root).unwrap().paths, ["src/child.rs", "src/lib.rs"]);
    assert_eq!(store.delegates().unwrap().len(), 1);
}

#[test]
fn codex_transcript_interleavings_preserve_drafts_and_queued_work() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    let holder = Identity { client: Client::Claude, session_id: "holder".into() };
    let draft = Identity { client: Client::Codex, session_id: "draft".into() };
    let queued = Identity { client: Client::Codex, session_id: "queued".into() };
    register(&coordinator, &holder, &repo, 303);
    let scope = [repo.join("src/lib.rs")];
    coordinator.start_for(holder, "holder", &scope, &[], &repo).unwrap();
    for (index, owner) in [&draft, &queued].into_iter().enumerate() {
        runtime.ingest(
            "codex",
            &json!({"session_id":owner.session_id, "cwd":repo,
            "hook_event_name":"SessionStart", "transcript_path":"opaque:root"}),
        );
        register(&coordinator, owner, &repo, 304 + index as u32);
    }
    coordinator.draft_for(draft.clone(), "draft", &scope, &[], &repo).unwrap();
    assert_eq!(
        coordinator.start_for(queued.clone(), "queued", &scope, &[], &repo).unwrap().kind,
        crate::domain::OutcomeKind::Blocked
    );

    for owner in [&draft, &queued] {
        let before = coordinator.store().unwrap().work(owner).unwrap().unwrap();
        for event in ["SubagentStart", "PostToolUse", "SubagentStop", "SessionStart", "PostToolUse"] {
            for transcript in [Some("opaque:branch"), None, Some("opaque:root")] {
                let payload = json!({"session_id":owner.session_id, "cwd":repo, "hook_event_name":event,
                    "transcript_path":transcript, "agent_id":"child", "tool_name":"Read"});
                runtime.ingest("codex", &payload);
                let store = coordinator.store().unwrap();
                assert_eq!(store.work(owner).unwrap().as_ref(), Some(&before), "{payload}");
                assert!(store.inbox(owner, true).unwrap().is_empty());
            }
        }
    }
}

#[test]
fn codex_session_end_ignores_branch_or_missing_transcripts_and_ends_the_owning_root() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    let identity = Identity { client: Client::Codex, session_id: "root".into() };
    runtime.ingest(
        "codex",
        &json!({
            "session_id":"root", "cwd":repo, "hook_event_name":"SessionStart",
            "transcript_path":"opaque:first"
        }),
    );
    register(&coordinator, &identity, &repo, 306);
    runtime.ingest(
        "codex",
        &json!({
            "session_id":"root", "cwd":repo, "hook_event_name":"SessionStart",
            "transcript_path":"opaque:fork"
        }),
    );
    assert_eq!(
        coordinator.start_for(identity.clone(), "fork work", &[repo.join("src/lib.rs")], &[], &repo).unwrap().kind,
        crate::domain::OutcomeKind::Ready
    );

    for transcript_path in [Some("opaque:fork"), Some("opaque:old"), Some(""), None] {
        let mut payload = json!({"session_id":"root", "cwd":repo, "hook_event_name":"SessionEnd"});
        if let Some(transcript_path) = transcript_path {
            payload["transcript_path"] = json!(transcript_path);
        }
        runtime.ingest("codex", &payload);
        let store = coordinator.store().unwrap();
        assert!(store.session(&identity).unwrap().is_some());
        assert!(store.work(&identity).unwrap().is_some());
    }

    runtime.ingest(
        "codex",
        &json!({
            "session_id":"root", "cwd":repo, "hook_event_name":"SessionEnd",
            "transcript_path":"opaque:first"
        }),
    );
    let store = coordinator.store().unwrap();
    assert!(store.session(&identity).unwrap().is_none());
    assert!(store.work(&identity).unwrap().is_none());
}

#[test]
fn codex_unanchored_sessions_survive_later_starts_and_ambiguous_ends_until_explicit_done() {
    for first_event in ["PostToolUse", "SubagentStart", "SessionStart"] {
        let temp = TempDir::new().unwrap();
        let (coordinator, repo) = runtime(&temp);
        let scheduler = RecordingScheduler::default();
        let runtime = HookRuntime::with_scheduler(&coordinator, &scheduler);
        let identity = Identity { client: Client::Codex, session_id: "root".into() };
        runtime.ingest("codex", &json!({"session_id":"root", "cwd":repo, "hook_event_name":first_event,
            "agent_id":"child", "transcript_path": if first_event == "SessionStart" { None } else { Some("opaque:child") }}));
        register(&coordinator, &identity, &repo, 307);
        assert_eq!(
            coordinator.start_for(identity.clone(), "work", &[repo.join("file.rs")], &[], &repo).unwrap().kind,
            crate::domain::OutcomeKind::Ready
        );
        let work = coordinator.store().unwrap().work(&identity).unwrap();
        runtime.ingest(
            "codex",
            &json!({"session_id":"root", "cwd":repo, "hook_event_name":"SessionStart",
            "transcript_path":"opaque:later"}),
        );
        assert_eq!(coordinator.store().unwrap().session(&identity).unwrap().unwrap().transcript_path, None);
        for transcript in [None, Some(""), Some("opaque:child"), Some("opaque:later")] {
            let mut payload = json!({"session_id":"root", "cwd":repo, "hook_event_name":"SessionEnd",
                "transcript_path":transcript});
            runtime.ingest("codex", &payload);
            payload.as_object_mut().unwrap().remove("transcript_path");
            runtime.ingest("codex", &payload);
            assert_eq!(coordinator.store().unwrap().work(&identity).unwrap(), work);
        }
        assert!(scheduler.0.lock().unwrap().is_empty());
        coordinator.done_for(&identity, &repo).unwrap();
        assert!(coordinator.store().unwrap().work(&identity).unwrap().is_none());
    }
}

#[test]
fn malformed_child_lifecycle_cannot_mutate_the_parent() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    let identity = Identity { client: Client::Codex, session_id: "root".into() };
    register(&coordinator, &identity, &repo, 308);
    let before = coordinator.store().unwrap().session(&identity).unwrap();
    for event in ["SubagentStart", "SubagentStop"] {
        for agent_id in [None, Some("")] {
            runtime.ingest(
                "codex",
                &json!({"session_id":"root", "cwd":repo, "hook_event_name":event,
                "agent_id":agent_id, "transcript_path":"opaque:child", "permission_mode":"plan"}),
            );
            assert_eq!(coordinator.store().unwrap().session(&identity).unwrap(), before);
        }
    }
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
    assert!(contains_exact_id("Resolved: `deadbeef`", "deadbeef"));
    assert!(!contains_exact_id("not-deadbeef0", "deadbeef"));
    assert!(!contains_exact_id("0deadbeef", "deadbeef"));
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
fn noc_matching_is_case_sensitive_and_requires_an_exact_trimmed_line() {
    for prompt in ["#noc", "  #noc\t", "before\r\n\t#noc  \r\nafter", "```\n#noc\n```"] {
        assert!(prompt_waives_coordination(prompt), "{prompt:?}");
    }
    for prompt in ["`#noc`", "#nocache", "#NOC", "before #noc", "#noc after", ""] {
        assert!(!prompt_waives_coordination(prompt), "{prompt:?}");
    }
}

#[test]
fn noc_waives_both_hosts_until_the_next_valid_untagged_prompt() {
    for (client, client_kind) in [("codex", Client::Codex), ("claude", Client::Claude)] {
        let temp = TempDir::new().unwrap();
        let (coordinator, repo) = runtime(&temp);
        let runtime = HookRuntime::new(&coordinator);
        let identity = Identity { client: client_kind, session_id: "self".into() };

        let tagged = runtime.ingest(
            client,
            &json!({
                "session_id":"self", "cwd":repo, "hook_event_name":"UserPromptSubmit",
                "prompt":"context\r\n  #noc \r\n"
            }),
        );
        assert_eq!(tagged, WAIVED_CONTEXT);
        assert!(coordinator.store().unwrap().session(&identity).unwrap().unwrap().coordination_waived);

        let repeated = runtime.ingest(
            client,
            &json!({"session_id":"self", "cwd":repo, "hook_event_name":"UserPromptSubmit", "prompt":"#noc"}),
        );
        assert_eq!(repeated, WAIVED_CONTEXT);

        let untagged = runtime.ingest(
            client,
            &json!({"session_id":"self", "cwd":repo, "hook_event_name":"UserPromptSubmit", "prompt":"continue"}),
        );
        assert_eq!(untagged, WAIVER_ENDED_CONTEXT);
        assert!(!coordinator.store().unwrap().session(&identity).unwrap().unwrap().coordination_waived);
    }
}

#[test]
fn malformed_prompt_preserves_an_active_waiver_without_leaking_payload() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    let identity = Identity { client: Client::Codex, session_id: "self".into() };
    runtime.ingest(
        "codex",
        &json!({"session_id":"self", "cwd":repo, "hook_event_name":"UserPromptSubmit", "prompt":"#noc"}),
    );

    let output = runtime.ingest(
        "codex",
        &json!({
            "session_id":"self", "cwd":repo, "hook_event_name":"UserPromptSubmit",
            "prompt":{"secret":"PRIVATE"}
        }),
    );
    assert_eq!(output, WAIVED_CONTEXT);
    assert!(!output.contains("PRIVATE"));
    assert!(coordinator.store().unwrap().session(&identity).unwrap().unwrap().coordination_waived);
}

#[test]
fn noc_context_appends_only_complete_compact_counts_and_suppresses_the_gate_reminder() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    runtime.ingest("claude", &json!({"session_id":"peer", "cwd":repo, "hook_event_name":"SessionStart"}));
    fs::write(repo.join("unattributed"), "dirty").unwrap();

    let output = runtime.ingest(
        "codex",
        &json!({"session_id":"self", "cwd":repo, "hook_event_name":"UserPromptSubmit", "prompt":"#noc"}),
    );
    assert_eq!(output, format!("{WAIVED_CONTEXT} Peers: 1; queued: 0; unread: 0."));
    assert!(output.chars().count() <= MAX_PRESENCE_CHARS);
    assert!(!output.contains("Acquire scopes"));

    let identity = Identity { client: Client::Codex, session_id: "self".into() };
    record_finding(&coordinator, &repo, &identity, "private finding");
    let repeated = runtime.ingest(
        "codex",
        &json!({"session_id":"self", "cwd":repo, "hook_event_name":"UserPromptSubmit", "prompt":"#noc"}),
    );
    assert!(repeated.ends_with("Peers: 1; queued: 0; unread: 0."));
    assert!(!repeated.contains("Findings p/t/h"), "the complete second fragment does not fit: {repeated:?}");
    assert!(repeated.chars().count() <= MAX_PRESENCE_CHARS);
}

#[test]
fn noc_context_includes_compact_finding_counts_when_they_fit() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    let identity = Identity { client: Client::Codex, session_id: "self".into() };
    runtime.ingest(
        "codex",
        &json!({"session_id":"self", "cwd":repo, "hook_event_name":"UserPromptSubmit", "prompt":"#noc"}),
    );
    record_finding(&coordinator, &repo, &identity, "private finding");

    let output = runtime.ingest(
        "codex",
        &json!({"session_id":"self", "cwd":repo, "hook_event_name":"UserPromptSubmit", "prompt":"#noc"}),
    );
    assert_eq!(output, format!("{WAIVED_CONTEXT} Findings p/t/h: 1/0/0."));
    assert!(output.chars().count() <= MAX_PRESENCE_CHARS);
}

#[test]
fn waived_sessions_keep_messages_touched_paths_findings_and_lifecycle_hooks_active() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    let identity = Identity { client: Client::Codex, session_id: "self".into() };
    let peer = Identity { client: Client::Claude, session_id: "peer".into() };
    runtime.ingest(
        "codex",
        &json!({"session_id":"self", "cwd":repo, "hook_event_name":"SessionStart",
        "transcript_path":"opaque:self"}),
    );
    runtime.ingest(
        "codex",
        &json!({"session_id":"self", "cwd":repo, "hook_event_name":"UserPromptSubmit", "prompt":"#noc"}),
    );
    let root = fs::canonicalize(&repo).unwrap().to_string_lossy().into_owned();
    coordinator
        .store()
        .unwrap()
        .send_message(&peer, std::slice::from_ref(&identity), "peer data", Some(&root), 100.0)
        .unwrap();

    let nudge = runtime.ingest(
        "codex",
        &json!({
            "session_id":"self", "cwd":repo, "hook_event_name":"PostToolUse",
            "tool_name":"apply_patch",
            "tool_input":{"command":"*** Begin Patch\n*** Update File: README.md\n*** End Patch"}
        }),
    );
    assert!(nudge.contains("1 unread peer messages"));
    let store = coordinator.store().unwrap();
    assert_eq!(store.touched(&identity, &root).unwrap().paths, ["README.md"]);
    assert!(store.session(&identity).unwrap().unwrap().coordination_waived);
    drop(store);

    let finding_id = record_finding(&coordinator, &repo, &identity, "waived finding");
    let stop = runtime.ingest(
        "codex",
        &json!({"session_id":"self", "cwd":repo, "hook_event_name":"Stop", "last_assistant_message":"done"}),
    );
    assert_eq!(stop, "{}");
    let findings = coordinator.store().unwrap().current_turn_findings(&identity).unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].id, finding_id);
    assert!(coordinator.store().unwrap().session(&identity).unwrap().unwrap().coordination_waived);

    runtime.ingest(
        "codex",
        &json!({"session_id":"self", "cwd":repo, "hook_event_name":"SessionEnd",
        "transcript_path":"opaque:self"}),
    );
    assert!(coordinator.store().unwrap().session(&identity).unwrap().is_none());
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
            coordination_waived: None,
            fingerprint: None,
            transcript_path: None,
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
fn main_stop_allows_omitted_finding_ids_without_marking_them_surfaced() {
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
        assert_eq!(output, if client == "codex" { "{}" } else { "" });
        let findings = coordinator.store().unwrap().current_turn_findings(&identity).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].id, id);
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
                "stop_hook_active":false, "last_assistant_message":format!("Resolved {id}")
            }),
        );
        assert_eq!(output, if client == "codex" { "{}" } else { "" });
        assert!(coordinator.store().unwrap().current_turn_findings(&identity).unwrap().is_empty());
    }
}

#[test]
fn continued_stop_keeps_unreported_findings_internal() {
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
fn voluntarily_reported_duplicate_sightings_surface_together() {
    let temp = TempDir::new().unwrap();
    let (coordinator, repo) = runtime(&temp);
    let runtime = HookRuntime::new(&coordinator);
    let identity = Identity { client: Client::Codex, session_id: "self".into() };
    begin_turn(&runtime, "codex", &repo, "self");
    let id = record_finding(&coordinator, &repo, &identity, "same report");
    assert_eq!(record_finding(&coordinator, &repo, &identity, "same report"), id);
    let output = runtime.ingest(
        "codex",
        &json!({
            "session_id":"self", "cwd":repo, "hook_event_name":"Stop",
            "stop_hook_active":false, "last_assistant_message":"done"
        }),
    );
    assert_eq!(output, "{}");
    assert_eq!(coordinator.store().unwrap().current_turn_findings(&identity).unwrap().len(), 1);
    assert_eq!(
        runtime.ingest(
            "codex",
            &json!({
                "session_id":"self", "cwd":repo, "hook_event_name":"Stop",
                "stop_hook_active":false, "last_assistant_message":format!("Resolved {id}")
            })
        ),
        "{}"
    );
    assert!(coordinator.store().unwrap().current_turn_findings(&identity).unwrap().is_empty());
}

#[test]
fn subagent_stop_allows_omitted_ids_and_never_marks_findings_user_surfaced() {
    for (client, client_kind) in [("codex", Client::Codex), ("claude", Client::Claude)] {
        let temp = TempDir::new().unwrap();
        let (coordinator, repo) = runtime(&temp);
        let runtime = HookRuntime::new(&coordinator);
        let identity = Identity { client: client_kind, session_id: "self".into() };
        begin_turn(&runtime, client, &repo, "self");
        let id = record_finding(&coordinator, &repo, &identity, "subagent finding");
        for message in ["done".to_owned(), format!("Resolved {id}")] {
            let output = runtime.ingest(
                client,
                &json!({
                    "session_id":"self", "cwd":repo, "hook_event_name":"SubagentStop", "agent_id":"child",
                    "stop_hook_active":false, "last_assistant_message":message
                }),
            );
            assert_eq!(output, if client == "codex" { "{}" } else { "" });
            assert_eq!(coordinator.store().unwrap().current_turn_findings(&identity).unwrap().len(), 1);
        }
    }
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
    // CI does not run under a Codex host process, so SessionStart cannot
    // discover a fingerprint there. Register deterministic test evidence so
    // the injected AliveProbe can establish complete provider coverage.
    let root = fs::canonicalize(&repo).unwrap().to_string_lossy().into_owned();
    coordinator
        .store()
        .unwrap()
        .upsert_session(&SessionUpdate {
            identity: identity.clone(),
            cwd: root.clone(),
            repo_root: Some(root),
            state: SessionState::Idle,
            source: "test".into(),
            name: None,
            waiting_for: None,
            permission_mode: None,
            update_permission_mode: false,
            coordination_waived: None,
            fingerprint: Some(ProcessFingerprint { pid: std::process::id(), start_token: Some("test".into()) }),
            transcript_path: None,
            started_at: Some(100.0),
            current: 100.0,
        })
        .unwrap();
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

#[test]
fn release_nudge_uses_only_the_payload_root_claim() {
    let temp = TempDir::new().unwrap();
    let (coordinator, first) = runtime(&temp);
    let second = additional_repo(&temp, "z-repo");
    let identity = Identity { client: Client::Codex, session_id: "self".into() };
    register(&coordinator, &identity, &first, 201);
    for root in [&first, &second] {
        fs::write(root.join("tracked.txt"), "clean\n").unwrap();
        assert!(
            std::process::Command::new("git")
                .args(["add", "tracked.txt"])
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
        assert!(
            std::process::Command::new("git")
                .args(["-c", "user.name=ai-coord test", "-c", "user.email=test@invalid", "commit", "-q", "-m", "init",])
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
    }
    assert_eq!(
        coordinator
            .start_bundle_for(
                identity.clone(),
                "work",
                &[first.join("tracked.txt"), second.join("tracked.txt")],
                &[],
                &first,
            )
            .unwrap()
            .kind,
        crate::domain::OutcomeKind::Ready
    );
    fs::write(first.join("tracked.txt"), "dirty\n").unwrap();

    let output = HookRuntime::new(&coordinator).ingest(
        "codex",
        &json!({
            "session_id":"self", "cwd":second, "hook_event_name":"PostToolUse",
            "tool_name":"Read", "tool_input":{}
        }),
    );
    assert!(output.contains("Owned scopes are clean"), "{output}");
}

#[test]
fn waker_requires_a_payload_root_claim_then_reevaluates_the_whole_bundle() {
    let temp = TempDir::new().unwrap();
    let (coordinator, first) = runtime(&temp);
    let second = additional_repo(&temp, "z-repo");
    let third = additional_repo(&temp, "zz-repo");
    let holder = Identity { client: Client::Codex, session_id: "holder".into() };
    let waiter = Identity { client: Client::Claude, session_id: "waiter".into() };
    register(&coordinator, &holder, &second, 210);
    register(&coordinator, &waiter, &first, 211);
    let scope = [PathBuf::from("src/lib.rs")];
    coordinator.start_for(holder.clone(), "holder", &scope, &[], &second).unwrap();
    assert_eq!(
        coordinator
            .start_bundle_for(
                waiter.clone(),
                "bundle",
                &[first.join("src/lib.rs"), second.join("src/lib.rs")],
                &[],
                &first,
            )
            .unwrap()
            .kind,
        crate::domain::OutcomeKind::Blocked
    );
    let runtime = HookRuntime::new(&coordinator);
    assert!(
        runtime
            .waker("claude", &json!({"session_id":"waiter", "cwd":third, "hook_event_name":"PostToolUseFailure"}),)
            .is_none()
    );
    coordinator.done_for(&holder, &second).unwrap();
    coordinator.store().unwrap().acknowledge(&waiter, None, 100.0).unwrap();
    assert_eq!(
        runtime
            .waker("claude", &json!({"session_id":"waiter", "cwd":first, "hook_event_name":"PostToolUseFailure"}),)
            .unwrap()
            .kind,
        crate::domain::OutcomeKind::Ready
    );
}

#[test]
fn session_end_cleans_identity_wide_bundle_work_and_deduplicates_wakeup() {
    let temp = TempDir::new().unwrap();
    let (coordinator, first) = runtime(&temp);
    let second = additional_repo(&temp, "z-repo");
    let holder = Identity { client: Client::Codex, session_id: "holder".into() };
    let waiter = Identity { client: Client::Claude, session_id: "waiter".into() };
    let scheduler = RecordingScheduler::default();
    let runtime = HookRuntime::with_scheduler(&coordinator, &scheduler);
    runtime.ingest(
        "codex",
        &json!({"session_id":"holder", "cwd":first, "hook_event_name":"SessionStart",
        "transcript_path":"opaque:holder"}),
    );
    register(&coordinator, &holder, &first, 220);
    register(&coordinator, &waiter, &first, 221);
    let claims = [first.join("src/lib.rs"), second.join("src/lib.rs")];
    coordinator.start_bundle_for(holder.clone(), "holder", &claims, &[], &first).unwrap();
    assert_eq!(
        coordinator.start_bundle_for(waiter.clone(), "waiter", &claims, &[], &first).unwrap().kind,
        crate::domain::OutcomeKind::Blocked
    );

    let revision = coordinator.store().unwrap().session(&holder).unwrap().unwrap().revision;
    runtime.ingest(
        "codex",
        &json!({"session_id":"holder", "cwd":second, "hook_event_name":"SessionStart",
        "transcript_path":"opaque:fork"}),
    );
    assert!(!coordinator.end_session_generation_for(&holder, revision).unwrap());
    runtime.ingest(
        "codex",
        &json!({"session_id":"holder", "cwd":second, "hook_event_name":"SessionEnd",
        "transcript_path":"opaque:fork"}),
    );
    assert!(coordinator.store().unwrap().work(&holder).unwrap().is_some());
    assert!(coordinator.store().unwrap().inbox(&waiter, true).unwrap().is_empty());
    for _ in 0..2 {
        runtime.ingest(
            "codex",
            &json!({"session_id":"holder", "cwd":second, "hook_event_name":"SessionEnd",
            "transcript_path":"opaque:holder"}),
        );
    }
    assert_eq!(scheduler.0.lock().unwrap().len(), 1);
    let store = coordinator.store().unwrap();
    assert!(store.work(&holder).unwrap().is_none());
    let wakeups = store.inbox(&waiter, true).unwrap();
    assert_eq!(wakeups.len(), 1);
    assert!(
        wakeups[0]
            .repo_root
            .as_deref()
            .is_some_and(|root| root == fs::canonicalize(&first).unwrap().to_str().unwrap() ||
                root == fs::canonicalize(&second).unwrap().to_str().unwrap())
    );
    for root in [&first, &second] {
        assert!(store.residual_owners(fs::canonicalize(root).unwrap().to_str().unwrap()).unwrap().is_empty());
    }
}

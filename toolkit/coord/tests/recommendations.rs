#![cfg(unix)]

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use assert_cmd::assert::OutputAssertExt;
use predicates::prelude::*;
use serde_json::{Value, json};
use tempfile::TempDir;

const BINARY: &str = env!("CARGO_BIN_EXE_ai-coord");

struct Fixture {
    temporary: TempDir,
    root: PathBuf,
    state: PathBuf,
    codex_home: PathBuf,
    claude_home: PathBuf,
    home: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let root = temporary.path().join("repo");
        let state = temporary.path().join("state");
        let codex_home = temporary.path().join("codex");
        let claude_home = temporary.path().join("claude");
        let home = temporary.path().join("home");
        fs::create_dir_all(root.join("src/generated")).expect("repository directories");
        fs::write(root.join("src/legacy.rs"), "pub fn legacy() {}\n").expect("legacy fixture");
        fs::write(root.join("src/independent.rs"), "pub fn independent() {}\n").expect("independent fixture");
        fs::write(root.join("src/generated/one.rs"), "pub fn generated() {}\n").expect("generated fixture");

        let init = Command::new("/usr/bin/git").args(["init", "--quiet"]).current_dir(&root).output().unwrap();
        assert!(init.status.success(), "git init: {}", String::from_utf8_lossy(&init.stderr));
        let add = Command::new("/usr/bin/git").args(["add", "."]).current_dir(&root).output().unwrap();
        assert!(add.status.success(), "git add: {}", String::from_utf8_lossy(&add.stderr));
        let commit = Command::new("/usr/bin/git")
            .args([
                "-c",
                "user.name=ai-coord recommendation test",
                "-c",
                "user.email=recommendation-test@invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ])
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(commit.status.success(), "git commit: {}", String::from_utf8_lossy(&commit.stderr));

        Self { temporary, root, state, codex_home, claude_home, home }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(BINARY);
        self.configure(&mut command);
        command
    }

    fn bash_command(&self) -> Command {
        let mut command = Command::new("/bin/bash");
        self.configure(&mut command);
        command
    }

    fn configure(&self, command: &mut Command) {
        command
            .current_dir(&self.root)
            .env("AI_COORD_STATE_DIR", &self.state)
            .env("AI_COORD_CLIENT", "codex")
            .env("CODEX_HOME", &self.codex_home)
            .env("CLAUDE_CONFIG_DIR", &self.claude_home)
            .env("HOME", &self.home)
            .env("PATH", "/usr/bin:/bin")
            .env_remove("CODEX_SESSION_ID")
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CLAUDE_CODE_SESSION_ID");
    }

    fn output_as(&self, session_id: &str, arguments: &[&str]) -> Output {
        self.output_as_in(session_id, &self.root, arguments)
    }

    fn output_as_in(&self, session_id: &str, cwd: &Path, arguments: &[&str]) -> Output {
        self.command()
            .current_dir(cwd)
            .env("AI_COORD_SESSION_ID", session_id)
            .args(arguments)
            .output()
            .expect("run ai-coord")
    }

    fn output_owned(&self, session_id: &str, arguments: &[String]) -> Output {
        self.command().env("AI_COORD_SESSION_ID", session_id).args(arguments).output().expect("run ai-coord")
    }

    fn status(&self) -> Value {
        let output = self.output_as("observer", &["status", "--all", "--json"]);
        assert!(matches!(output.status.code(), Some(0 | 2)), "stderr={}", String::from_utf8_lossy(&output.stderr));
        serde_json::from_slice(&output.stdout).expect("status JSON")
    }

    fn work(&self, session_id: &str) -> Value {
        self.status()["work"]
            .as_array()
            .unwrap()
            .iter()
            .find(|work| work["session_id"] == session_id)
            .unwrap_or_else(|| panic!("missing work for {session_id}"))
            .clone()
    }

    fn baseline(&self, session_id: &str) -> Vec<u8> {
        let output = self.output_as(session_id, &["baseline"]);
        output.clone().assert().success();
        output.stdout
    }

    fn outside(&self) -> &Path {
        self.temporary.path()
    }
}

#[derive(Default)]
struct Hosts(Vec<SyntheticHost>);

struct SyntheticHost {
    session_id: String,
    child: Child,
    request: PathBuf,
    response: PathBuf,
}

impl Hosts {
    fn spawn(&mut self, fixture: &Fixture, session_id: &str) {
        self.0.push(spawn_synthetic_host(fixture, session_id));
    }

    fn post_tool(&self, fixture: &Fixture, session_id: &str) -> String {
        let host = self.0.iter().find(|host| host.session_id == session_id).expect("synthetic host");
        let _ = fs::remove_file(&host.response);
        let payload = json!({
            "session_id": session_id,
            "cwd": fixture.root,
            "hook_event_name": "PostToolUse",
            "tool_name": "Read",
            "tool_input": {},
        });
        fs::write(&host.request, payload.to_string()).expect("write synthetic hook request");
        let deadline = Instant::now() + Duration::from_secs(5);
        while !host.response.is_file() {
            assert!(Instant::now() < deadline, "synthetic host {session_id} did not return hook output");
            thread::sleep(Duration::from_millis(10));
        }
        fs::read_to_string(&host.response).expect("read synthetic hook response")
    }
}

impl Drop for Hosts {
    fn drop(&mut self) {
        for host in &mut self.0 {
            let _ = host.child.kill();
            let _ = host.child.wait();
        }
    }
}

#[test]
fn accepted_recommendation_requires_explicit_narrowing_and_a_fresh_sender_start() {
    let fixture = Fixture::new();
    let mut hosts = Hosts::default();
    for session in ["holder", "replacement"] {
        hosts.spawn(&fixture, session);
        assert_strong_session(&fixture, session);
    }
    fixture.output_as("holder", &["name", "🦀 Historical Holder"]).assert().success();
    fixture.output_as("replacement", &["name", "🐙 Replacement Agent"]).assert().success();

    let holder_start = fixture.output_as(
        "holder",
        &[
            "start",
            "--recursive",
            "src/generated",
            "polish legacy and independent code",
            "src/legacy.rs",
            "src/independent.rs",
        ],
    );
    holder_start.assert().success();
    let replacement_start =
        fixture.output_as("replacement", &["start", "replace legacy implementation", "src/legacy.rs"]);
    replacement_start.clone().assert().failure().code(3).stdout(predicate::str::starts_with("BLOCKED\t"));

    let holder_before = fixture.work("holder");
    let replacement_before = fixture.work("replacement");
    let baseline_before = fixture.baseline("holder");
    let legacy_before = fs::read(fixture.root.join("src/legacy.rs")).unwrap();
    let independent_before = fs::read(fixture.root.join("src/independent.rs")).unwrap();
    let reason = format!("{}\n\tkeeps all required verification\u{1b}", "界".repeat(300));
    let replacement = "Remove the historical adapter, update its callers, and verify retained behavior.";
    let sent = fixture.output_as(
        "replacement",
        &[
            "recommend",
            "send",
            "holder",
            "--action",
            "defer",
            "--path",
            "src/legacy.rs",
            "--recursive",
            "src/generated",
            "--reason",
            &reason,
            "--replacement",
            replacement,
        ],
    );
    sent.clone().assert().success();
    let id = tsv_id(&sent.stdout, "RECOMMENDED");

    assert_eq!(fixture.work("holder"), holder_before);
    assert_eq!(fixture.work("replacement"), replacement_before);
    assert_eq!(fixture.baseline("holder"), baseline_before);
    assert_eq!(fs::read(fixture.root.join("src/legacy.rs")).unwrap(), legacy_before);
    assert_eq!(fs::read(fixture.root.join("src/independent.rs")).unwrap(), independent_before);

    let checkpoint = hosts.post_tool(&fixture, "holder");
    let hook_json: Value = serde_json::from_str(&checkpoint).expect("hook JSON");
    let context = hook_json["hookSpecificOutput"]["additionalContext"].as_str().expect("hook context");
    assert!(context.contains("Work recommendations need review"), "{context}");
    assert!(context.chars().count() <= 200, "{context}");
    assert!(!context.contains(&id), "{context}");
    assert!(!context.contains("界"), "{context}");

    let shown = fixture.output_as_in("holder", fixture.outside(), &["recommend", "show", &id, "--json"]);
    shown.clone().assert().success();
    let shown: Value = serde_json::from_slice(&shown.stdout).expect("recommendation JSON");
    assert_eq!(shown["schema_version"], 1);
    let recommendation = &shown["recommendation"];
    assert_eq!(recommendation["reason"], format!("{} keeps all required verification", "界".repeat(300)));
    assert!(recommendation["reason"].as_str().unwrap().chars().count() > 240);
    assert_eq!(recommendation["replacement"], replacement);
    assert_eq!(recommendation["sender"]["callsign"], "🐙 Replacement Agent");
    assert_eq!(recommendation["recipient"]["callsign"], "🦀 Historical Holder");
    assert_eq!(recommendation["scopes"].as_array().unwrap().len(), 2);

    let incoming = fixture.output_as("holder", &["recommend", "list", "--json"]);
    incoming.clone().assert().success();
    let incoming: Value = serde_json::from_slice(&incoming.stdout).unwrap();
    assert_eq!(incoming["schema_version"], 1);
    assert_eq!(incoming["recommendations"].as_array().unwrap().len(), 1);
    let outgoing = fixture.output_as("replacement", &["recommend", "list", "--sent", "--json"]);
    outgoing.clone().assert().success();
    let outgoing: Value = serde_json::from_slice(&outgoing.stdout).unwrap();
    assert_eq!(outgoing["recommendations"][0]["id"], id);

    let accepted = fixture.output_as_in(
        "holder",
        fixture.outside(),
        &[
            "recommend",
            "respond",
            &id,
            "--decision",
            "accepted",
            "--reason",
            "Defer legacy polish, retain independent validation, and inspect the replacement.",
        ],
    );
    accepted.clone().assert().success();
    assert_eq!(String::from_utf8_lossy(&accepted.stdout), format!("ACCEPTED\t{id}\n"));

    assert_eq!(fixture.work("holder"), holder_before);
    assert_eq!(fixture.work("replacement"), replacement_before);
    assert_eq!(fixture.baseline("holder"), baseline_before);
    assert_eq!(fs::read(fixture.root.join("src/legacy.rs")).unwrap(), legacy_before);
    assert_eq!(fs::read(fixture.root.join("src/independent.rs")).unwrap(), independent_before);
    let pending = fixture.output_as("holder", &["recommend", "list", "--json"]);
    assert_eq!(serde_json::from_slice::<Value>(&pending.stdout).unwrap()["recommendations"], json!([]));

    let narrowed = fixture.output_as("holder", &["start", "retain independent validation", "src/independent.rs"]);
    narrowed.clone().assert().success();
    assert_eq!(String::from_utf8_lossy(&narrowed.stdout), "READY\tsrc/independent.rs\n");
    assert_eq!(fixture.work("replacement")["state"], "queued");
    let accepted_history = fixture.output_as("holder", &["recommend", "show", &id, "--json"]);
    assert_eq!(
        serde_json::from_slice::<Value>(&accepted_history.stdout).unwrap()["recommendation"]["state"],
        "accepted"
    );

    let foreground = fixture.output_as("replacement", &["start", "replace legacy implementation", "src/legacy.rs"]);
    foreground.clone().assert().success();
    assert_eq!(String::from_utf8_lossy(&foreground.stdout), "READY\tsrc/legacy.rs\n");
    let withdrawn = fixture.output_as_in(
        "replacement",
        fixture.outside(),
        &["recommend", "withdraw", &id, "--reason", "Replacement is complete; inspect its result."],
    );
    withdrawn.clone().assert().success();
    assert_eq!(String::from_utf8_lossy(&withdrawn.stdout), format!("WITHDRAWN\t{id}\n"));
}

#[test]
fn dirty_partial_edits_survive_rejection_and_acceptance_without_transferring_ownership() {
    let fixture = Fixture::new();
    let mut hosts = Hosts::default();
    for session in ["holder", "replacement"] {
        hosts.spawn(&fixture, session);
        assert_strong_session(&fixture, session);
    }
    fixture
        .output_as("holder", &["start", "preserve and polish legacy", "src/legacy.rs", "src/independent.rs"])
        .assert()
        .success();
    fixture.output_as("replacement", &["start", "remove legacy", "src/legacy.rs"]).assert().failure().code(3);
    fs::write(fixture.root.join("src/legacy.rs"), "pub fn legacy() { /* owned partial edit */ }\n").unwrap();

    let holder_before = fixture.work("holder");
    let replacement_before = fixture.work("replacement");
    let baseline_before = fixture.baseline("holder");
    let file_before = fs::read(fixture.root.join("src/legacy.rs")).unwrap();
    let status_before = git_status(&fixture.root);
    let sent = fixture.output_as(
        "replacement",
        &[
            "recommend",
            "send",
            "holder",
            "--action",
            "omit",
            "--path",
            "src/legacy.rs",
            "--reason",
            "The planned deletion could make this partial polish redundant.",
            "--replacement",
            "Remove the legacy implementation after the holder handles its owned partial edit.",
        ],
    );
    sent.clone().assert().success();
    let id = tsv_id(&sent.stdout, "RECOMMENDED");
    let rejected = fixture.output_as_in(
        "holder",
        fixture.outside(),
        &[
            "recommend",
            "respond",
            &id,
            "--decision",
            "rejected",
            "--reason",
            "The user explicitly requires preserving this implementation; resolve that authority conflict first.",
        ],
    );
    rejected.clone().assert().success();
    assert_eq!(String::from_utf8_lossy(&rejected.stdout), format!("REJECTED\t{id}\n"));

    assert_eq!(fixture.work("holder"), holder_before);
    assert_eq!(fixture.work("replacement"), replacement_before);
    assert_eq!(fixture.baseline("holder"), baseline_before);
    assert_eq!(fs::read(fixture.root.join("src/legacy.rs")).unwrap(), file_before);
    assert_eq!(git_status(&fixture.root), status_before);
    let history = fixture.output_as("holder", &["recommend", "list", "--all", "--json"]);
    let history: Value = serde_json::from_slice(&history.stdout).unwrap();
    assert_eq!(history["recommendations"][0]["state"], "rejected");
    assert_eq!(history["recommendations"][0]["decision"], "rejected");

    let sent = fixture.output_as(
        "replacement",
        &[
            "recommend",
            "send",
            "holder",
            "--action",
            "defer",
            "--path",
            "src/legacy.rs",
            "--reason",
            "Defer cosmetic polish while the preservation requirement is resolved.",
            "--replacement",
            "Resolve the requirement before changing the implementation; retain existing partial edits.",
        ],
    );
    sent.clone().assert().success();
    let id = tsv_id(&sent.stdout, "RECOMMENDED");
    fixture
        .output_as(
            "holder",
            &[
                "recommend",
                "respond",
                &id,
                "--decision",
                "accepted",
                "--reason",
                "Deferring only cosmetic polish; retaining the owned partial edit and required validation.",
            ],
        )
        .assert()
        .success()
        .stdout(format!("ACCEPTED\t{id}\n"));
    assert_eq!(fixture.work("holder"), holder_before);
    assert_eq!(fixture.work("replacement"), replacement_before);
    assert_eq!(fixture.baseline("holder"), baseline_before);
    assert_eq!(fs::read(fixture.root.join("src/legacy.rs")).unwrap(), file_before);
    assert_eq!(git_status(&fixture.root), status_before);
    fixture
        .output_as("replacement", &["start", "remove legacy", "src/legacy.rs"])
        .assert()
        .failure()
        .code(3)
        .stdout(predicate::str::starts_with("BLOCKED\t"));
}

#[test]
fn endpoint_privacy_idempotency_conflicts_and_callsign_snapshots_are_cli_contracts() {
    let fixture = Fixture::new();
    let mut hosts = Hosts::default();
    for session in ["sender", "recipient", "outsider"] {
        hosts.spawn(&fixture, session);
        assert_strong_session(&fixture, session);
    }
    fixture.output_as("sender", &["name", "🦉 Sender Original"]).assert().success();
    fixture.output_as("recipient", &["name", "🦊 Recipient Original"]).assert().success();
    fixture.output_as("recipient", &["start", "recipient work", "src/legacy.rs"]).assert().success();
    fixture.output_as("sender", &["start", "sender replacement", "src/legacy.rs"]).assert().failure().code(3);

    let send_args = [
        "recommend",
        "send",
        "recipient",
        "--action",
        "defer",
        "--path",
        "src/legacy.rs",
        "--reason",
        "The replacement changes this input.",
        "--replacement",
        "Replace it and rerun validation.",
    ];
    let sent = fixture.output_as("sender", &send_args);
    sent.clone().assert().success();
    let id = tsv_id(&sent.stdout, "RECOMMENDED");
    let duplicate = fixture.output_as("sender", &send_args);
    duplicate.clone().assert().success();
    assert_eq!(String::from_utf8_lossy(&duplicate.stdout), format!("EXISTING\t{id}\n"));

    fixture.output_as("sender", &["name", "🦉 Sender Renamed"]).assert().success();
    fixture.output_as("recipient", &["name", "🦊 Recipient Renamed"]).assert().success();
    let shown = fixture.output_as_in("recipient", fixture.outside(), &["recommend", "show", &id, "--json"]);
    let shown: Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(shown["recommendation"]["sender"]["callsign"], "🦉 Sender Original");
    assert_eq!(shown["recommendation"]["recipient"]["callsign"], "🦊 Recipient Original");

    fixture
        .output_as_in("outsider", fixture.outside(), &["recommend", "show", &id, "--json"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("not found for this endpoint"));
    fixture
        .output_as("recipient", &["recommend", "withdraw", &id, "--reason", "not mine"])
        .assert()
        .failure()
        .code(64)
        .stderr(predicate::str::contains("only the recommendation sender may withdraw"));
    fixture
        .output_as("sender", &["recommend", "respond", &id, "--decision", "rejected", "--reason", "not mine"])
        .assert()
        .failure()
        .code(64)
        .stderr(predicate::str::contains("only the recommendation recipient may respond"));

    let decision_args =
        ["recommend", "respond", &id, "--decision", "rejected", "--reason", "Preserve the required implementation."];
    fixture.output_as_in("recipient", fixture.outside(), &decision_args).assert().success();
    let repeated = fixture.output_as_in("recipient", fixture.outside(), &decision_args);
    repeated.clone().assert().success();
    assert_eq!(String::from_utf8_lossy(&repeated.stdout), format!("REJECTED\t{id}\n"));
    let conflict = fixture.output_as_in(
        "recipient",
        fixture.outside(),
        &["recommend", "respond", &id, "--decision", "accepted", "--reason", "Changed my mind."],
    );
    conflict.clone().assert().failure().code(3);
    assert_eq!(String::from_utf8_lossy(&conflict.stdout), format!("CONFLICT\t{id}\n"));
}

#[test]
fn parser_semantic_limits_target_rules_and_scope_rules_keep_exit_mappings() {
    let fixture = Fixture::new();
    let mut hosts = Hosts::default();
    for session in ["sender", "recipient"] {
        hosts.spawn(&fixture, session);
        assert_strong_session(&fixture, session);
    }
    fixture.output_as("recipient", &["start", "recipient source tree", "--recursive", "src"]).assert().success();
    fixture.output_as("sender", &["start", "sender replacement", "src/legacy.rs"]).assert().failure().code(3);

    fixture
        .output_as("sender", &["recommend", "send", "recipient"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("--action"));
    for target in ["repo", "sender"] {
        fixture
            .output_as(
                "sender",
                &[
                    "recommend",
                    "send",
                    target,
                    "--action",
                    "defer",
                    "--path",
                    "src/legacy.rs",
                    "--reason",
                    "reason",
                    "--replacement",
                    "replacement",
                ],
            )
            .assert()
            .failure()
            .code(64);
    }
    fixture
        .output_as(
            "sender",
            &[
                "recommend",
                "send",
                "missing",
                "--action",
                "defer",
                "--path",
                "src/legacy.rs",
                "--reason",
                "reason",
                "--replacement",
                "replacement",
            ],
        )
        .assert()
        .failure()
        .code(1);
    fixture
        .output_as(
            "sender",
            &[
                "recommend",
                "send",
                "recipient",
                "--action",
                "defer",
                "--path",
                "../escape.rs",
                "--reason",
                "reason",
                "--replacement",
                "replacement",
            ],
        )
        .assert()
        .failure()
        .code(64);

    let overlong = "界".repeat(2001);
    fixture
        .output_as(
            "sender",
            &[
                "recommend",
                "send",
                "recipient",
                "--action",
                "defer",
                "--path",
                "src/legacy.rs",
                "--reason",
                &overlong,
                "--replacement",
                "replacement",
            ],
        )
        .assert()
        .failure()
        .code(64)
        .stderr(predicate::str::contains("1 to 2000 Unicode characters"));

    let mut too_many = vec![
        "recommend".to_owned(),
        "send".to_owned(),
        "recipient".to_owned(),
        "--action".to_owned(),
        "defer".to_owned(),
        "--reason".to_owned(),
        "reason".to_owned(),
        "--replacement".to_owned(),
        "replacement".to_owned(),
    ];
    for index in 0..51 {
        too_many.push("--path".to_owned());
        too_many.push(format!("src/generated-{index}.rs"));
    }
    fixture
        .output_owned("sender", &too_many)
        .assert()
        .failure()
        .code(64)
        .stderr(predicate::str::contains("1 to 50 scopes"));

    let exact_limit = "界".repeat(2000);
    let sent = fixture.output_as(
        "sender",
        &[
            "recommend",
            "send",
            "recipient",
            "--action",
            "omit",
            "--path",
            "src/legacy.rs",
            "--reason",
            &exact_limit,
            "--replacement",
            "replacement",
        ],
    );
    sent.clone().assert().success();
    let id = tsv_id(&sent.stdout, "RECOMMENDED");
    let shown = fixture.output_as_in("recipient", fixture.outside(), &["recommend", "show", &id, "--json"]);
    let shown: Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(shown["recommendation"]["reason"].as_str().unwrap().chars().count(), 2000);

    fixture
        .output_as_in("sender", fixture.outside(), &["recommend", "list"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("recommend list requires a Git worktree"));
}

fn spawn_synthetic_host(fixture: &Fixture, session_id: &str) -> SyntheticHost {
    let request = fixture.temporary.path().join(format!("{session_id}.request"));
    let response = fixture.temporary.path().join(format!("{session_id}.response"));
    let mut host = fixture.bash_command();
    host.env("AI_COORD_TEST_BIN", BINARY)
        .env("AI_COORD_HOST_REQUEST", &request)
        .env("AI_COORD_HOST_RESPONSE", &response)
        .args([
            "-c",
            "exec -a codex /bin/bash -c 'trap \"exit 130\" INT; \"$AI_COORD_TEST_BIN\" hook codex; while :; do if [ -f \"$AI_COORD_HOST_REQUEST\" ]; then \"$AI_COORD_TEST_BIN\" hook codex < \"$AI_COORD_HOST_REQUEST\" > \"$AI_COORD_HOST_RESPONSE.tmp\"; mv \"$AI_COORD_HOST_RESPONSE.tmp\" \"$AI_COORD_HOST_RESPONSE\"; rm \"$AI_COORD_HOST_REQUEST\"; fi; sleep 0.01; done'",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = host.spawn().expect("start synthetic Codex host");
    let payload = json!({
        "hook_event_name": "SessionStart",
        "session_id": session_id,
        "cwd": fixture.root,
        "transcript_path": format!("opaque:{session_id}"),
    });
    child.stdin.take().unwrap().write_all(payload.to_string().as_bytes()).unwrap();
    SyntheticHost { session_id: session_id.to_owned(), child, request, response }
}

fn assert_strong_session(fixture: &Fixture, session_id: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let found = fixture.status()["sessions"].as_array().is_some_and(|sessions| {
            sessions.iter().any(|session| session["session_id"] == session_id && session["pid"].is_u64())
        });
        if found {
            return;
        }
        assert!(Instant::now() < deadline, "synthetic host session {session_id} never became live");
        thread::sleep(Duration::from_millis(20));
    }
}

fn tsv_id(stdout: &[u8], expected: &str) -> String {
    let line = String::from_utf8_lossy(stdout);
    let (outcome, id) = line.trim().split_once('\t').expect("TSV outcome");
    assert_eq!(outcome, expected);
    id.to_owned()
}

fn git_status(root: &Path) -> String {
    let output = Command::new("/usr/bin/git").args(["status", "--short"]).current_dir(root).output().unwrap();
    assert!(output.status.success(), "git status: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

#![cfg(unix)]

use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use assert_cmd::assert::Assert;
use predicates::prelude::*;
use rusqlite::Connection;
use serde_json::{Value, json};
use tempfile::TempDir;

const BINARY: &str = env!("CARGO_BIN_EXE_ai-coord");

trait OutputAssertExt {
    fn assert(&self) -> Assert;
}

impl OutputAssertExt for Output {
    fn assert(&self) -> Assert {
        Assert::new(Output { status: self.status, stdout: self.stdout.clone(), stderr: self.stderr.clone() })
    }
}

struct Fixture {
    _temporary: TempDir,
    root: PathBuf,
    state: PathBuf,
    codex_home: PathBuf,
    claude_home: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let root = temporary.path().join("repo");
        let state = temporary.path().join("state");
        let codex_home = temporary.path().join("codex");
        let claude_home = temporary.path().join("claude");
        fs::create_dir_all(root.join("src")).expect("repository directories");
        let result = Command::new("git").args(["init", "--quiet"]).current_dir(&root).output().expect("git init");
        assert!(result.status.success(), "git init: {}", String::from_utf8_lossy(&result.stderr));
        Self { _temporary: temporary, root, state, codex_home, claude_home }
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
            .env("AI_COORD_SESSION_ID", "cli-test")
            .env("CODEX_HOME", &self.codex_home)
            .env("CLAUDE_CONFIG_DIR", &self.claude_home)
            .env("HOME", self._temporary.path().join("home"))
            .env("PATH", "/usr/bin:/bin")
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CLAUDE_CODE_SESSION_ID");
    }

    fn output(&self, arguments: &[&str]) -> Output {
        self.command().args(arguments).output().expect("run ai-coord")
    }

    fn output_as(&self, session_id: &str, arguments: &[&str]) -> Output {
        self.command().env("AI_COORD_SESSION_ID", session_id).args(arguments).output().expect("run ai-coord as session")
    }

    fn output_as_in(&self, session_id: &str, cwd: &std::path::Path, arguments: &[&str]) -> Output {
        self.command()
            .current_dir(cwd)
            .env("AI_COORD_SESSION_ID", session_id)
            .args(arguments)
            .output()
            .expect("run ai-coord as session in directory")
    }

    fn output_with_path(&self, arguments: &[&str], executable_path: &std::path::Path) -> Output {
        self.command().env("PATH", executable_path).args(arguments).output().expect("run ai-coord with PATH")
    }

    fn output_as_with_hash_log(
        &self,
        session_id: &str,
        arguments: &[&str],
        executable_path: &str,
        log: &Path,
    ) -> Output {
        self.command()
            .env("AI_COORD_SESSION_ID", session_id)
            .env("AI_COORD_HASH_OBJECT_LOG", log)
            .env("PATH", executable_path)
            .args(arguments)
            .output()
            .expect("run ai-coord with Git hash-object logger")
    }

    fn json_status(&self) -> (i32, Value) {
        let output = self.output(&["status", "--all", "--json"]);
        let payload = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "status JSON: {error}; stderr={} stdout={}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            )
        });
        (output.status.code().expect("status code"), payload)
    }
}

#[test]
fn parser_and_semantic_usage_keep_distinct_exit_codes() {
    let fixture = Fixture::new();

    let help = fixture.output(&["--help"]);
    help.assert().success();
    assert!(String::from_utf8_lossy(&help.stdout).contains("Coordinate parallel Codex and Claude Code agents"));

    let parser_error = fixture.output(&["wait", "--timeout-seconds", "0"]);
    parser_error.assert().failure().code(2);
    parser_error.assert().stdout(predicate::str::is_empty());
    assert!(String::from_utf8_lossy(&parser_error.stderr).starts_with("error: invalid value '0'"));

    let semantic_error = fixture.output(&["finding", "add", "   "]);
    semantic_error.assert().failure().code(64);
    semantic_error.assert().stdout(predicate::str::is_empty());
    assert_eq!(String::from_utf8_lossy(&semantic_error.stderr), "error: finding summary must contain text\n");

    let removed_note = fixture.output(&["note", "old"]);
    removed_note.assert().failure().code(2);
    assert!(String::from_utf8_lossy(&removed_note.stderr).contains("unrecognized subcommand 'note'"));

    let conflicting_inbox = fixture.output(&["inbox", "--ack", "abc", "--ack-all"]);
    conflicting_inbox.assert().failure().code(64);
    assert_eq!(String::from_utf8_lossy(&conflicting_inbox.stderr), "error: use only one of --ack or --ack-all\n");
}

#[test]
fn identity_commands_and_state_are_fully_isolated() {
    let fixture = Fixture::new();

    let named = fixture.output(&["name", "🦀 Ferris Test"]);
    named.assert().success();
    assert_eq!(String::from_utf8_lossy(&named.stdout), "NAMED\t🦀 Ferris Test\n");

    let trailer = fixture.output(&["trailer"]);
    trailer.assert().success();
    assert_eq!(String::from_utf8_lossy(&trailer.stdout), "Agent-Session: codex/cli-test\n");

    let finding = fixture.output(&["finding", "add", "--kind", "bug", "--path", "src/lib.rs", "integration finding"]);
    finding.assert().success();
    assert!(String::from_utf8_lossy(&finding.stdout).starts_with("ADDED\t"));

    let (code, status) = fixture.json_status();
    assert!(
        matches!(code, 0 | 2),
        "status is complete under a detectable Codex ancestor and partial when the test host is unknown"
    );
    assert_eq!(status["schema_version"], 7);
    assert_eq!(status["scope"]["kind"], "machine");
    assert_eq!(status["sessions"][0]["callsign"], "🦀 Ferris Test");
    assert_eq!(status["sessions"][0]["coordination_waived"], false);
    assert!(fixture.state.join("state.db").is_file());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&fixture.state).unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(fs::metadata(fixture.state.join("state.db")).unwrap().permissions().mode() & 0o777, 0o600);
    }
}

#[test]
fn hook_input_is_fail_open_and_never_echoes_payload() {
    let fixture = Fixture::new();

    let malformed = run_with_stdin(fixture.command(), &["hook", "codex"], b"not json");
    malformed.assert().success();
    malformed.assert().stdout(predicate::str::is_empty());
    malformed.assert().stderr(predicate::str::is_empty());

    let stop =
        run_with_stdin(fixture.command(), &["hook", "codex"], br#"{"hook_event_name":"Stop","private":"do not leak"}"#);
    stop.assert().success();
    assert_eq!(String::from_utf8_lossy(&stop.stdout), "{}\n");
    assert!(!String::from_utf8_lossy(&stop.stdout).contains("do not leak"));
    stop.assert().stderr(predicate::str::is_empty());

    let waker = run_with_stdin(fixture.command(), &["waker", "claude"], b"not json");
    waker.assert().success();
    waker.assert().stdout(predicate::str::is_empty());
    waker.assert().stderr(predicate::str::is_empty());
}

#[test]
fn coordination_commands_preserve_tsv_outputs_and_embedded_codes() {
    let fixture = Fixture::new();
    let mut sender = spawn_synthetic_host(&fixture, "sender-host");
    let mut recipient = spawn_synthetic_host(&fixture, "recipient-host");
    assert_strong_session(&fixture, "sender-host");
    assert_strong_session(&fixture, "recipient-host");

    let sender_name = fixture.output_as("sender-host", &["name", "🦀 Sender"]);
    let recipient_name = fixture.output_as("recipient-host", &["name", "🐙 Recipient"]);
    assert_eq!(String::from_utf8_lossy(&sender_name.stdout), "NAMED\t🦀 Sender\n");
    assert_eq!(String::from_utf8_lossy(&recipient_name.stdout), "NAMED\t🐙 Recipient\n");

    let start = fixture.output_as("sender-host", &["start", "exact work", "src/app.rs"]);
    start.assert().success();
    assert_eq!(String::from_utf8_lossy(&start.stdout), "READY\tsrc/app.rs\n");

    let wait = fixture.output_as("sender-host", &["wait", "-t", "1"]);
    wait.assert().success();
    assert_eq!(String::from_utf8_lossy(&wait.stdout), "READY\tsrc/app.rs\n");

    let baseline = fixture.output_as("sender-host", &["baseline"]);
    baseline.assert().success();
    baseline.assert().stdout(predicate::str::is_empty());

    let sent = fixture.output_as("sender-host", &["msg", "recipient-host", "ready for review"]);
    sent.assert().success();
    assert!(String::from_utf8_lossy(&sent.stdout).starts_with("SENT\t1\t"));

    let inbox = fixture.output_as("recipient-host", &["inbox"]);
    inbox.assert().success();
    let inbox_text = String::from_utf8_lossy(&inbox.stdout);
    assert!(inbox_text.starts_with("ID\tAGE\tFROM\tTEXT\n"));
    assert!(inbox_text.contains("\t🦀 Sender\tready for review\n"));
    let acknowledged = fixture.output_as("recipient-host", &["inbox", "--ack-all"]);
    assert_eq!(String::from_utf8_lossy(&acknowledged.stdout), "ACK\t1\n");

    let finding = fixture.output_as("sender-host", &["finding", "add", "durable finding"]);
    let finding_id = String::from_utf8_lossy(&finding.stdout).trim().strip_prefix("ADDED\t").unwrap().to_owned();
    let resolved = fixture.output_as("sender-host", &["finding", "resolve", &finding_id, "--as", "fixed"]);
    assert_eq!(String::from_utf8_lossy(&resolved.stdout), format!("RESOLVED\t{finding_id}\tfixed\n"));

    let done = fixture.output_as("sender-host", &["done"]);
    assert_eq!(String::from_utf8_lossy(&done.stdout), "DONE\treleased\n");
    let repeated = fixture.output_as("sender-host", &["done"]);
    assert_eq!(String::from_utf8_lossy(&repeated.stdout), "DONE\talready clear\n");
    let draft = fixture.output_as("sender-host", &["draft", "planning only", "src/planned.rs"]);
    assert_eq!(String::from_utf8_lossy(&draft.stdout), "DRAFT\t1\n");

    let _ = sender.kill();
    let _ = sender.wait();
    let _ = recipient.kill();
    let _ = recipient.wait();
}

#[test]
fn bundle_cli_lifecycle_is_atomic_and_uses_v7_claims() {
    let fixture = Fixture::new();
    let second = fixture._temporary.path().join("z-repo");
    fs::create_dir_all(second.join("src")).unwrap();
    assert!(Command::new("git").args(["init", "--quiet"]).current_dir(&second).status().unwrap().success());
    let mut host = spawn_synthetic_host(&fixture, "multi-host");
    assert_strong_session(&fixture, "multi-host");
    let first_path = fixture.root.join("src/a.rs").to_string_lossy().into_owned();
    let second_path = second.join("src/b.rs").to_string_lossy().into_owned();
    let first_root = fs::canonicalize(&fixture.root).unwrap().to_string_lossy().into_owned();
    let second_root = fs::canonicalize(&second).unwrap().to_string_lossy().into_owned();
    let expected_first_path = format!("{first_root}/src/a.rs");
    let expected_second_path = format!("{second_root}/src/b.rs");

    let start =
        fixture.output_as_in("multi-host", &fixture.root, &["bundle", "start", "two roots", &second_path, &first_path]);
    start.assert().success();
    assert_eq!(
        String::from_utf8_lossy(&start.stdout),
        format!("READY\t{expected_first_path}\t{expected_second_path}\n")
    );

    let repo_status = fixture.output_as_in("multi-host", &fixture.root, &["status", "--json"]);
    assert!(matches!(repo_status.status.code(), Some(0 | 2)));
    let repo_status: Value = serde_json::from_slice(&repo_status.stdout).unwrap();
    assert_eq!(repo_status["schema_version"], 7);
    assert!(repo_status["sessions"].as_array().unwrap().iter().any(|row| row["session_id"] == "multi-host"));
    let repo_work = repo_status["work"].as_array().unwrap();
    assert_eq!(repo_work.len(), 1);
    assert_eq!(repo_work[0]["label"], "two roots");
    assert_eq!(repo_work[0]["scope_count"], 2);
    assert_eq!(repo_work[0]["claims"].as_array().unwrap().len(), 2);
    assert_eq!(repo_work[0]["claims"][0]["repo_root"], first_root);
    assert_eq!(repo_work[0]["claims"][1]["repo_root"], second_root);

    let terminal = fixture.output_as_in("multi-host", &second, &["status", "--all"]);
    assert!(matches!(terminal.status.code(), Some(0 | 2)));
    let terminal = String::from_utf8_lossy(&terminal.stdout);
    assert_eq!(terminal.lines().filter(|line| line.contains("\tmulti-host\t")).count(), 1);
    assert!(terminal.contains(&format!("repos={first_root},{second_root}")));
    assert!(terminal.contains(&format!("paths={expected_first_path},{expected_second_path}")));

    assert_eq!(
        String::from_utf8_lossy(&fixture.output_as_in("multi-host", &second, &["done"]).stdout),
        "DONE\treleased\n"
    );

    let _ = host.kill();
    let _ = host.wait();
}

#[test]
fn bundle_draft_promotion_and_ordinary_mismatch_are_explicit() {
    let fixture = Fixture::new();
    let second = fixture._temporary.path().join("z-repo");
    fs::create_dir_all(second.join("src")).unwrap();
    assert!(Command::new("git").args(["init", "--quiet"]).current_dir(&second).status().unwrap().success());
    let mut host = spawn_synthetic_host(&fixture, "order-host");
    assert_strong_session(&fixture, "order-host");
    let first_path = fixture.root.join("src/a.rs").to_string_lossy().into_owned();
    let second_path = second.join("src/b.rs").to_string_lossy().into_owned();
    let expected_first_path = format!("{}/src/a.rs", fs::canonicalize(&fixture.root).unwrap().display());
    let expected_second_path = format!("{}/src/b.rs", fs::canonicalize(&second).unwrap().display());

    let draft =
        fixture.output_as_in("order-host", &fixture.root, &["bundle", "draft", "draft", &first_path, &second_path]);
    assert_eq!(String::from_utf8_lossy(&draft.stdout), "DRAFT\t2\n");
    let rejected = fixture.output_as_in("order-host", &fixture.root, &["start", "--draft"]);
    rejected.assert().failure().code(1);
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("bundle start --draft"));
    let promoted = fixture.output_as_in("order-host", &fixture.root, &["bundle", "start", "--draft"]);
    assert_eq!(
        String::from_utf8_lossy(&promoted.stdout),
        format!("READY\t{expected_first_path}\t{expected_second_path}\n")
    );
    fixture.output_as_in("order-host", &fixture.root, &["done"]);

    fixture.output_as_in("order-host", &fixture.root, &["draft", "ordinary", "src/ordinary.rs"]).assert().success();
    let rejected = fixture.output_as_in("order-host", &fixture.root, &["bundle", "start", "--draft"]);
    rejected.assert().failure().code(1);
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("ai-coord start --draft"));
    fixture.output_as_in("order-host", &fixture.root, &["done"]);

    let _ = host.kill();
    let _ = host.wait();
}

#[test]
fn bundle_validation_and_done_all_parser_error_leave_work_unchanged() {
    let fixture = Fixture::new();
    let second = fixture._temporary.path().join("z-repo");
    fs::create_dir_all(second.join("src")).unwrap();
    assert!(Command::new("git").args(["init", "--quiet"]).current_dir(&second).status().unwrap().success());
    let first_path = fixture.root.join("src/a.rs").to_string_lossy().into_owned();
    let second_path = second.join("src/b.rs").to_string_lossy().into_owned();
    let non_git_path = fixture._temporary.path().join("not-a-repository/file.rs").to_string_lossy().into_owned();
    let mut host = spawn_synthetic_host(&fixture, "validation-host");
    assert_strong_session(&fixture, "validation-host");
    fixture
        .output_as_in("validation-host", &fixture.root, &["bundle", "draft", "draft", &first_path, &second_path])
        .assert()
        .success();
    let before = fixture.json_status().1["work"].clone();
    for arguments in [
        vec!["bundle", "start", "relative", "src/a.rs", "src/b.rs"],
        vec!["bundle", "start", "one root", &first_path, &fixture.root.join("src/c.rs").to_string_lossy()],
        vec!["bundle", "start", "non git", &first_path, &non_git_path],
        vec!["bundle", "start", "empty"],
        vec![
            "bundle",
            "start",
            "recursive roots",
            "--recursive",
            &fixture.root.to_string_lossy(),
            "--recursive",
            &second.to_string_lossy(),
        ],
    ] {
        let result = fixture.output_as_in("validation-host", &fixture.root, &arguments);
        result.assert().failure();
        assert_eq!(fixture.json_status().1["work"], before);
    }
    let done_all = fixture.output_as_in("validation-host", &fixture.root, &["done", "--all"]);
    done_all.assert().failure().code(2);
    assert!(String::from_utf8_lossy(&done_all.stderr).contains("unexpected argument '--all'"));
    assert_eq!(fixture.json_status().1["work"], before);
    fixture.output_as_in("validation-host", &fixture.root, &["done"]);
    let _ = host.kill();
    let _ = host.wait();
}

#[test]
fn finding_commands_deduplicate_sightings_and_enforce_lifecycle_evidence() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.root.join("docs")).unwrap();
    fs::write(fixture.root.join("src/a.rs"), "fn a() {}\n").unwrap();
    fs::write(fixture.root.join("docs/a.md"), "# A\n").unwrap();
    let absolute = fixture.root.join("src/a.rs").to_string_lossy().into_owned();

    let first = fixture.output(&[
        "finding",
        "add",
        "--kind",
        "bug",
        "--path",
        "src/a.rs",
        "--path",
        "docs/a.md",
        "shared failure",
    ]);
    first.assert().success();
    let first_id = String::from_utf8_lossy(&first.stdout).trim().strip_prefix("ADDED\t").unwrap().to_owned();

    let duplicate = fixture.output(&[
        "finding",
        "add",
        "--kind",
        "docs",
        "--path",
        "docs/a.md",
        "--path",
        &absolute,
        "shared   failure",
    ]);
    assert_eq!(String::from_utf8_lossy(&duplicate.stdout), format!("SIGHTING\t{first_id}\n"));

    let related = fixture.output(&["finding", "add", "--path", "src/a.rs", "related failure"]);
    let related_output = String::from_utf8_lossy(&related.stdout);
    assert!(related_output.starts_with("ADDED\t"));
    assert!(related_output.contains(&format!("CANDIDATE\t{first_id}\tshared failure\n")));

    let shown = fixture.output(&["finding", "show", &first_id, "--json"]);
    let shown: Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(shown["kind"], "bug", "exact dedup ignores and preserves kind");
    assert_eq!(shown["state"], "pending");
    assert_eq!(shown["paths"], json!(["docs/a.md", "src/a.rs"]));
    assert_eq!(shown["sighting_count"], 2);
    assert_eq!(shown["triaging"], false);

    let handed_off = fixture.output(&["finding", "handoff", &first_id, "--path", &absolute]);
    assert_eq!(String::from_utf8_lossy(&handed_off.stdout), format!("HANDED_OFF\t{first_id}\tsrc/a.rs\n"));
    let resolved = fixture.output(&["finding", "resolve", &first_id, "--as", "fixed", "--commit", "abcdef0"]);
    assert_eq!(String::from_utf8_lossy(&resolved.stdout), format!("RESOLVED\t{first_id}\tfixed\n"));

    let open: Value = serde_json::from_slice(&fixture.output(&["finding", "list", "--json"]).stdout).unwrap();
    assert!(open.as_array().unwrap().iter().all(|finding| finding["id"] != first_id));
    let all: Value = serde_json::from_slice(&fixture.output(&["finding", "list", "--all", "--json"]).stdout).unwrap();
    assert!(all.as_array().unwrap().iter().any(|finding| finding["id"] == first_id));

    let recurrence = fixture.output(&["finding", "add", "--path", "src/a.rs", "--path", "docs/a.md", "shared failure"]);
    let recurrence_id =
        String::from_utf8_lossy(&recurrence.stdout).lines().next().unwrap().strip_prefix("ADDED\t").unwrap().to_owned();
    assert_ne!(recurrence_id, first_id);
    let missing_canonical = fixture.output(&["finding", "resolve", &recurrence_id, "--as", "duplicate"]);
    missing_canonical.assert().failure().code(64);
    assert!(String::from_utf8_lossy(&missing_canonical.stderr).contains("--canonical is required"));
    let marked_duplicate =
        fixture.output(&["finding", "resolve", &recurrence_id, "--as", "duplicate", "--canonical", &first_id]);
    marked_duplicate.assert().success();
    assert_eq!(
        String::from_utf8_lossy(&fixture.output(&["finding", "reopen", &first_id]).stdout),
        format!("REOPENED\t{first_id}\n")
    );

    let outside = fixture._temporary.path().join("outside.txt");
    fs::write(&outside, "outside\n").unwrap();
    std::os::unix::fs::symlink(&outside, fixture.root.join("outside-link")).unwrap();
    let escaped = fixture.output(&["finding", "add", "--path", "outside-link", "must reject escape"]);
    escaped.assert().failure().code(64);
    assert!(String::from_utf8_lossy(&escaped.stderr).contains("finding path escapes repository"));

    let connection = Connection::open(fixture.state.join("state.db")).unwrap();
    let observations: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM finding_observations o
             JOIN finding_sightings s ON s.id = o.sighting_id
             WHERE s.finding_id = ?1 AND o.content_sha256 IS NOT NULL",
            [&first_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(observations, 4, "both paths are observed for both exact sightings");
}

#[test]
fn draft_create_replace_promote_and_done_preserve_scope_privacy() {
    let fixture = Fixture::new();
    let mut host = spawn_synthetic_host(&fixture, "draft-host");
    assert_strong_session(&fixture, "draft-host");

    let created = fixture.output_as("draft-host", &["draft", "private plan", "src/private.rs", "docs/private.md"]);
    created.assert().success();
    assert_eq!(String::from_utf8_lossy(&created.stdout), "DRAFT\t2\n");

    let (_, snapshot) = fixture.json_status();
    let draft = snapshot["work"].as_array().unwrap().iter().find(|work| work["session_id"] == "draft-host").unwrap();
    assert_eq!(draft["state"], "draft");
    assert_eq!(draft["scope_count"], 2);
    assert!(draft.get("scopes").is_none());
    assert_eq!(draft["claims"][0]["scope_count"], 2);
    assert!(draft["claims"][0].get("scopes").is_none());
    assert!(!serde_json::to_string(draft).unwrap().contains("private.rs"));

    let replaced = fixture.output_as("draft-host", &["draft", "revised plan", "--recursive", "src"]);
    assert_eq!(String::from_utf8_lossy(&replaced.stdout), "DRAFT\t1\n");
    let bypass = fixture.output_as("draft-host", &["start", "drifted execution", "src/other.rs"]);
    bypass.assert().failure().code(1);
    assert!(String::from_utf8_lossy(&bypass.stderr).contains("a draft exists"));

    let promoted = fixture.output_as("draft-host", &["start", "--draft"]);
    promoted.assert().success();
    assert_eq!(String::from_utf8_lossy(&promoted.stdout), "READY\tsrc\n");
    let (_, snapshot) = fixture.json_status();
    let active = snapshot["work"].as_array().unwrap().iter().find(|work| work["session_id"] == "draft-host").unwrap();
    assert_eq!(active["state"], "active");
    assert_eq!(active["scope_count"], 1);
    assert_eq!(active["claims"][0]["scopes"], json!([{"path":"src", "kind":"recursive"}]));

    let rejected = fixture.output_as("draft-host", &["draft", "must release", "src/new.rs"]);
    rejected.assert().failure().code(1);
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("run ai-coord done"));
    assert_eq!(String::from_utf8_lossy(&fixture.output_as("draft-host", &["done"]).stdout), "DONE\treleased\n");
    assert!(fixture.json_status().1["work"].as_array().unwrap().is_empty());

    let _ = host.kill();
    let _ = host.wait();
}

#[test]
fn draft_and_direct_start_require_scopes_and_draft_promotion_is_exclusive() {
    let fixture = Fixture::new();
    for arguments in [["draft", "empty"].as_slice(), ["start", "empty"].as_slice()] {
        let output = fixture.output(arguments);
        output.assert().failure().code(64);
        assert_eq!(String::from_utf8_lossy(&output.stderr), "error: at least one scope is required\n");
    }

    let conflict = fixture.output(&["start", "--draft", "label"]);
    conflict.assert().failure().code(2);
    assert!(String::from_utf8_lossy(&conflict.stderr).contains("--draft"));
}

#[test]
fn directory_scope_errors_include_copy_paste_ready_recursive_commands() {
    let fixture = Fixture::new();

    for (arguments, expected) in [
        (
            ["start", "regenerate all reports", "src"].as_slice(),
            "re-run: ai-coord start --recursive 'src' 'regenerate all reports'",
        ),
        (
            ["draft", "regenerate all reports", "src"].as_slice(),
            "re-run: ai-coord draft --recursive 'src' 'regenerate all reports'",
        ),
        (
            ["start", "--recursive", "regenerate all reports", "src"].as_slice(),
            "re-run: ai-coord start --recursive 'src' 'regenerate all reports'",
        ),
    ] {
        let output = fixture.output(arguments);
        output.assert().failure().code(64);
        output.assert().stdout(predicate::str::is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
    }
}

#[test]
fn labels_that_fail_scope_normalization_do_not_break_validation() {
    let fixture = Fixture::new();

    let output = fixture.output(&["draft", "fix [2025] *reports* under ~", "tracked.txt"]);
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "DRAFT\t1\n");
}

#[test]
fn promotion_revalidates_paths_and_repository_without_consuming_the_draft() {
    let fixture = Fixture::new();
    let mut host = spawn_synthetic_host(&fixture, "revalidate-host");
    assert_strong_session(&fixture, "revalidate-host");

    let drafted = fixture.output_as("revalidate-host", &["draft", "revalidate me", "--recursive", "planned"]);
    assert_eq!(String::from_utf8_lossy(&drafted.stdout), "DRAFT\t1\n");
    fs::write(fixture.root.join("planned"), "now a file\n").unwrap();
    let invalid = fixture.output_as("revalidate-host", &["start", "--draft"]);
    invalid.assert().failure().code(64);
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("recursive scope is not a directory: planned"));
    assert_eq!(work_state(&fixture, "revalidate-host"), Some("draft".to_owned()));

    fs::remove_file(fixture.root.join("planned")).unwrap();
    let other = fixture._temporary.path().join("other-repo");
    fs::create_dir(&other).unwrap();
    assert!(Command::new("git").args(["init", "--quiet"]).current_dir(&other).status().unwrap().success());
    let mismatch = fixture.output_as_in("revalidate-host", &other, &["start", "--draft"]);
    mismatch.assert().failure().code(1);
    assert!(String::from_utf8_lossy(&mismatch.stderr).contains("draft belongs to"));
    assert_eq!(work_state(&fixture, "revalidate-host"), Some("draft".to_owned()));

    let _ = host.kill();
    let _ = host.wait();
}

#[test]
fn promotion_queues_on_unknown_coverage_and_wait_preserves_submitted_work() {
    let fixture = Fixture::new();
    let bin = fixture._temporary.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let codex = bin.join("codex");
    fs::write(&codex, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&codex, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let executable_path = format!("{}:/usr/bin:/bin", bin.display());
    assert_eq!(
        String::from_utf8_lossy(&fixture.output(&["draft", "unknown work", "src/unknown.rs"]).stdout),
        "DRAFT\t1\n"
    );

    let promoted = fixture.output_with_path(&["start", "--draft"], std::path::Path::new(&executable_path));
    promoted.assert().failure().code(2);
    assert_eq!(String::from_utf8_lossy(&promoted.stdout), "UNKNOWN\tcoverage\n");
    assert_eq!(work_state(&fixture, "cli-test"), Some("queued".to_owned()));
    let work = work_item(&fixture, "cli-test").unwrap();
    assert_eq!(work["scope_count"], 1);
    assert_eq!(work["claims"][0]["scopes"], json!([{"path":"src/unknown.rs", "kind":"exact"}]));

    let waited = fixture.output_with_path(&["wait", "-t", "1"], std::path::Path::new(&executable_path));
    waited.assert().failure().code(2);
    assert_eq!(String::from_utf8_lossy(&waited.stdout), "UNKNOWN\tcoverage\n");
    assert_eq!(String::from_utf8_lossy(&fixture.output(&["done"]).stdout), "DONE\treleased\n");
}

#[test]
fn blocked_draft_promotion_hashes_only_dirt_in_the_requested_exact_scope() {
    let fixture = Fixture::new();
    fs::write(fixture.root.join(".gitignore"), "target/\n").unwrap();
    commit_paths(&fixture, &[".gitignore"]);
    assert!(git_status(&fixture).is_empty(), "holder must start from clean Git dirt");

    let mut holder = spawn_synthetic_host(&fixture, "hash-holder");
    let mut contender = spawn_synthetic_host(&fixture, "hash-contender");
    assert_strong_session(&fixture, "hash-holder");
    assert_strong_session(&fixture, "hash-contender");
    assert_eq!(
        String::from_utf8_lossy(&fixture.output_as("hash-holder", &["name", "🧱 Holder"]).stdout),
        "NAMED\t🧱 Holder\n"
    );
    assert_eq!(
        String::from_utf8_lossy(&fixture.output_as("hash-holder", &["start", "ignore owner", ".gitignore"]).stdout),
        "READY\t.gitignore\n"
    );

    fs::create_dir(fixture.root.join("unrelated")).unwrap();
    for index in 0..300 {
        fs::write(fixture.root.join(format!("unrelated/file-{index:03}.txt")), format!("{index}\n")).unwrap();
    }
    fs::write(fixture.root.join(".gitignore"), "target/\nbuild/\n").unwrap();
    assert_eq!(
        String::from_utf8_lossy(
            &fixture.output_as("hash-contender", &["draft", "promote blocked ignore", ".gitignore"]).stdout
        ),
        "DRAFT\t1\n"
    );

    let (executable_path, log) = install_hash_object_logger(&fixture);
    let promoted = fixture.output_as_with_hash_log("hash-contender", &["start", "--draft"], &executable_path, &log);
    promoted.assert().failure().code(3);
    assert_eq!(String::from_utf8_lossy(&promoted.stdout), "BLOCKED\t🧱 Holder\t.gitignore\n");
    assert_eq!(hash_object_invocations(&log), 1);

    for child in [&mut holder, &mut contender] {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[test]
fn recursive_scope_dirty_settling_uses_bounded_hash_object_batches() {
    let fixture = Fixture::new();
    let mut host = spawn_synthetic_host(&fixture, "hash-recursive");
    assert_strong_session(&fixture, "hash-recursive");
    fs::create_dir(fixture.root.join("bulk")).unwrap();
    for index in 0..300 {
        fs::write(fixture.root.join(format!("bulk/file-{index:03}.txt")), format!("{index}\n")).unwrap();
    }

    let (executable_path, log) = install_hash_object_logger(&fixture);
    let started = fixture.output_as_with_hash_log(
        "hash-recursive",
        &["start", "bulk dirt", "--recursive", "bulk"],
        &executable_path,
        &log,
    );
    started.assert().failure().code(2);
    assert!(
        String::from_utf8_lossy(&started.stdout).starts_with("UNKNOWN\tdirty-settling:bulk/file-000.txt"),
        "{}",
        String::from_utf8_lossy(&started.stdout)
    );
    assert!((1..=3).contains(&hash_object_invocations(&log)));

    let _ = host.kill();
    let _ = host.wait();
}

#[test]
fn fifo_age_begins_at_draft_promotion_not_draft_creation() {
    let fixture = Fixture::new();
    let mut holder = spawn_synthetic_host(&fixture, "fifo-holder");
    let mut drafted = spawn_synthetic_host(&fixture, "fifo-drafted");
    let mut direct = spawn_synthetic_host(&fixture, "fifo-direct");
    for session in ["fifo-holder", "fifo-drafted", "fifo-direct"] {
        assert_strong_session(&fixture, session);
    }
    let scope = "src/fifo.rs";
    assert_eq!(
        String::from_utf8_lossy(&fixture.output_as("fifo-holder", &["start", "holder", scope]).stdout),
        format!("READY\t{scope}\n")
    );
    assert_eq!(
        String::from_utf8_lossy(&fixture.output_as("fifo-drafted", &["draft", "drafted", scope]).stdout),
        "DRAFT\t1\n"
    );
    thread::sleep(Duration::from_millis(20));
    assert_eq!(fixture.output_as("fifo-direct", &["start", "direct", scope]).status.code(), Some(3));
    thread::sleep(Duration::from_millis(20));
    assert_eq!(fixture.output_as("fifo-drafted", &["start", "--draft"]).status.code(), Some(3));

    let direct_work = work_item(&fixture, "fifo-direct").unwrap();
    let drafted_work = work_item(&fixture, "fifo-drafted").unwrap();
    assert!(
        direct_work["submitted_at"].as_f64().unwrap() < drafted_work["submitted_at"].as_f64().unwrap(),
        "draft creation must not establish FIFO age"
    );
    assert!(drafted_work["draft_created_at"].as_f64().unwrap() < drafted_work["submitted_at"].as_f64().unwrap());

    fixture.output_as("fifo-holder", &["done"]);
    assert_eq!(
        fixture.output_as("fifo-direct", &["start", "direct", scope]).status.code(),
        Some(0),
        "the earlier submitted direct work should promote first"
    );
    fixture.output_as("fifo-drafted", &["inbox", "--ack-all"]);
    let still_queued = fixture.output_as("fifo-drafted", &["start", "drafted", scope]);
    still_queued.assert().failure().code(3);

    for child in [&mut holder, &mut drafted, &mut direct] {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[test]
fn link_and_check_use_only_the_configured_temporary_roots() {
    let fixture = Fixture::new();
    let claude_settings = fixture.claude_home.join("alternate.json");
    let path = claude_settings.to_string_lossy();

    let preview = fixture.output(&["link", "claude", "--path", &path, "--dry-run"]);
    preview.assert().success();
    assert_eq!(
        String::from_utf8_lossy(&preview.stdout),
        format!("WOULD_UPDATE\tclaude\t{}\ttrust=skipped\n", claude_settings.display())
    );
    assert!(!claude_settings.exists());

    let linked = fixture.output(&["link", "claude", "--path", &path]);
    linked.assert().success();
    assert!(claude_settings.is_file());
    assert!(String::from_utf8_lossy(&linked.stdout).starts_with("UPDATED\tclaude\t"));

    let repeated = fixture.output(&["link", "claude", "--path", &path]);
    repeated.assert().success();
    assert!(String::from_utf8_lossy(&repeated.stdout).starts_with("OK\tclaude\t"));

    let malformed = fixture.claude_home.join("malformed.json");
    fs::write(&malformed, br#"{"hooks":[]}"#).unwrap();
    let rejected = fixture.output(&["link", "claude", "--path", &malformed.to_string_lossy()]);
    rejected.assert().failure().code(64);
    assert_eq!(
        String::from_utf8_lossy(&rejected.stderr),
        "error: hooks field must be an object; pass --force to replace it\n"
    );

    let check = fixture.output(&["check", "--json"]);
    check.assert().failure().code(2);
    let reports: Vec<Value> = serde_json::from_slice(&check.stdout).expect("check JSON");
    let state = reports.iter().find(|report| report["component"] == "state").expect("state report");
    assert_eq!(state["schema_version"], 15);
    assert_eq!(state["path"], fixture.state.join("state.db").to_string_lossy().as_ref());
    let codex_hooks = reports.iter().find(|report| report["component"] == "hooks:codex").expect("hook report");
    assert!(codex_hooks["error"].is_null());
    assert_eq!(codex_hooks["missing"].as_array().map(Vec::len), Some(7));
    assert!(reports.iter().any(|report| report["component"] == "hooks-trust:codex"));
}

#[test]
fn dashboard_snapshot_matches_the_frontend_shape_and_ctrl_c_is_graceful() {
    let fixture = Fixture::new();
    let added = fixture.output(&["finding", "add", "--path", "docs/api.md", "SSE fixture finding"]);
    let finding_id = String::from_utf8_lossy(&added.stdout).trim().strip_prefix("ADDED\t").unwrap().to_owned();
    let port = unused_port();
    let mut server = fixture.command();
    let mut child = server
        .args(["serve", "--port", &port.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start server");
    let response = request_when_ready(port, "/api/snapshot", Duration::from_secs(5));
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    let body = response.split_once("\r\n\r\n").expect("HTTP body").1;
    let payload: Value = serde_json::from_str(body).expect("dashboard JSON");
    for key in [
        "schema_version",
        "complete",
        "scope",
        "self",
        "providers",
        "sessions",
        "work",
        "findings",
        "delegates",
        "outside_scope",
        "messages",
        "generated_at",
        "generation",
    ] {
        assert!(payload.get(key).is_some(), "missing dashboard field {key}");
    }
    let finding = payload["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["id"] == finding_id)
        .expect("snapshot finding");
    for key in [
        "id",
        "repo_root",
        "summary",
        "kind",
        "state",
        "paths",
        "created_at",
        "updated_at",
        "terminal_at",
        "handoff_path",
        "commit_oid",
        "canonical_id",
        "sighting_count",
        "triaging",
    ] {
        assert!(finding.get(key).is_some(), "missing dashboard finding field {key}");
    }
    assert!(finding["kind"].is_null());
    assert!(finding["terminal_at"].is_null());
    assert!(finding["handoff_path"].is_null());
    assert!(finding["commit_oid"].is_null());
    assert!(finding["canonical_id"].is_null());
    assert_eq!(finding["sighting_count"], 1);
    assert_eq!(finding["triaging"], false);

    send_signal(&child, libc::SIGINT);
    wait_for_exit(&mut child, Duration::from_secs(5));
    let output = child.wait_with_output().expect("server output");
    output.assert().success();
    assert!(
        String::from_utf8_lossy(&output.stdout).contains(&format!("Serving dashboard API at http://127.0.0.1:{port}"))
    );
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn status_removes_every_common_host_termination_without_an_age_grace() {
    let fixture = Fixture::new();
    for (label, signal) in [
        ("terminal-close", libc::SIGHUP),
        ("ctrl-c", libc::SIGINT),
        ("terminated", libc::SIGTERM),
        ("crashed", libc::SIGKILL),
    ] {
        let session_id = format!("{label}-host");
        let mut child = spawn_synthetic_host(&fixture, &session_id);
        assert_strong_session(&fixture, &session_id);

        send_signal(&child, signal);
        // Reconcile before `wait`: SIGKILL therefore exercises an unreaped
        // zombie, while the other signals cover normal terminal teardown.
        let removed = wait_for_session_absence(&fixture, &session_id, Duration::from_secs(3));
        if !removed {
            let _ = child.kill();
        }
        let _ = child.wait();
        assert!(removed, "{label} host remained visible after an immediate status reconciliation");
    }

    let session_id = "normal-session-end";
    let mut child = spawn_synthetic_host(&fixture, session_id);
    assert_strong_session(&fixture, session_id);
    let ended = run_with_stdin(
        fixture.command(),
        &["hook", "codex"],
        json!({
            "hook_event_name": "SessionEnd",
            "session_id": session_id,
            "cwd": fixture.root,
        })
        .to_string()
        .as_bytes(),
    );
    ended.assert().success();
    ended.assert().stdout(predicate::str::is_empty());
    assert!(wait_for_session_absence(&fixture, session_id, Duration::from_secs(1)));
    let _ = child.kill();
    let _ = child.wait();
}

fn run_with_stdin(mut command: Command, arguments: &[&str], input: &[u8]) -> Output {
    let mut child = command
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn command");
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().expect("command output")
}

fn unused_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port()
}

fn request_when_ready(port: u16, target: &str, timeout: Duration) -> String {
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(mut connection) = TcpStream::connect(("127.0.0.1", port)) {
            write!(connection, "GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").unwrap();
            let mut response = String::new();
            connection.read_to_string(&mut response).unwrap();
            return response;
        }
        assert!(Instant::now() < deadline, "server did not start before timeout");
        thread::sleep(Duration::from_millis(20));
    }
}

fn send_signal(child: &Child, signal: libc::c_int) {
    // SAFETY: the child PID is live and `kill` has no pointer preconditions.
    let result = unsafe { libc::kill(child.id() as libc::pid_t, signal) };
    assert_eq!(result, 0, "signal child: {}", std::io::Error::last_os_error());
}

fn install_hash_object_logger(fixture: &Fixture) -> (String, PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let bin = fixture._temporary.path().join("git-wrapper-bin");
    let log = fixture._temporary.path().join("hash-object.log");
    fs::create_dir(&bin).unwrap();
    let git = bin.join("git");
    fs::write(
        &git,
        "#!/bin/sh\nfor argument in \"$@\"; do\n  if [ \"$argument\" = \"hash-object\" ]; then\n    printf '%s\\n' hash-object >> \"$AI_COORD_HASH_OBJECT_LOG\"\n    break\n  fi\ndone\nexec /usr/bin/git \"$@\"\n",
    )
    .unwrap();
    fs::set_permissions(&git, fs::Permissions::from_mode(0o700)).unwrap();
    (format!("{}:/usr/bin:/bin", bin.display()), log)
}

fn hash_object_invocations(log: &Path) -> usize {
    fs::read_to_string(log).unwrap_or_default().lines().count()
}

fn commit_paths(fixture: &Fixture, paths: &[&str]) {
    let added =
        Command::new("/usr/bin/git").arg("add").arg("--").args(paths).current_dir(&fixture.root).output().unwrap();
    assert!(added.status.success(), "git add: {}", String::from_utf8_lossy(&added.stderr));
    let committed = Command::new("/usr/bin/git")
        .args(["-c", "user.name=ai-coord test", "-c", "user.email=test@invalid", "commit", "--quiet", "-m", "fixture"])
        .current_dir(&fixture.root)
        .output()
        .unwrap();
    assert!(committed.status.success(), "git commit: {}", String::from_utf8_lossy(&committed.stderr));
}

fn git_status(fixture: &Fixture) -> String {
    let output = Command::new("/usr/bin/git").args(["status", "--short"]).current_dir(&fixture.root).output().unwrap();
    assert!(output.status.success(), "git status: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

fn spawn_synthetic_host(fixture: &Fixture, session_id: &str) -> Child {
    let mut host = fixture.bash_command();
    host.env("AI_COORD_TEST_BIN", BINARY)
        .args([
            "-c",
            "exec -a codex /bin/bash -c 'trap \"exit 130\" INT; \"$AI_COORD_TEST_BIN\" hook codex; while :; do sleep 1; done'",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = host.spawn().expect("start synthetic Codex host");
    let payload = json!({
        "hook_event_name": "SessionStart",
        "session_id": session_id,
        "cwd": fixture.root,
    });
    child.stdin.take().unwrap().write_all(payload.to_string().as_bytes()).unwrap();
    child
}

fn assert_strong_session(fixture: &Fixture, session_id: &str) {
    let live = wait_for_status(fixture, Duration::from_secs(5), |snapshot| {
        snapshot["sessions"].as_array().is_some_and(|sessions| {
            sessions.iter().any(|session| session["session_id"] == session_id && session["pid"].is_u64())
        })
    });
    assert!(live, "synthetic host session {session_id} never acquired a strong process fingerprint");
}

fn wait_for_session_absence(fixture: &Fixture, session_id: &str, timeout: Duration) -> bool {
    wait_for_status(fixture, timeout, |snapshot| {
        snapshot["sessions"]
            .as_array()
            .is_some_and(|sessions| sessions.iter().all(|session| session["session_id"] != session_id))
    })
}

fn wait_for_exit(child: &mut Child, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait().expect("query child").is_some() {
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("child did not exit after signal");
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn wait_for_status(fixture: &Fixture, timeout: Duration, predicate: impl Fn(&Value) -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        let (_, snapshot) = fixture.json_status();
        if predicate(&snapshot) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn work_item(fixture: &Fixture, session_id: &str) -> Option<Value> {
    fixture.json_status().1.get("work")?.as_array()?.iter().find(|work| work["session_id"] == session_id).cloned()
}

fn work_state(fixture: &Fixture, session_id: &str) -> Option<String> {
    work_item(fixture, session_id)?.get("state")?.as_str().map(str::to_owned)
}

#![cfg(unix)]

use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use rusqlite::Connection;
use serde_json::{Value, json};
use tempfile::TempDir;

const BINARY: &str = env!("CARGO_BIN_EXE_ai-coord");

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
    assert_eq!(help.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&help.stdout).contains("Coordinate parallel Codex and Claude Code agents"));

    let parser_error = fixture.output(&["wait", "--timeout-seconds", "0"]);
    assert_eq!(parser_error.status.code(), Some(2));
    assert!(parser_error.stdout.is_empty());
    assert!(String::from_utf8_lossy(&parser_error.stderr).starts_with("error: invalid value '0'"));

    let semantic_error = fixture.output(&["finding", "add", "   "]);
    assert_eq!(semantic_error.status.code(), Some(64));
    assert!(semantic_error.stdout.is_empty());
    assert_eq!(String::from_utf8_lossy(&semantic_error.stderr), "error: finding summary must contain text\n");

    let removed_note = fixture.output(&["note", "old"]);
    assert_eq!(removed_note.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&removed_note.stderr).contains("unrecognized subcommand 'note'"));

    let conflicting_inbox = fixture.output(&["inbox", "--ack", "abc", "--ack-all"]);
    assert_eq!(conflicting_inbox.status.code(), Some(64));
    assert_eq!(String::from_utf8_lossy(&conflicting_inbox.stderr), "error: use only one of --ack or --ack-all\n");
}

#[test]
fn identity_commands_and_state_are_fully_isolated() {
    let fixture = Fixture::new();

    let named = fixture.output(&["name", "🦀 Ferris Test"]);
    assert_eq!(named.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&named.stdout), "NAMED\t🦀 Ferris Test\n");

    let trailer = fixture.output(&["trailer"]);
    assert_eq!(trailer.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&trailer.stdout), "Agent-Session: codex/cli-test\n");

    let finding = fixture.output(&["finding", "add", "--kind", "bug", "--path", "src/lib.rs", "integration finding"]);
    assert_eq!(finding.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&finding.stdout).starts_with("ADDED\t"));

    let (code, status) = fixture.json_status();
    assert!(
        matches!(code, 0 | 2),
        "status is complete under a detectable Codex ancestor and partial when the test host is unknown"
    );
    assert_eq!(status["schema_version"], 4);
    assert_eq!(status["scope"]["kind"], "machine");
    assert_eq!(status["sessions"][0]["callsign"], "🦀 Ferris Test");
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
    assert_eq!(malformed.status.code(), Some(0));
    assert!(malformed.stdout.is_empty());
    assert!(malformed.stderr.is_empty());

    let stop =
        run_with_stdin(fixture.command(), &["hook", "codex"], br#"{"hook_event_name":"Stop","private":"do not leak"}"#);
    assert_eq!(stop.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&stop.stdout), "{}\n");
    assert!(!String::from_utf8_lossy(&stop.stdout).contains("do not leak"));
    assert!(stop.stderr.is_empty());

    let waker = run_with_stdin(fixture.command(), &["waker", "claude"], b"not json");
    assert_eq!(waker.status.code(), Some(0));
    assert!(waker.stdout.is_empty());
    assert!(waker.stderr.is_empty());
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
    assert_eq!(start.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&start.stdout), "READY\tsrc/app.rs\n");

    let wait = fixture.output_as("sender-host", &["wait", "-t", "1"]);
    assert_eq!(wait.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&wait.stdout), "READY\tsrc/app.rs\n");

    let baseline = fixture.output_as("sender-host", &["baseline"]);
    assert_eq!(baseline.status.code(), Some(0));
    assert!(baseline.stdout.is_empty());

    let sent = fixture.output_as("sender-host", &["msg", "recipient-host", "ready for review"]);
    assert_eq!(sent.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&sent.stdout).starts_with("SENT\t1\t"));

    let inbox = fixture.output_as("recipient-host", &["inbox"]);
    assert_eq!(inbox.status.code(), Some(0));
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
    assert_eq!(first.status.code(), Some(0));
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
    assert_eq!(missing_canonical.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&missing_canonical.stderr).contains("--canonical is required"));
    let marked_duplicate =
        fixture.output(&["finding", "resolve", &recurrence_id, "--as", "duplicate", "--canonical", &first_id]);
    assert_eq!(marked_duplicate.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&fixture.output(&["finding", "reopen", &first_id]).stdout),
        format!("REOPENED\t{first_id}\n")
    );

    let outside = fixture._temporary.path().join("outside.txt");
    fs::write(&outside, "outside\n").unwrap();
    std::os::unix::fs::symlink(&outside, fixture.root.join("outside-link")).unwrap();
    let escaped = fixture.output(&["finding", "add", "--path", "outside-link", "must reject escape"]);
    assert_eq!(escaped.status.code(), Some(64));
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
    assert_eq!(created.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&created.stdout), "DRAFT\t2\n");

    let (_, snapshot) = fixture.json_status();
    let draft = snapshot["work"].as_array().unwrap().iter().find(|work| work["session_id"] == "draft-host").unwrap();
    assert_eq!(draft["state"], "draft");
    assert_eq!(draft["scope_count"], 2);
    assert!(draft.get("scopes").is_none());
    assert!(!serde_json::to_string(draft).unwrap().contains("private.rs"));

    let replaced = fixture.output_as("draft-host", &["draft", "revised plan", "--recursive", "src"]);
    assert_eq!(String::from_utf8_lossy(&replaced.stdout), "DRAFT\t1\n");
    let bypass = fixture.output_as("draft-host", &["start", "drifted execution", "src/other.rs"]);
    assert_eq!(bypass.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bypass.stderr).contains("a draft exists"));

    let promoted = fixture.output_as("draft-host", &["start", "--draft"]);
    assert_eq!(promoted.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&promoted.stdout), "READY\tsrc\n");
    let (_, snapshot) = fixture.json_status();
    let active = snapshot["work"].as_array().unwrap().iter().find(|work| work["session_id"] == "draft-host").unwrap();
    assert_eq!(active["state"], "active");
    assert_eq!(active["scopes"], json!([{"path":"src", "kind":"recursive"}]));
    assert!(active.get("scope_count").is_none());

    let rejected = fixture.output_as("draft-host", &["draft", "must release", "src/new.rs"]);
    assert_eq!(rejected.status.code(), Some(1));
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
        assert_eq!(output.status.code(), Some(64));
        assert_eq!(String::from_utf8_lossy(&output.stderr), "error: at least one scope is required\n");
    }

    let conflict = fixture.output(&["start", "--draft", "label"]);
    assert_eq!(conflict.status.code(), Some(2));
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
        assert_eq!(output.status.code(), Some(64));
        assert!(output.stdout.is_empty());
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
    assert_eq!(invalid.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("recursive scope is not a directory: planned"));
    assert_eq!(work_state(&fixture, "revalidate-host"), Some("draft".to_owned()));

    fs::remove_file(fixture.root.join("planned")).unwrap();
    let other = fixture._temporary.path().join("other-repo");
    fs::create_dir(&other).unwrap();
    assert!(Command::new("git").args(["init", "--quiet"]).current_dir(&other).status().unwrap().success());
    let mismatch = fixture.output_as_in("revalidate-host", &other, &["start", "--draft"]);
    assert_eq!(mismatch.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&mismatch.stderr).contains("draft belongs to another repository"));
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
    assert_eq!(promoted.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&promoted.stdout), "UNKNOWN\tcoverage\n");
    assert_eq!(work_state(&fixture, "cli-test"), Some("queued".to_owned()));
    let work = work_item(&fixture, "cli-test").unwrap();
    assert!(work.get("scope_count").is_none());
    assert_eq!(work["scopes"], json!([{"path":"src/unknown.rs", "kind":"exact"}]));

    let waited = fixture.output_with_path(&["wait", "-t", "1"], std::path::Path::new(&executable_path));
    assert_eq!(waited.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&waited.stdout), "UNKNOWN\tcoverage\n");
    assert_eq!(String::from_utf8_lossy(&fixture.output(&["done"]).stdout), "DONE\treleased\n");
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
    assert_eq!(still_queued.status.code(), Some(3));

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
    assert_eq!(preview.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&preview.stdout),
        format!("WOULD_UPDATE\tclaude\t{}\ttrust=skipped\n", claude_settings.display())
    );
    assert!(!claude_settings.exists());

    let linked = fixture.output(&["link", "claude", "--path", &path]);
    assert_eq!(linked.status.code(), Some(0));
    assert!(claude_settings.is_file());
    assert!(String::from_utf8_lossy(&linked.stdout).starts_with("UPDATED\tclaude\t"));

    let repeated = fixture.output(&["link", "claude", "--path", &path]);
    assert_eq!(repeated.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&repeated.stdout).starts_with("OK\tclaude\t"));

    let malformed = fixture.claude_home.join("malformed.json");
    fs::write(&malformed, br#"{"hooks":[]}"#).unwrap();
    let rejected = fixture.output(&["link", "claude", "--path", &malformed.to_string_lossy()]);
    assert_eq!(rejected.status.code(), Some(64));
    assert_eq!(
        String::from_utf8_lossy(&rejected.stderr),
        "error: hooks field must be an object; pass --force to replace it\n"
    );

    let check = fixture.output(&["check", "--json"]);
    assert_eq!(check.status.code(), Some(2));
    let reports: Vec<Value> = serde_json::from_slice(&check.stdout).expect("check JSON");
    let state = reports.iter().find(|report| report["component"] == "state").expect("state report");
    assert_eq!(state["schema_version"], 12);
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
    assert_eq!(output.status.code(), Some(0));
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
    assert_eq!(ended.status.code(), Some(0));
    assert!(ended.stdout.is_empty());
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

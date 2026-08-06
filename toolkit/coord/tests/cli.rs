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

    let semantic_error = fixture.output(&["note"]);
    assert_eq!(semantic_error.status.code(), Some(64));
    assert!(semantic_error.stdout.is_empty());
    assert_eq!(String::from_utf8_lossy(&semantic_error.stderr), "error: provide note text or --done ID\n");

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

    let note = fixture.output(&["note", "integration finding"]);
    assert_eq!(note.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&note.stdout).starts_with("NOTE\t"));

    let (code, status) = fixture.json_status();
    assert!(
        matches!(code, 0 | 2),
        "status is complete under a detectable Codex ancestor and partial when the test host is unknown"
    );
    assert_eq!(status["schema_version"], 1);
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

    let note = fixture.output_as("sender-host", &["note", "durable finding"]);
    let note_id = String::from_utf8_lossy(&note.stdout).trim().strip_prefix("NOTE\t").unwrap().to_owned();
    let resolved = fixture.output_as("sender-host", &["note", "--done", &note_id]);
    assert_eq!(String::from_utf8_lossy(&resolved.stdout), format!("DONE\t{note_id}\n"));

    let done = fixture.output_as("sender-host", &["done"]);
    assert_eq!(String::from_utf8_lossy(&done.stdout), "DONE\treleased\n");
    let repeated = fixture.output_as("sender-host", &["done"]);
    assert_eq!(String::from_utf8_lossy(&repeated.stdout), "DONE\talready clear\n");
    let intent = fixture.output_as("sender-host", &["start", "planning only"]);
    assert_eq!(String::from_utf8_lossy(&intent.stdout), "INTENT\tplanning only\n");

    let _ = sender.kill();
    let _ = sender.wait();
    let _ = recipient.kill();
    let _ = recipient.wait();
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
    assert_eq!(state["schema_version"], 9);
    assert_eq!(state["path"], fixture.state.join("state.db").to_string_lossy().as_ref());
    let codex_hooks = reports.iter().find(|report| report["component"] == "hooks:codex").expect("hook report");
    assert!(codex_hooks["error"].is_null());
    assert_eq!(codex_hooks["missing"].as_array().map(Vec::len), Some(7));
    assert!(reports.iter().any(|report| report["component"] == "hooks-trust:codex"));
}

#[test]
fn dashboard_snapshot_matches_the_frontend_shape_and_ctrl_c_is_graceful() {
    let fixture = Fixture::new();
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
        "claims",
        "notes",
        "delegates",
        "outside_scope",
        "messages",
        "generated_at",
        "generation",
    ] {
        assert!(payload.get(key).is_some(), "missing dashboard field {key}");
    }

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

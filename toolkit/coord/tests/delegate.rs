#![cfg(unix)]

//! Integration coverage for the delegate lifecycle guard: lifecycle commands
//! (e.g. `start`) reject callers whose environment looks like a delegate of
//! another session's identity, while read-only commands stay unaffected.

use std::{
    fs,
    io::Write,
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
        fs::create_dir_all(&root).expect("repository directory");
        let init = Command::new("git").args(["init", "--quiet"]).current_dir(&root).output().expect("git init");
        assert!(init.status.success(), "git init: {}", String::from_utf8_lossy(&init.stderr));
        for (key, value) in [("user.name", "ai-coord delegate test"), ("user.email", "delegate-test@invalid")] {
            let config =
                Command::new("git").args(["config", key, value]).current_dir(&root).output().expect("git config");
            assert!(config.status.success(), "git config {key}: {}", String::from_utf8_lossy(&config.stderr));
        }
        fs::write(root.join("README.md"), "delegate guard fixture\n").expect("write README.md");
        let add = Command::new("git").args(["add", "README.md"]).current_dir(&root).output().expect("git add");
        assert!(add.status.success(), "git add: {}", String::from_utf8_lossy(&add.stderr));
        let commit = Command::new("git")
            .args(["commit", "--quiet", "-m", "fixture"])
            .current_dir(&root)
            .output()
            .expect("git commit");
        assert!(commit.status.success(), "git commit: {}", String::from_utf8_lossy(&commit.stderr));
        Self { _temporary: temporary, root, state, codex_home, claude_home, home }
    }

    /// Base command with a clean coordination environment: no override, no
    /// host identity variables. Callers layer in exactly the variables their
    /// scenario needs.
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
            .env("CODEX_HOME", &self.codex_home)
            .env("CLAUDE_CONFIG_DIR", &self.claude_home)
            .env("HOME", &self.home)
            .env("PATH", "/usr/bin:/bin")
            .env_remove("AI_COORD_CLIENT")
            .env_remove("AI_COORD_SESSION_ID")
            .env_remove("CODEX_SESSION_ID")
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CLAUDE_CODE_SESSION_ID");
    }

    fn output(&self, arguments: &[&str]) -> Output {
        self.command().args(arguments).output().expect("run ai-coord")
    }

    fn json_status(&self) -> Value {
        let output = self.output(&["status", "--all", "--json"]);
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "status JSON: {error}; stderr={} stdout={}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            )
        })
    }
}

/// Spawn a real long-lived process registered under `session_id` via a
/// synthetic Codex `SessionStart` hook, so provider coverage treats that
/// session as a live host with a real process fingerprint. Mirrors the
/// pattern in `tests/cli.rs` (not imported from it, per crate write scope).
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
        "transcript_path": format!("opaque:{session_id}"),
    });
    child.stdin.take().unwrap().write_all(payload.to_string().as_bytes()).unwrap();
    child
}

fn wait_for_status(fixture: &Fixture, timeout: Duration, predicate: impl Fn(&Value) -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        let snapshot = fixture.json_status();
        if predicate(&snapshot) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn assert_strong_session(fixture: &Fixture, session_id: &str) {
    let live = wait_for_status(fixture, Duration::from_secs(5), |snapshot| {
        snapshot["sessions"].as_array().is_some_and(|sessions| {
            sessions.iter().any(|session| session["session_id"] == session_id && session["pid"].is_u64())
        })
    });
    assert!(live, "synthetic host session {session_id} never acquired a strong process fingerprint");
}

#[test]
fn rule_a_override_with_differing_host_identity_rejects_lifecycle_command() {
    let fixture = Fixture::new();

    let start = fixture
        .command()
        .env("AI_COORD_CLIENT", "codex")
        .env("AI_COORD_SESSION_ID", "parent")
        .env("CODEX_SESSION_ID", "child")
        .args(["start", "x", "README.md"])
        .output()
        .expect("run ai-coord start");

    assert_eq!(start.status.code(), Some(64), "stderr={}", String::from_utf8_lossy(&start.stderr));
    assert!(String::from_utf8_lossy(&start.stderr).contains("delegate of codex/parent"));
}

#[test]
fn rule_a_override_with_differing_host_identity_leaves_read_only_commands_unaffected() {
    let fixture = Fixture::new();

    let status = fixture
        .command()
        .env("AI_COORD_CLIENT", "codex")
        .env("AI_COORD_SESSION_ID", "parent")
        .env("CODEX_SESSION_ID", "child")
        .args(["status"])
        .output()
        .expect("run ai-coord status");
    let code = status.status.code().expect("status exit code");
    assert!(matches!(code, 0 | 2), "status exit code: {code}; stderr={}", String::from_utf8_lossy(&status.stderr));
}

#[test]
fn override_matching_host_identity_stays_ready() {
    let fixture = Fixture::new();
    let mut host = spawn_synthetic_host(&fixture, "parent");
    assert_strong_session(&fixture, "parent");

    let start = fixture
        .command()
        .env("AI_COORD_CLIENT", "codex")
        .env("AI_COORD_SESSION_ID", "parent")
        .env("CODEX_SESSION_ID", "parent")
        .args(["start", "x", "README.md"])
        .output()
        .expect("run ai-coord start");

    assert_eq!(start.status.code(), Some(0), "stderr={}", String::from_utf8_lossy(&start.stderr));
    assert_eq!(String::from_utf8_lossy(&start.stdout), "READY\tREADME.md\n");

    let _ = host.kill();
    let _ = host.wait();
}

#[test]
fn rule_b_unconfirmed_without_active_delegate_row_stays_ready() {
    let fixture = Fixture::new();
    let mut host = spawn_synthetic_host(&fixture, "root");
    assert_strong_session(&fixture, "root");

    let start = fixture
        .command()
        .env("CODEX_SESSION_ID", "root")
        .env("CODEX_THREAD_ID", "child")
        .args(["start", "x", "README.md"])
        .output()
        .expect("run ai-coord start");

    assert_eq!(start.status.code(), Some(0), "stderr={}", String::from_utf8_lossy(&start.stderr));
    assert_eq!(String::from_utf8_lossy(&start.stdout), "READY\tREADME.md\n");

    let _ = host.kill();
    let _ = host.wait();
}

/// The guard sits only on `lifecycle_identity()`, which the eight
/// lifecycle-claim commands use; every other `required_identity()` caller —
/// including `touched` (explicitly permitted for delegates by the
/// codex-handoff skill) and `inbox` (subagents legitimately read the
/// parent's inbox) — stays unguarded even under a rule (a) delegate
/// environment, while `start` is still rejected in the same environment.
#[test]
fn rule_a_delegate_environment_still_allows_touched_and_inbox_but_rejects_start() {
    let fixture = Fixture::new();
    let delegate_env = [("AI_COORD_CLIENT", "codex"), ("AI_COORD_SESSION_ID", "parent"), ("CODEX_SESSION_ID", "child")];

    let touched = fixture.command().envs(delegate_env).args(["touched"]).output().expect("run ai-coord touched");
    assert_eq!(touched.status.code(), Some(0), "stderr={}", String::from_utf8_lossy(&touched.stderr));

    let inbox = fixture.command().envs(delegate_env).args(["inbox"]).output().expect("run ai-coord inbox");
    assert_eq!(inbox.status.code(), Some(0), "stderr={}", String::from_utf8_lossy(&inbox.stderr));

    let start =
        fixture.command().envs(delegate_env).args(["start", "x", "README.md"]).output().expect("run ai-coord start");
    assert_eq!(start.status.code(), Some(64), "stderr={}", String::from_utf8_lossy(&start.stderr));
    assert!(String::from_utf8_lossy(&start.stderr).contains("delegate of codex/parent"));
}

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use assert_cmd::{Command, assert::Assert};
use predicates::prelude::*;
use rusqlite::Connection;
use tempfile::TempDir;

struct TestEnv {
    _root: TempDir,
    home: PathBuf,
    xdg: PathBuf,
    project: PathBuf,
    bin: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let xdg = root.path().join("xdg");
        let project = root.path().join("project");
        let bin = root.path().join("bin");
        for path in [&home, &xdg, &project, &bin] {
            fs::create_dir_all(path).unwrap();
        }
        Self { _root: root, home, xdg, project, bin }
    }

    fn config_path(&self) -> PathBuf {
        self.xdg.join("ai-notify/config.yaml")
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ai-notify"));
        command
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.xdg)
            .env("PATH", &self.bin)
            .env("AI_NOTIFY_LOG", "0")
            .current_dir(&self.project);
        command
    }

    fn run(&self, arguments: &[&str], input: &str) -> Assert {
        let mut command = self.command();
        command.args(arguments).write_stdin(input);
        command.assert()
    }

    fn write_runtime_config(&self, database: &Path, log: &Path) {
        let path = self.config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            format!(
                "cleanup:\n  auto_cleanup_enabled: false\n  export_before_cleanup: false\n\
                 database:\n  path: {}\n\
                 logging:\n  level: DEBUG\n  path: {}\n\
                 notification:\n  threshold_seconds: 0\n",
                database.display(),
                log.display()
            ),
        )
        .unwrap();
    }
}

#[test]
fn version_and_usage_exit_codes_are_stable() {
    let environment = TestEnv::new();
    environment.run(&["--version"], "").success().stdout(predicate::str::contains("ai-notify 1.0.0"));

    environment.run(&["codex"], "").code(2).stderr(predicate::str::contains("Missing JSON payload"));

    environment.run(&["cleanup", "--days", "0", "--dry-run"], "").code(2);
}

#[test]
fn config_show_edit_and_reset_are_isolated_and_confirmed() {
    let environment = TestEnv::new();
    let config = environment._root.path().join("custom/config.yaml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(&config, "notification:\n  threshold_seconds: 17\n").unwrap();

    environment.run(&["config", "show", "--path", config.to_str().unwrap()], "").success().stdout(
        predicate::str::contains("Current Configuration").and(predicate::str::contains("Notification Threshold: 17s")),
    );

    environment
        .run(&["config", "show", "--path", environment._root.path().join("missing.yaml").to_str().unwrap()], "")
        .code(2);

    let editor = environment.bin.join("fake-editor");
    fs::write(&editor, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&editor, fs::Permissions::from_mode(0o755)).unwrap();
    let edit_target = environment._root.path().join("edited/config.yaml");
    let mut edit = environment.command();
    edit.env("EDITOR", &editor).args(["config", "edit", "--path", edit_target.to_str().unwrap()]);
    edit.assert().success().stdout(predicate::str::contains("Configuration is valid"));
    assert!(edit_target.exists());

    environment.run(&["config", "reset", "--path", config.to_str().unwrap()], "n\n").code(1);
    assert!(fs::read_to_string(&config).unwrap().contains("17"));

    environment
        .run(&["config", "reset", "--path", config.to_str().unwrap()], "y\n")
        .success()
        .stdout(predicate::str::contains("reset to defaults"));
    assert!(fs::read_to_string(config).unwrap().contains("threshold_seconds: 10"));
}

#[test]
fn test_notification_succeeds_when_notifier_is_unavailable() {
    let environment = TestEnv::new();
    environment
        .run(&["test"], "")
        .success()
        .stdout(predicate::str::contains("Test notification").and(predicate::str::contains("unavailable")));
}

#[test]
fn codex_accepts_argument_and_stdin_payloads_and_rejects_bad_json_as_usage() {
    let environment = TestEnv::new();
    let payload = r#"{"type":"agent-turn-complete","input-messages":["hello"],"last-assistant-message":"done"}"#;

    environment.run(&["codex", payload], "").success();
    environment.run(&["codex", "--stdin"], payload).success();

    environment.run(&["codex", "{"], "").code(2).stderr(predicate::str::contains("Failed to parse JSON"));
}

#[test]
fn link_claude_supports_dry_run_updates_and_schema_failures() {
    let environment = TestEnv::new();
    let settings = environment.home.join(".claude/settings.json");

    environment
        .run(&["link", "claude", "--path", settings.to_str().unwrap(), "--dry-run"], "")
        .success()
        .stdout(predicate::str::contains("Would update hooks"));
    assert!(!settings.exists());

    environment.run(&["link", "claude", "--path", settings.to_str().unwrap()], "").success();
    let contents = fs::read_to_string(&settings).unwrap();
    assert!(contents.contains("ai-notify event stop"));
    assert!(contents.contains("AskUserQuestion"));

    let malformed = environment.home.join(".claude/bad.json");
    fs::write(&malformed, r#"{"hooks":[]}"#).unwrap();
    environment
        .run(&["link", "claude", "--path", malformed.to_str().unwrap()], "")
        .code(1)
        .stderr(predicate::str::contains("hooks field must be an object"));
}

#[test]
fn link_codex_preserves_conflicts_and_supports_force_and_profiles() {
    let environment = TestEnv::new();
    let config = environment.home.join(".codex/config.toml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(&config, "# keep\nnotify = [\"other\"]\n").unwrap();

    environment
        .run(&["link", "codex", "--path", config.to_str().unwrap()], "")
        .code(1)
        .stdout(predicate::str::contains("Refusing to replace"));
    assert!(fs::read_to_string(&config).unwrap().contains("other"));

    environment
        .run(&["link", "codex", "--path", config.to_str().unwrap(), "--force"], "")
        .success()
        .stdout(predicate::str::contains("Replaced previous notify"));
    assert!(fs::read_to_string(&config).unwrap().contains("[\"ai-notify\", \"codex\"]"));

    environment.run(&["link", "codex", "--path", config.to_str().unwrap(), "--profile", "quiet"], "").success();
    assert!(config.with_file_name("quiet.config.toml").exists());
}

#[test]
fn check_treats_missing_as_diagnostic_and_parse_errors_as_failures() {
    let environment = TestEnv::new();
    environment.run(&["check"], "").success().stdout(
        predicate::str::contains("Claude Code hooks: MISSING")
            .and(predicate::str::contains("Codex CLI notify: MISSING")),
    );

    let settings = environment.home.join(".claude/settings.json");
    fs::create_dir_all(settings.parent().unwrap()).unwrap();
    fs::write(&settings, "{").unwrap();
    environment.run(&["check"], "").code(1).stdout(predicate::str::contains("Errors:"));

    let second = TestEnv::new();
    let codex = second.home.join(".codex/config.toml");
    fs::create_dir_all(codex.parent().unwrap()).unwrap();
    fs::write(codex, "notify = [\"broken\"\n").unwrap();
    second.run(&["check"], "").code(1).stdout(predicate::str::contains("Codex CLI notify: ERROR"));
}

#[test]
fn cleanup_dry_run_cancellation_and_deletion_use_configured_database() {
    let environment = TestEnv::new();
    let database = environment._root.path().join("state/sessions.db");
    let log = environment._root.path().join("logs/ai-notify.log");
    environment.write_runtime_config(&database, &log);

    let prompt = r#"{"session_id":"cleanup-session","prompt":"old work","cwd":"/tmp/project"}"#;
    environment.run(&["event", "user-prompt-submit"], prompt).success();
    let connection = Connection::open(&database).unwrap();
    connection.execute("UPDATE sessions SET created_at = datetime('now', '-40 days')", []).unwrap();
    drop(connection);

    environment
        .run(&["cleanup", "--days", "30", "--dry-run"], "")
        .success()
        .stdout(predicate::str::contains("Sessions to delete: 1"));

    environment
        .run(&["cleanup", "--days", "30", "--no-export"], "n\n")
        .success()
        .stdout(predicate::str::contains("Cleanup cancelled"));

    environment
        .run(&["cleanup", "--days", "30", "--no-export"], "y\n")
        .success()
        .stdout(predicate::str::contains("Sessions deleted: 1"));
    let remaining: i64 =
        Connection::open(database).unwrap().query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0)).unwrap();
    assert_eq!(remaining, 0);
}

#[test]
fn every_claude_event_path_runs_with_custom_database_and_log_paths() {
    let environment = TestEnv::new();
    let database = environment._root.path().join("custom/state.db");
    let log = environment._root.path().join("custom/logs/notify.log");
    environment.write_runtime_config(&database, &log);

    let mut logged_prompt = environment.command();
    logged_prompt
        .env("AI_NOTIFY_LOG", "1")
        .args(["event", "user-prompt-submit"])
        .write_stdin(br#"{"session_id":"s1","prompt":"do work","cwd":"/tmp/project"}"#);
    logged_prompt.assert().success();
    assert!(database.exists());
    assert!(log.exists());

    let cases = [
        ("stop", r#"{"session_id":"s1","cwd":"/tmp/project","last_assistant_message":"done"}"#),
        ("user-prompt-submit", r#"{"session_id":"s2","prompt":"retry","cwd":"/tmp/project"}"#),
        ("stop-failure", r#"{"session_id":"s2","cwd":"/tmp/project","error":"rate limit"}"#),
        ("notification", r#"{"session_id":"s1","notification_type":"idle_prompt","message":"waiting for input"}"#),
        ("permission-request", r#"{"cwd":"/tmp/project","tool_name":"Bash","tool_input":{"command":"echo ok"}}"#),
        ("ask-user-question", r#"{"cwd":"/tmp/project","tool_input":{"questions":[{"question":"Continue?"}]}}"#),
    ];
    for (event, payload) in cases {
        environment.run(&["event", event], payload).success();
    }

    environment.run(&["event", "stop"], "not json").code(2);
}

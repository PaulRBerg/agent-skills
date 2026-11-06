#![cfg(unix)]

use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

use serde_json::json;
use tempfile::TempDir;

const BINARY: &str = env!("CARGO_BIN_EXE_ai-coord");

struct Fixture {
    temporary: TempDir,
    root: PathBuf,
    log: PathBuf,
    bin: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("repo");
        let log = temporary.path().join("git.log");
        let bin = temporary.path().join("bin");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&bin).unwrap();
        let git = bin.join("git");
        fs::write(
            &git,
            r#"#!/bin/sh
printf '%s\0' "$@" >> "$AI_COORD_GIT_LOG"
printf '\n' >> "$AI_COORD_GIT_LOG"
if [ -f "$AI_COORD_FAIL_BATCH_ONCE" ]; then
    for argument in "$@"; do
        if [ "$argument" = hash-object ]; then
            rm "$AI_COORD_FAIL_BATCH_ONCE"
            exit 1
        fi
    done
fi
exec /usr/bin/git "$@"
"#,
        )
        .unwrap();
        fs::set_permissions(git, fs::Permissions::from_mode(0o700)).unwrap();
        let fixture = Self { temporary, root, log, bin };
        fixture.git(&["init", "--quiet"]);
        fixture
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("/usr/bin/git").args(args).current_dir(&self.root).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }

    fn command(&self) -> Command {
        let mut command = Command::new(BINARY);
        command
            .current_dir(&self.root)
            .env("AI_COORD_STATE_DIR", self.temporary.path().join("state"))
            .env("AI_COORD_CLIENT", "codex")
            .env("AI_COORD_SESSION_ID", "git-performance")
            .env("AI_COORD_GIT_LOG", &self.log)
            .env("CODEX_HOME", self.temporary.path().join("codex"))
            .env("CLAUDE_CONFIG_DIR", self.temporary.path().join("claude"))
            .env("HOME", self.temporary.path().join("home"))
            .env("PATH", format!("{}:/usr/bin:/bin", self.bin.display()))
            .env_remove("CODEX_SESSION_ID")
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CLAUDE_CODE_SESSION_ID");
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        let output = self.command().args(args).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        output
    }

    fn calls(&self, subcommand: &str) -> Vec<Vec<String>> {
        fs::read_to_string(&self.log)
            .unwrap()
            .lines()
            .map(|line| line.split_terminator('\0').map(str::to_owned).collect::<Vec<_>>())
            .filter(|args| args.iter().any(|arg| arg == subcommand))
            .collect()
    }
}

#[test]
fn nested_repository_does_not_amplify_blob_hash_batches() {
    let fixture = Fixture::new();
    for index in 0..260 {
        fs::write(fixture.root.join(format!("file-{index:03}")), vec![b'x'; 64 * 1024]).unwrap();
    }
    fs::create_dir(fixture.root.join("nested")).unwrap();
    fixture.git(&["-C", "nested", "init", "--quiet"]);

    fixture.run(&["status", "--json"]);

    let calls = fixture.calls("hash-object");
    assert_eq!(calls.len(), 3, "260 files must use three batches: {calls:?}");
    assert!(calls.iter().all(|args| !args.iter().any(|arg| arg == "nested/")));
    assert!(fixture.calls("status").iter().all(|args| args.iter().any(|arg| arg == "--no-optional-locks")));
}

#[test]
fn failed_blob_batch_still_falls_back_for_every_path() {
    let fixture = Fixture::new();
    for name in ["first", "middle", "last"] {
        fs::write(fixture.root.join(name), name).unwrap();
    }
    let fail_once = fixture.temporary.path().join("fail-batch");
    fs::write(&fail_once, "").unwrap();
    let output =
        fixture.command().env("AI_COORD_FAIL_BATCH_ONCE", &fail_once).args(["status", "--json"]).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let calls = fixture.calls("hash-object");
    assert_eq!(calls.len(), 4, "one failed batch plus three individual retries: {calls:?}");
    for name in ["first", "middle", "last"] {
        assert_eq!(calls[1..].iter().filter(|args| args.last().is_some_and(|arg| arg == name)).count(), 1);
    }
}

#[test]
fn post_tool_uses_one_scoped_scan_for_claim_and_outside_write() {
    let fixture = Fixture::new();
    fs::write(fixture.root.join("owned.txt"), "clean\n").unwrap();
    fixture.git(&["add", "owned.txt"]);
    fixture.git(&["-c", "user.name=ai-coord test", "-c", "user.email=test@invalid", "commit", "-qm", "fixture"]);
    fixture.run(&["start", "hook work", "owned.txt"]);
    fs::create_dir_all(fixture.root.join("app/[slug]")).unwrap();
    fs::write(fixture.root.join("app/[slug]/page.tsx"), "outside\n").unwrap();
    fs::create_dir(fixture.root.join("unrelated")).unwrap();
    fs::write(fixture.root.join("unrelated/file"), "unrelated\n").unwrap();
    fs::write(&fixture.log, "").unwrap();

    let payload = json!({
        "session_id": "git-performance", "cwd": fixture.root, "hook_event_name": "PostToolUse",
        "tool_name": "Write", "tool_input": {"file_path": "app/[slug]/page.tsx"}
    });
    let mut child =
        fixture.command().args(["hook", "codex"]).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(payload.to_string().as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("outside your claim"));

    let calls = fixture.calls("status");
    assert_eq!(calls.len(), 1, "post-tool checks must share one status call: {calls:?}");
    let args = &calls[0];
    assert!(args.iter().any(|arg| arg == "--no-optional-locks"));
    assert!(args.iter().any(|arg| arg == "--literal-pathspecs"));
    let separator = args.iter().position(|arg| arg == "--").unwrap();
    assert_eq!(&args[separator + 1..], ["app/[slug]/page.tsx", "owned.txt"]);
}

mod common;

use std::{fs, process::Command};

use common::{Harness, exit_code, git_at, stderr, stdout, write_executable};

#[test]
fn standalone_push_handles_existing_upstream_and_clean_ahead() {
    let harness = Harness::new("push-upstream");
    let remote = harness.root.join("remote.git");
    init_bare(&remote, &harness.home);
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.git(["branch", "-M", "main"]);
    harness.git(["remote", "add", "origin", &format!("file://{}", remote.display())]);
    harness.git(["push", "--quiet", "-u", "origin", "HEAD"]);
    harness.write("intended.txt", "ahead\n");
    harness.commit_all("ahead");

    let pushed = harness.success(["push"]);
    assert_eq!(stdout(&pushed), "PUSHED main\n");
    assert_eq!(harness.git(["rev-parse", "HEAD"]), git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]));
    let idempotent = harness.success(["push"]);
    assert_eq!(stdout(&idempotent), "PUSHED main\n");
}

#[test]
fn new_remote_branch_is_created_and_upstream_is_set() {
    let harness = Harness::new("push-new");
    let remote = harness.root.join("remote.git");
    init_bare(&remote, &harness.home);
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.git(["branch", "-M", "feature-new"]);
    harness.git(["remote", "add", "origin", &format!("file://{}", remote.display())]);

    let pushed = harness.success(["push"]);
    assert_eq!(stdout(&pushed), "PUSHED_NEW feature-new\n");
    assert_eq!(harness.git(["rev-parse", "--abbrev-ref", "@{upstream}"]), "origin/feature-new");
    assert_eq!(
        harness.git(["rev-parse", "HEAD"]),
        git_at(&remote, &harness.home, ["rev-parse", "refs/heads/feature-new"])
    );
}

#[test]
fn existing_remote_branch_without_upstream_is_not_reported_as_new() {
    let harness = Harness::new("push-existing-no-upstream");
    let remote = harness.root.join("remote.git");
    init_bare(&remote, &harness.home);
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.git(["branch", "-M", "main"]);
    harness.git(["remote", "add", "origin", &format!("file://{}", remote.display())]);
    harness.git(["push", "--quiet", "origin", "HEAD:refs/heads/main"]);
    harness.write("intended.txt", "ahead\n");
    harness.commit_all("ahead");

    let pushed = harness.success(["push"]);
    assert_eq!(stdout(&pushed), "PUSHED main\n");
    assert_eq!(harness.git(["rev-parse", "--abbrev-ref", "@{upstream}"]), "origin/main");
}

#[test]
fn behind_branch_is_a_safe_noncompletion() {
    let harness = Harness::new("push-behind");
    let remote = harness.root.join("remote.git");
    let updater = harness.root.join("updater");
    init_bare(&remote, &harness.home);
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.git(["branch", "-M", "main"]);
    harness.git(["remote", "add", "origin", &format!("file://{}", remote.display())]);
    harness.git(["push", "--quiet", "-u", "origin", "HEAD"]);
    clone_repository(&remote, &updater, &harness.home);
    fs::write(updater.join("remote.txt"), "remote\n").unwrap();
    git_at(&updater, &harness.home, ["add", "remote.txt"]);
    git_at(&updater, &harness.home, ["commit", "--quiet", "-m", "remote"]);
    git_at(&updater, &harness.home, ["push", "--quiet"]);
    let remote_before = git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]);

    let behind = harness.command(["push"]);
    assert_eq!(exit_code(&behind), 3);
    assert_eq!(stdout(&behind), "BEHIND main 1\n");
    assert_eq!(git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]), remote_before);
}

#[test]
fn recognized_non_fast_forward_is_fetched_and_retried_once() {
    let harness = Harness::new("push-retry");
    let remote = harness.root.join("remote.git");
    init_bare(&remote, &harness.home);
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.git(["branch", "-M", "main"]);
    harness.git(["remote", "add", "origin", &format!("file://{}", remote.display())]);
    harness.git(["push", "--quiet", "-u", "origin", "HEAD"]);
    harness.write("intended.txt", "retry\n");
    harness.commit_all("retry");

    let attempts = harness.root.join("push-attempts");
    let marker = harness.root.join("rejected-once");
    let real_git = git_binary();
    write_executable(
        &harness.shim.join("git"),
        "#!/bin/sh\ncase \" $* \" in *' push '*)\n  printf 'push\\n' >> \"$PUSH_ATTEMPTS\"\n  if [ ! -e \"$PUSH_MARKER\" ]; then\n    : > \"$PUSH_MARKER\"\n    printf ' ! [rejected] main -> main (non-fast-forward)\\n' >&2\n    printf 'error: failed to push some refs\\n' >&2\n    exit 1\n  fi\n;; esac\nexec \"$REAL_GIT\" \"$@\"\n",
    );
    let pushed = harness.command_with_env(
        ["push"],
        [
            ("REAL_GIT", real_git.to_string_lossy().into_owned()),
            ("PUSH_ATTEMPTS", attempts.to_string_lossy().into_owned()),
            ("PUSH_MARKER", marker.to_string_lossy().into_owned()),
        ],
    );
    assert!(pushed.status.success(), "{}", stderr(&pushed));
    assert_eq!(stdout(&pushed), "PUSHED main\n");
    assert_eq!(fs::read_to_string(attempts).unwrap(), "push\npush\n");
}

#[test]
fn commit_push_receipt_replays_without_duplicate_commit() {
    let harness = Harness::new("commit-push");
    let remote = harness.root.join("remote.git");
    init_bare(&remote, &harness.home);
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.git(["branch", "-M", "main"]);
    harness.git(["remote", "add", "origin", &format!("file://{}", remote.display())]);
    harness.git(["push", "--quiet", "-u", "origin", "HEAD"]);
    harness.write("intended.txt", "changed\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);

    let output = harness.success(["commit", &transaction, "-m", "test: commit and push", "--push"]);
    assert!(stdout(&output).contains("COMMITTED"));
    assert!(stdout(&output).contains("PUSHED main"));
    let created = harness.git(["rev-parse", "HEAD"]);
    let replay = harness.success(["commit", &transaction, "-m", "ignored", "--push"]);
    assert!(stdout(&replay).starts_with(&format!("PUSHED {transaction} {created}")));
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "2");
}

#[test]
fn commit_push_behind_receipt_retries_push_without_duplicate_commit() {
    let harness = Harness::new("commit-push-behind");
    let remote = harness.root.join("remote.git");
    let updater = harness.root.join("updater");
    init_bare(&remote, &harness.home);
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.git(["branch", "-M", "main"]);
    harness.git(["remote", "add", "origin", &format!("file://{}", remote.display())]);
    harness.git(["push", "--quiet", "-u", "origin", "HEAD"]);
    harness.write("intended.txt", "local\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);

    clone_repository(&remote, &updater, &harness.home);
    fs::write(updater.join("remote.txt"), "remote\n").unwrap();
    git_at(&updater, &harness.home, ["add", "remote.txt"]);
    git_at(&updater, &harness.home, ["commit", "--quiet", "-m", "remote"]);
    git_at(&updater, &harness.home, ["push", "--quiet"]);

    let behind = harness.command(["commit", &transaction, "-m", "test: local", "--push"]);
    assert_eq!(exit_code(&behind), 3);
    assert!(stdout(&behind).contains(&format!("COMMITTED {transaction} ")));
    assert!(stdout(&behind).contains("BEHIND main 1"));
    let receipt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(harness.transaction_json(&transaction)).unwrap()).unwrap();
    assert!(receipt["terminal_at"].is_null());
    assert_eq!(receipt["push_requested"], true);
    let created = harness.git(["rev-parse", "HEAD"]);
    let replay = harness.command(["commit", &transaction, "-m", "ignored", "--push"]);
    assert_eq!(exit_code(&replay), 3);
    assert!(stdout(&replay).contains(&format!("COMMITTED {transaction} {created}")));
    assert_eq!(harness.git(["rev-parse", "HEAD"]), created);
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "2");
}

fn init_bare(path: &std::path::Path, home: &std::path::Path) {
    let output = Command::new("git")
        .args(["init", "--bare", "--quiet"])
        .arg(path)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

fn clone_repository(remote: &std::path::Path, destination: &std::path::Path, home: &std::path::Path) {
    let output = Command::new("git")
        .args(["clone", "--quiet", "--branch", "main"])
        .arg(format!("file://{}", remote.display()))
        .arg(destination)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    git_at(destination, home, ["config", "user.name", "AI Commit Test"]);
    git_at(destination, home, ["config", "user.email", "ai-commit@example.com"]);
    git_at(destination, home, ["config", "commit.gpgsign", "false"]);
}

fn git_binary() -> std::path::PathBuf {
    let output = Command::new("sh").args(["-c", "command -v git"]).output().unwrap();
    assert!(output.status.success());
    std::path::PathBuf::from(String::from_utf8(output.stdout).unwrap().trim())
}

mod common;

use std::{fs, process::Command};

use common::{Harness, exit_code, git_at, git_binary, init_bare, stderr, stdout, write_executable};

#[test]
fn local_only_push_skips_without_fetch_push_or_rebase() {
    let harness = Harness::new("push-no-remote");
    harness.git(["symbolic-ref", "HEAD", "refs/heads/main"]);
    let real_git = git_binary();
    let marker = harness.root.join("network-ran");
    write_executable(
        &harness.shim.join("git"),
        "#!/bin/sh\ncase \" $* \" in *' fetch '*|*' push '*|*' rebase '*) : > \"$NETWORK_MARKER\"; exit 91;; esac\nexec \"$REAL_GIT\" \"$@\"\n",
    );

    for has_commit in [false, true] {
        if has_commit {
            harness.write("intended.txt", "base\n");
            harness.commit_all("base");
        }
        for arguments in [vec!["push"], vec!["push", "--rebase"]] {
            let skipped = harness.success_with_env(
                arguments,
                [("REAL_GIT", real_git.to_str().unwrap()), ("NETWORK_MARKER", marker.to_str().unwrap())],
            );
            assert_eq!(stdout(&skipped), "PUSH_SKIPPED main no-remote\n");
            assert!(stderr(&skipped).is_empty());
            assert!(!marker.exists());
            assert!(harness.git(["remote"]).is_empty());
        }
    }
}

#[test]
fn commit_without_remotes_keeps_a_local_receipt_until_push_is_requested_again() {
    let harness = Harness::new("commit-push-no-remote");
    harness.git(["symbolic-ref", "HEAD", "refs/heads/main"]);
    harness.write("intended.txt", "local\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);

    let committed = harness.success(["commit", &transaction, "-m", "test: local", "--push", "--rebase"]);
    let head = harness.git(["rev-parse", "HEAD"]);
    let local_receipt = format!("COMMITTED {transaction} {}\n", &head[..12]);
    let skipped_receipt = format!("{local_receipt}PUSH_SKIPPED main no-remote\n");
    assert_eq!(stdout(&committed), skipped_receipt);
    let receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(harness.transaction_json(&transaction)).unwrap()).unwrap();
    assert_eq!(receipt["status"], "committed");
    assert_eq!(receipt["push_requested"], false);
    assert!(receipt["terminal_at"].is_number());
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "local");

    let replay = harness.success(["commit", &transaction, "-m", "ignored", "--push"]);
    assert_eq!(stdout(&replay), skipped_receipt);
    assert_eq!(harness.git(["rev-parse", "HEAD"]), head);

    let remote = harness.root.join("remote.git");
    init_bare(&remote, &harness.home);
    harness.git(["remote", "add", "origin", &format!("file://{}", remote.display())]);
    let local_replay = harness.success(["commit", &transaction, "-m", "ignored"]);
    assert_eq!(stdout(&local_replay), local_receipt);
    assert!(harness.git(["ls-remote", "origin", "refs/heads/main"]).is_empty());

    let pushed = harness.success(["commit", &transaction, "-m", "ignored", "--push"]);
    assert_eq!(stdout(&pushed), format!("{local_receipt}PUSHED_NEW main\n"));
    assert_eq!(git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]), head);
}

#[test]
fn a_named_remote_without_origin_or_upstream_is_still_a_configuration_error() {
    let harness = Harness::new("push-other-remote");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.git(["remote", "add", "upstream", "file:///nonexistent.git"]);

    let refused = harness.command(["push"]);
    assert_eq!(exit_code(&refused), 2);
    assert!(stderr(&refused).contains("remote 'origin' does not exist"));
    assert!(stdout(&refused).is_empty());
}

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
fn configured_upstream_without_a_tracking_ref_uses_its_remote() {
    let harness = Harness::new("push-configured-missing-tracking-ref");
    let origin = harness.root.join("origin.git");
    let upstream = harness.root.join("upstream.git");
    init_bare(&origin, &harness.home);
    init_bare(&upstream, &harness.home);
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.git(["branch", "-M", "main"]);
    harness.git(["remote", "add", "origin", &format!("file://{}", origin.display())]);
    harness.git(["remote", "add", "upstream", &format!("file://{}", upstream.display())]);
    harness.git(["config", "branch.main.remote", "upstream"]);
    harness.git(["config", "branch.main.merge", "refs/heads/main"]);

    let pushed = harness.success(["push"]);

    assert_eq!(stdout(&pushed), "PUSHED main\n");
    assert_eq!(harness.git(["rev-parse", "HEAD"]), git_at(&upstream, &harness.home, ["rev-parse", "refs/heads/main"]));
    assert!(harness.git(["ls-remote", "origin", "refs/heads/main"]).is_empty());
}

#[test]
fn standalone_push_refuses_a_differently_named_upstream() {
    let harness = Harness::new("push-mismatched-upstream");
    let remote = harness.root.join("remote.git");
    init_bare(&remote, &harness.home);
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.git(["branch", "-M", "main"]);
    harness.git(["remote", "add", "origin", &format!("file://{}", remote.display())]);
    harness.git(["push", "--quiet", "-u", "origin", "HEAD"]);
    harness.git(["switch", "--quiet", "-c", "fix", "--track", "origin/main"]);
    harness.write("intended.txt", "fix work\n");
    harness.commit_all("fix work");
    let remote_before = git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]);
    let local_before = harness.git(["rev-parse", "HEAD"]);

    let refused = harness.command(["push"]);
    assert_eq!(exit_code(&refused), 2);
    let message = stderr(&refused);
    assert!(message.contains("origin/main"), "{message}");
    assert!(message.contains("fix"), "{message}");
    assert!(message.contains("git config branch.fix.pushRemote origin"), "{message}");
    assert_eq!(git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]), remote_before);
    assert_eq!(harness.git(["rev-parse", "HEAD"]), local_before);
}

#[test]
fn commit_push_refuses_a_differently_named_upstream_without_moving_the_remote() {
    let harness = Harness::new("commit-push-mismatched-upstream");
    let remote = harness.root.join("remote.git");
    init_bare(&remote, &harness.home);
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.git(["branch", "-M", "main"]);
    harness.git(["remote", "add", "origin", &format!("file://{}", remote.display())]);
    harness.git(["push", "--quiet", "-u", "origin", "HEAD"]);
    harness.git(["switch", "--quiet", "-c", "fix", "--track", "origin/main"]);
    harness.write("intended.txt", "fix work\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    let remote_before = git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]);

    let refused = harness.command(["commit", &transaction, "-m", "test: mismatched upstream push", "--push"]);
    assert_eq!(exit_code(&refused), 2);
    let message = stderr(&refused);
    assert!(message.contains("origin/main"), "{message}");
    assert!(message.contains("fix"), "{message}");
    assert!(stdout(&refused).contains("COMMITTED"), "{}", stdout(&refused));
    assert_eq!(git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]), remote_before);
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "fix work");
}

/// Repository whose `pr-1` branch has the configuration `gh pr checkout --branch pr-1` writes for a
/// fork PR: the fork URL as remote and pushRemote, merging the fork's `main`. Returns the origin and
/// fork remotes.
fn fork_pr_fixture(name: &str) -> (Harness, std::path::PathBuf, std::path::PathBuf) {
    let harness = Harness::new(name);
    let origin = harness.root.join("origin.git");
    let fork = harness.root.join("fork.git");
    init_bare(&origin, &harness.home);
    init_bare(&fork, &harness.home);
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.git(["branch", "-M", "main"]);
    harness.git(["remote", "add", "origin", &format!("file://{}", origin.display())]);
    harness.git(["push", "--quiet", "-u", "origin", "HEAD"]);
    let fork_url = format!("file://{}", fork.display());
    harness.git(["push", "--quiet", &fork_url, "HEAD:refs/heads/main"]);
    harness.git(["switch", "--quiet", "-c", "pr-1"]);
    harness.git(["config", "branch.pr-1.remote", &fork_url]);
    harness.git(["config", "branch.pr-1.pushRemote", &fork_url]);
    harness.git(["config", "branch.pr-1.merge", "refs/heads/main"]);
    (harness, origin, fork)
}

#[test]
fn fork_pr_checkout_pushes_to_the_pr_head_without_changing_tracking() {
    let (harness, origin, fork) = fork_pr_fixture("push-fork-pr");
    harness.write("intended.txt", "pr work\n");
    harness.commit_all("pr work");
    let config_before = harness.git(["config", "--get-regexp", "^branch\\.pr-1\\."]);

    let pushed = harness.success(["push"]);

    assert_eq!(stdout(&pushed), "PUSHED pr-1\n");
    assert_eq!(harness.git(["rev-parse", "HEAD"]), git_at(&fork, &harness.home, ["rev-parse", "refs/heads/main"]));
    assert!(harness.git(["ls-remote", "origin", "refs/heads/pr-1"]).is_empty());
    assert_ne!(harness.git(["rev-parse", "HEAD"]), git_at(&origin, &harness.home, ["rev-parse", "refs/heads/main"]));
    assert_eq!(harness.git(["config", "--get-regexp", "^branch\\.pr-1\\."]), config_before);
}

#[test]
fn url_upstream_pushes_without_any_named_remotes() {
    let (harness, _origin, fork) = fork_pr_fixture("push-url-no-named-remotes");
    harness.git(["remote", "remove", "origin"]);
    assert!(harness.git(["remote"]).is_empty());
    harness.write("intended.txt", "pr work\n");
    harness.commit_all("pr work");

    let pushed = harness.success(["push"]);

    assert_eq!(stdout(&pushed), "PUSHED pr-1\n");
    assert_eq!(harness.git(["rev-parse", "HEAD"]), git_at(&fork, &harness.home, ["rev-parse", "refs/heads/main"]));
}

#[test]
fn fork_pr_checkout_rebases_onto_the_fetched_pr_head() {
    let (harness, _origin, fork) = fork_pr_fixture("push-fork-pr-rebase");
    let contributor = harness.root.join("contributor");
    clone_repository(&fork, &contributor, &harness.home);
    fs::write(contributor.join("remote.txt"), "contributor\n").unwrap();
    git_at(&contributor, &harness.home, ["add", "remote.txt"]);
    git_at(&contributor, &harness.home, ["commit", "--quiet", "-m", "contributor"]);
    git_at(&contributor, &harness.home, ["push", "--quiet"]);
    harness.write("intended.txt", "pr work\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);

    let output = harness.success(["commit", &transaction, "-m", "test: pr work", "--push", "--rebase"]);

    let receipt = stdout(&output);
    assert!(receipt.contains("REBASED pr-1 1\n"), "{receipt}");
    assert!(receipt.ends_with("PUSHED pr-1\n"), "{receipt}");
    let head = harness.git(["rev-parse", "HEAD"]);
    assert_eq!(git_at(&fork, &harness.home, ["rev-parse", "refs/heads/main"]), head);
    assert_eq!(harness.read("remote.txt"), "contributor\n");
}

#[test]
fn named_pr_remote_with_matching_push_remote_pushes_to_a_differently_named_head() {
    let (harness, _origin, fork) = fork_pr_fixture("push-named-pr-remote");
    harness.git(["remote", "add", "contributor", &format!("file://{}", fork.display())]);
    harness.git(["fetch", "--quiet", "contributor"]);
    harness.git(["branch", "--quiet", "--set-upstream-to=contributor/main", "pr-1"]);
    harness.git(["config", "branch.pr-1.pushRemote", "contributor"]);
    harness.write("intended.txt", "pr work\n");
    harness.commit_all("pr work");

    let pushed = harness.success(["push"]);

    assert_eq!(stdout(&pushed), "PUSHED pr-1\n");
    assert_eq!(harness.git(["rev-parse", "HEAD"]), git_at(&fork, &harness.home, ["rev-parse", "refs/heads/main"]));
    assert_eq!(harness.git(["rev-parse", "--abbrev-ref", "@{upstream}"]), "contributor/main");
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
        "#!/bin/sh\ncase \" $* \" in *' push '*)\n  printf 'push\\n' >> \"$PUSH_ATTEMPTS\"\n  if [ ! -e \"$PUSH_MARKER\" ]; then\n    : > \"$PUSH_MARKER\"\n    printf 'To file:///remote.git\\n!\\tHEAD:refs/heads/main\\t[rejected] (non-fast-forward)\\nDone\\n'\n    printf 'error: failed to push some refs\\n' >&2\n    exit 1\n  fi\n;; esac\nexec \"$REAL_GIT\" \"$@\"\n",
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
fn unrelated_push_rejection_is_not_retried() {
    let harness = Harness::new("push-unrelated-rejection");
    let remote = harness.root.join("remote.git");
    init_bare(&remote, &harness.home);
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.git(["branch", "-M", "main"]);
    harness.git(["remote", "add", "origin", &format!("file://{}", remote.display())]);
    harness.git(["push", "--quiet", "-u", "origin", "HEAD"]);
    harness.write("intended.txt", "rejected\n");
    harness.commit_all("rejected");

    let attempts = harness.root.join("push-attempts");
    let real_git = git_binary();
    write_executable(
        &harness.shim.join("git"),
        "#!/bin/sh\ncase \" $* \" in *' push '*)\n  printf 'push\\n' >> \"$PUSH_ATTEMPTS\"\n  printf 'To file:///remote.git\\n!\\tHEAD:refs/heads/main\\t[remote rejected] (pre-receive hook declined)\\nDone\\n'\n  printf 'error: failed to push some refs\\n' >&2\n  exit 1\n;; esac\nexec \"$REAL_GIT\" \"$@\"\n",
    );
    let rejected = harness.command_with_env(
        ["push"],
        [
            ("REAL_GIT", real_git.to_string_lossy().into_owned()),
            ("PUSH_ATTEMPTS", attempts.to_string_lossy().into_owned()),
        ],
    );

    assert_eq!(exit_code(&rejected), 1);
    assert!(stderr(&rejected).contains("error: failed to push some refs"));
    assert_eq!(fs::read_to_string(attempts).unwrap(), "push\n");
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
    assert!(stdout(&replay).starts_with(&format!("PUSHED {transaction} {}\n", &created[..12])));
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
    assert!(stdout(&replay).contains(&format!("COMMITTED {transaction} {}\n", &created[..12])));
    assert_eq!(harness.git(["rev-parse", "HEAD"]), created);
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "2");
}

#[test]
fn commit_push_replay_after_rebase_reports_integrated_head() {
    let harness = Harness::new("commit-push-rebase");
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
    let created = harness.git(["rev-parse", "HEAD"]);
    harness.git(["pull", "--quiet", "--rebase", "--no-autostash"]);
    let rebased = harness.git(["rev-parse", "HEAD"]);
    assert_ne!(rebased, created);

    let replay = harness.success(["commit", &transaction, "-m", "ignored", "--push"]);
    assert_eq!(
        stdout(&replay),
        format!(
            "COMMITTED {transaction} {}\nINTEGRATED {transaction} {}\nPUSHED main\n",
            &created[..12],
            &rebased[..12]
        )
    );
    harness.git(["fetch", "--quiet"]);
    assert_eq!(harness.git(["rev-parse", "origin/main"]), rebased);
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "3");
}

#[test]
fn commit_push_replay_after_identical_upstream_change_reports_integrated_head() {
    let harness = Harness::new("commit-push-identical");
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
    fs::write(updater.join("intended.txt"), "local\n").unwrap();
    git_at(&updater, &harness.home, ["commit", "--quiet", "-a", "-m", "same change upstream"]);
    git_at(&updater, &harness.home, ["push", "--quiet"]);

    let behind = harness.command(["commit", &transaction, "-m", "test: local", "--push"]);
    assert_eq!(exit_code(&behind), 3);
    let created = harness.git(["rev-parse", "HEAD"]);
    harness.git(["pull", "--quiet", "--rebase", "--no-autostash"]);
    let head = harness.git(["rev-parse", "HEAD"]);
    assert_eq!(head, harness.git(["rev-parse", "origin/main"]));

    let replay = harness.success(["commit", &transaction, "-m", "ignored", "--push"]);
    assert_eq!(
        stdout(&replay),
        format!("COMMITTED {transaction} {}\nINTEGRATED {transaction} {}\nPUSHED main\n", &created[..12], &head[..12])
    );
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "2");
    assert_eq!(harness.read("intended.txt"), "local\n");
}

#[test]
fn commit_push_replay_after_dropped_commit_reports_superseded_and_pushes_nothing() {
    let harness = Harness::new("commit-push-superseded");
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
    let created = harness.git(["rev-parse", "HEAD"]);
    harness.git(["fetch", "--quiet"]);
    harness.git(["reset", "--quiet", "--hard", "origin/main"]);
    let upstream = harness.git(["rev-parse", "origin/main"]);

    let replay = harness.command(["commit", &transaction, "-m", "ignored", "--push"]);
    assert_eq!(exit_code(&replay), 3);
    assert_eq!(
        stdout(&replay),
        format!("COMMITTED {transaction} {}\nSUPERSEDED {transaction} {}\n", &created[..12], &created[..12])
    );
    harness.git(["fetch", "--quiet"]);
    assert_eq!(harness.git(["rev-parse", "origin/main"]), upstream);
    let receipt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(harness.transaction_json(&transaction)).unwrap()).unwrap();
    assert!(receipt["terminal_at"].is_null());
    assert_eq!(receipt["push_requested"], true);
}

/// Repository with `main` pushed to a bare remote, plus an `updater` clone that has pushed
/// `remote.txt` so the local branch is one commit behind.
fn behind_fixture(name: &str) -> (Harness, std::path::PathBuf) {
    let harness = Harness::new(name);
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
    (harness, remote)
}

#[test]
fn push_rebase_integrates_a_clean_behind_branch_and_pushes() {
    let (harness, remote) = behind_fixture("push-rebase-clean");
    harness.write("intended.txt", "local\n");
    harness.commit_all("local");
    harness.write(".gitignore", "ignored.txt\n");
    harness.commit_all("ignore");
    harness.write("ignored.txt", "ignored files are not dirt\n");
    let local_before = harness.git(["rev-parse", "HEAD"]);

    let pushed = harness.success(["push", "--rebase"]);

    assert_eq!(stdout(&pushed), "REBASED main 1\nPUSHED main\n");
    let head = harness.git(["rev-parse", "HEAD"]);
    assert_ne!(head, local_before);
    assert_eq!(git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]), head);
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "4");
    assert_eq!(harness.read("remote.txt"), "remote\n");
    assert_eq!(harness.read("intended.txt"), "local\n");
    assert_eq!(harness.read("ignored.txt"), "ignored files are not dirt\n");
    assert!(harness.git(["status", "--porcelain"]).is_empty());
}

#[test]
fn push_rebase_refuses_an_untracked_file_and_leaves_the_branch_alone() {
    let (harness, remote) = behind_fixture("push-rebase-untracked");
    harness.write("intended.txt", "local\n");
    harness.commit_all("local");
    harness.write("notes.txt", "untracked work from another agent\n");
    let local_before = harness.git(["rev-parse", "HEAD"]);
    let remote_before = git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]);

    let behind = harness.command(["push", "--rebase"]);

    assert_eq!(exit_code(&behind), 3);
    assert_eq!(stdout(&behind), "BEHIND main 1\n");
    assert!(stderr(&behind).contains("not clean"), "{}", stderr(&behind));
    assert_eq!(harness.git(["rev-parse", "HEAD"]), local_before);
    assert_eq!(git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]), remote_before);
    assert_eq!(harness.read("notes.txt"), "untracked work from another agent\n");
}

#[test]
fn push_rebase_integrates_again_when_the_remote_moves_before_the_push() {
    let (harness, remote) = behind_fixture("push-rebase-moving-remote");
    let updater = harness.root.join("updater");
    harness.write("intended.txt", "local\n");
    harness.commit_all("local");

    // The first push attempt lets the updater land another commit first, so the real push is
    // rejected as non-fast-forward after the first rebase already integrated the earlier one.
    let marker = harness.root.join("moved-once");
    let real_git = git_binary();
    write_executable(
        &harness.shim.join("git"),
        "#!/bin/sh\ncase \" $* \" in *' push '*)\n  if [ ! -e \"$PUSH_MARKER\" ]; then\n    : > \"$PUSH_MARKER\"\n    \"$REAL_GIT\" -C \"$UPDATER\" commit --quiet --allow-empty -m 'remote again' >/dev/null 2>&1\n    \"$REAL_GIT\" -C \"$UPDATER\" push --quiet >/dev/null 2>&1\n  fi\n;; esac\nexec \"$REAL_GIT\" \"$@\"\n",
    );
    let pushed = harness.command_with_env(
        ["push", "--rebase"],
        [
            ("REAL_GIT", real_git.to_string_lossy().into_owned()),
            ("PUSH_MARKER", marker.to_string_lossy().into_owned()),
            ("UPDATER", updater.to_string_lossy().into_owned()),
        ],
    );

    assert!(pushed.status.success(), "{}", stderr(&pushed));
    assert_eq!(stdout(&pushed), "REBASED main 2\nPUSHED main\n");
    let head = harness.git(["rev-parse", "HEAD"]);
    assert_eq!(git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]), head);
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "4");
    assert_eq!(harness.git(["log", "--format=%s", "-n", "2"]), "local\nremote again");
}

#[test]
fn push_rebase_refuses_tracked_changes_and_leaves_the_branch_alone() {
    let (harness, remote) = behind_fixture("push-rebase-dirty");
    harness.write("intended.txt", "local\n");
    harness.commit_all("local");
    harness.write("intended.txt", "another agent's unstaged edit\n");
    let local_before = harness.git(["rev-parse", "HEAD"]);
    let remote_before = git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]);

    let behind = harness.command(["push", "--rebase"]);

    assert_eq!(exit_code(&behind), 3);
    assert_eq!(stdout(&behind), "BEHIND main 1\n");
    assert!(stderr(&behind).contains("not clean"), "{}", stderr(&behind));
    assert_eq!(harness.git(["rev-parse", "HEAD"]), local_before);
    assert_eq!(git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]), remote_before);
    assert_eq!(harness.read("intended.txt"), "another agent's unstaged edit\n");
}

#[test]
fn push_rebase_aborts_a_conflicting_rebase_and_reports_behind() {
    let (harness, remote) = behind_fixture("push-rebase-conflict");
    harness.write("remote.txt", "conflicting local\n");
    harness.git(["add", "remote.txt"]);
    harness.git(["commit", "--quiet", "-m", "conflicting local"]);
    let local_before = harness.git(["rev-parse", "HEAD"]);
    let remote_before = git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]);

    let behind = harness.command(["push", "--rebase"]);

    assert_eq!(exit_code(&behind), 3);
    assert_eq!(stdout(&behind), "BEHIND main 1\n");
    assert!(stderr(&behind).contains("aborted"), "{}", stderr(&behind));
    assert_eq!(harness.git(["rev-parse", "HEAD"]), local_before);
    assert_eq!(git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]), remote_before);
    assert!(harness.git(["status", "--porcelain"]).is_empty());
    assert!(!harness.root.join("repo/.git/rebase-merge").exists());
    assert!(!harness.root.join("repo/.git/rebase-apply").exists());
}

#[test]
fn commit_push_rebase_reports_rebased_integrated_and_pushed() {
    let (harness, remote) = behind_fixture("commit-push-rebase-flag");
    harness.write("intended.txt", "local\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);

    let output = harness.success(["commit", &transaction, "-m", "test: local", "--push", "--rebase"]);

    let head = harness.git(["rev-parse", "HEAD"]);
    let receipt_text = stdout(&output);
    let lines: Vec<&str> = receipt_text.lines().collect();
    assert_eq!(lines.len(), 4, "{receipt_text}");
    assert!(lines[0].starts_with(&format!("COMMITTED {transaction} ")), "{}", lines[0]);
    assert_eq!(lines[1], "REBASED main 1");
    assert_eq!(lines[2], format!("INTEGRATED {transaction} {}", &head[..12]));
    assert_eq!(lines[3], "PUSHED main");
    assert_eq!(git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]), head);
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "3");
    let receipt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(harness.transaction_json(&transaction)).unwrap()).unwrap();
    assert_eq!(receipt["status"], "pushed");
    let replay = harness.success(["commit", &transaction, "-m", "ignored", "--push"]);
    assert!(stdout(&replay).starts_with(&format!("PUSHED {transaction} ")), "{}", stdout(&replay));
}

#[test]
fn commit_push_rebase_retains_unrelated_upstream_changes_in_the_same_file() {
    for automatic in [false, true] {
        let (harness, remote, transaction) = same_file_rebase_fixture("commit-push-same-file");
        let created = if automatic {
            None
        } else {
            let behind = harness.command(["commit", &transaction, "-m", "test: local", "--push"]);
            assert_eq!(exit_code(&behind), 3);
            let created = harness.git(["rev-parse", "HEAD"]);
            harness.git(["pull", "--quiet", "--rebase", "--no-autostash"]);
            Some(created)
        };

        let output = harness.success(["commit", &transaction, "-m", "test: local", "--push", "--rebase"]);
        let head = harness.git(["rev-parse", "HEAD"]);
        let lines: Vec<_> = stdout(&output).lines().map(str::to_owned).collect();
        assert_eq!(lines.len(), if automatic { 4 } else { 3 });
        assert!(lines[0].starts_with(&format!("COMMITTED {transaction} ")));
        if let Some(created) = created {
            assert_eq!(lines[0], format!("COMMITTED {transaction} {}", &created[..12]));
            assert_ne!(created, head);
        } else {
            assert_eq!(lines[1], "REBASED main 1");
        }
        assert_eq!(lines[lines.len() - 2], format!("INTEGRATED {transaction} {}", &head[..12]));
        assert_eq!(lines.last().unwrap(), "PUSHED main");
        assert_eq!(git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]), head);
        assert_eq!(
            harness.read("intended.txt"),
            same_file_content()
                .replace("line 03", "local 03")
                .replace("line 10", "remote 10")
                .replace("line 17", "local 17")
        );
        assert!(harness.git(["status", "--porcelain"]).is_empty());
    }
}

#[test]
fn commit_push_replay_rejects_reverted_changed_and_whitespace_changed_hunks() {
    for (name, replacement) in [("reverted", "line 03"), ("changed", "different 03"), ("whitespace", "local 03 ")] {
        let (harness, remote, transaction) = same_file_rebase_fixture(name);
        let behind = harness.command(["commit", &transaction, "-m", "test: local", "--push"]);
        assert_eq!(exit_code(&behind), 3);
        let created = harness.git(["rev-parse", "HEAD"]);
        harness.git(["pull", "--quiet", "--rebase", "--no-autostash"]);
        harness.write("intended.txt", &harness.read("intended.txt").replace("local 03", replacement));
        harness.commit_all("change one intended hunk");
        let head = harness.git(["rev-parse", "HEAD"]);
        let upstream = git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]);

        let replay = harness.command(["commit", &transaction, "-m", "ignored", "--push"]);

        assert_eq!(exit_code(&replay), 3, "{}", stderr(&replay));
        assert_eq!(
            stdout(&replay),
            format!("COMMITTED {transaction} {}\nSUPERSEDED {transaction} {}\n", &created[..12], &created[..12])
        );
        assert_eq!(harness.git(["rev-parse", "HEAD"]), head);
        assert_eq!(git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]), upstream);
        assert!(harness.git(["status", "--porcelain"]).is_empty());
    }
}

#[test]
fn commit_push_integration_bypasses_custom_and_union_merge_drivers() {
    for driver in ["ours", "union"] {
        let (harness, remote, transaction) = same_file_rebase_fixture(driver);
        let behind = harness.command(["commit", &transaction, "-m", "test: local", "--push"]);
        assert_eq!(exit_code(&behind), 3);
        harness.git(["pull", "--quiet", "--rebase", "--no-autostash"]);
        harness.write("intended.txt", &harness.read("intended.txt").replace("local 03", "changed 03"));
        harness.commit_all("change intended content");
        harness.write(".git/info/attributes", &format!("intended.txt merge={driver}\n"));
        harness.git(["config", "merge.ours.driver", "touch \"$DRIVER_MARKER\""]);
        let marker = harness.root.join("driver-ran");
        let upstream = git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]);

        let replay = harness.command_with_env(
            ["commit", &transaction, "-m", "ignored", "--push"],
            [("DRIVER_MARKER", marker.as_os_str())],
        );

        assert_eq!(exit_code(&replay), 3, "{}", stderr(&replay));
        assert!(stdout(&replay).contains("SUPERSEDED"));
        assert!(!marker.exists());
        assert_eq!(git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]), upstream);
    }
}

#[test]
fn commit_push_integration_git_failure_does_not_push_or_report_superseded() {
    let (harness, remote, transaction) = same_file_rebase_fixture("integration-git-failure");
    let behind = harness.command(["commit", &transaction, "-m", "test: local", "--push"]);
    assert_eq!(exit_code(&behind), 3);
    harness.git(["pull", "--quiet", "--rebase", "--no-autostash"]);
    let upstream = git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]);
    let real_git = git_binary();
    write_executable(
        &harness.shim.join("git"),
        "#!/bin/sh\ncase \" $* \" in *' merge-file '*) exit 128;; esac\nexec \"$REAL_GIT\" \"$@\"\n",
    );

    let replay = harness
        .command_with_env(["commit", &transaction, "-m", "ignored", "--push"], [("REAL_GIT", real_git.as_os_str())]);

    assert_eq!(exit_code(&replay), 1);
    assert!(!stdout(&replay).contains("SUPERSEDED"));
    assert!(!stdout(&replay).contains("PUSHED"));
    assert_eq!(git_at(&remote, &harness.home, ["rev-parse", "refs/heads/main"]), upstream);
}

fn same_file_content() -> String {
    (1..=20).map(|line| format!("line {line:02}\n")).collect()
}

fn same_file_rebase_fixture(name: &str) -> (Harness, std::path::PathBuf, String) {
    let harness = Harness::new(name);
    let remote = harness.root.join("remote.git");
    let updater = harness.root.join("updater");
    init_bare(&remote, &harness.home);
    harness.write("intended.txt", &same_file_content());
    harness.commit_all("base");
    harness.git(["branch", "-M", "main"]);
    harness.git(["remote", "add", "origin", &format!("file://{}", remote.display())]);
    harness.git(["push", "--quiet", "-u", "origin", "HEAD"]);
    harness.write("intended.txt", &same_file_content().replace("line 03", "local 03").replace("line 17", "local 17"));
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    clone_repository(&remote, &updater, &harness.home);
    fs::write(updater.join("intended.txt"), same_file_content().replace("line 10", "remote 10")).unwrap();
    git_at(&updater, &harness.home, ["commit", "--quiet", "-a", "-m", "upstream edit in same file"]);
    git_at(&updater, &harness.home, ["push", "--quiet"]);
    (harness, remote, transaction)
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

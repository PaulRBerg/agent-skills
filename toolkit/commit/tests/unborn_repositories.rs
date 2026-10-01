mod common;

use std::{fs, path::Path, process::Command};

use common::{Harness, exit_code, git_at, stderr, stdout, write_executable};

#[test]
fn explicit_prepare_creates_a_root_commit_without_creating_an_index() {
    let harness = Harness::new("unborn-explicit");
    let branch = harness.git(["symbolic-ref", "--short", "HEAD"]);
    harness.write("intended.txt", "initial\n");

    let prepared = harness.success(["prepare", "--porcelain", "--", "intended.txt"]);
    let transaction = prepared_id(&stdout(&prepared));
    assert!(!harness.git_output(["rev-parse", "--verify", "HEAD"]).status.success());
    assert!(!harness.git_output(["show-ref", "--verify", "--quiet", &format!("refs/heads/{branch}")]).status.success());
    assert!(!harness.repo.join(".git/index").exists());
    assert!(stdout(&harness.success(["show", &transaction])).contains("unborn\ttrue\n"));

    harness.success(["commit", &transaction, "-m", "feat: initial commit"]);
    let parents = harness.git(["rev-list", "--parents", "-n", "1", "HEAD"]);
    assert_eq!(parents.split_ascii_whitespace().count(), 1);
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "initial");
    assert_eq!(harness.read("intended.txt"), "initial\n");
    assert!(harness.repo.join(".git/index").is_file());
    assert!(harness.git(["status", "--short"]).is_empty());
}

#[test]
fn all_and_staged_modes_handle_initial_worktrees_and_indexes() {
    let all = Harness::new("unborn-all");
    all.write("one.txt", "one\n");
    all.write("nested/two.txt", "two\n");
    let prepared = all.success(["prepare", "--all", "--porcelain"]);
    let transaction = prepared_id(&stdout(&prepared));
    all.success(["commit", &transaction, "-m", "feat: add initial files"]);
    assert_eq!(all.git(["show", "HEAD:one.txt"]), "one");
    assert_eq!(all.git(["show", "HEAD:nested/two.txt"]), "two");

    let empty_staged = Harness::new("unborn-empty-staged");
    let empty = empty_staged.command(["prepare", "--staged"]);
    assert_eq!(exit_code(&empty), 2);
    assert!(stderr(&empty).contains("prepared transaction has no changes"), "{}", stderr(&empty));
    assert!(!empty_staged.repo.join(".git/index").exists());

    let staged = Harness::new("unborn-staged");
    staged.write("staged.txt", "exact snapshot\n");
    staged.git(["add", "staged.txt"]);
    let prepared = staged.success(["prepare", "--staged", "--porcelain"]);
    let transaction = prepared_id(&stdout(&prepared));
    staged.success(["commit", &transaction, "-m", "feat: stage initial file"]);
    assert_eq!(staged.git(["show", "HEAD:staged.txt"]), "exact snapshot");
}

#[test]
fn explicit_initial_commit_preserves_unrelated_initial_staging() {
    let harness = Harness::new("unborn-unrelated-staging");
    harness.write("intended.txt", "intended\n");
    harness.write("unrelated.txt", "staged elsewhere\n");
    harness.git(["add", "unrelated.txt"]);
    let unrelated = harness.git(["rev-parse", ":unrelated.txt"]);

    let (transaction, _) = harness.prepare(&["intended.txt"]);
    harness.success(["commit", &transaction, "-m", "feat: commit intended initial file"]);
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "intended");
    assert!(harness.git(["ls-tree", "HEAD", "unrelated.txt"]).is_empty());
    assert_eq!(harness.git(["rev-parse", ":unrelated.txt"]), unrelated);
    assert_eq!(harness.git(["diff", "--cached", "--name-only"]), "unrelated.txt");
}

#[test]
fn initial_hooks_and_signing_follow_the_regular_transaction_flow() {
    let hooks = Harness::new("unborn-hooks");
    hooks.write("intended.txt", "needs formatting\n");
    write_executable(
        &hooks.repo.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nset -eu\nprintf 'formatted\\n' > intended.txt\nprintf 'from hook\\n' > hook-added.txt\ngit add -- intended.txt hook-added.txt\n",
    );
    let (transaction, _) = hooks.prepare(&["intended.txt"]);
    let committed = hooks.success(["commit", &transaction, "-m", "feat: format initial content"]);
    assert!(stdout(&committed).contains("HOOK_ADDED hook-added.txt"));
    assert_eq!(hooks.git(["show", "HEAD:intended.txt"]), "formatted");
    assert_eq!(hooks.git(["show", "HEAD:hook-added.txt"]), "from hook");
    assert!(hooks.git(["status", "--short"]).is_empty());

    let signing = Harness::new("unborn-signing");
    signing.write("intended.txt", "initial\n");
    let (transaction, _) = signing.prepare(&["intended.txt"]);
    let signer = signing.root.join("failing-gpg");
    write_executable(&signer, "#!/bin/sh\nprintf 'intentional signing failure\\n' >&2\nexit 1\n");
    signing.git(["config", "commit.gpgsign", "true"]);
    signing.git(["config", "gpg.program", signer.to_str().unwrap()]);
    let failed = signing.command(["commit", &transaction, "-m", "feat: signed initial"]);
    assert_eq!(exit_code(&failed), 1);
    assert!(stderr(&failed).contains("failed to sign"));
    signing.success(["commit", &transaction, "-m", "feat: unsigned initial", "--no-gpg-sign"]);
    assert_eq!(signing.git(["rev-list", "--count", "HEAD"]), "1");
}

#[test]
fn branch_creation_after_prepare_is_applied_or_conflicts_without_mutation() {
    let clean = Harness::new("unborn-moved-clean");
    clean.write("intended.txt", "prepared\n");
    let (transaction, _) = clean.prepare(&["intended.txt"]);
    let winner = create_root(&clean, "winner.txt", "winner\n", "winner");
    clean.write("winner.txt", "winner\n");
    clean.success(["commit", &transaction, "-m", "feat: add prepared initial file"]);
    assert_eq!(clean.git(["rev-parse", "HEAD^"]), winner);
    assert_eq!(clean.git(["show", "HEAD:intended.txt"]), "prepared");
    assert!(clean.git(["status", "--short"]).is_empty());

    let conflict = Harness::new("unborn-moved-conflict");
    conflict.write("intended.txt", "prepared\n");
    let (transaction, _) = conflict.prepare(&["intended.txt"]);
    let winner = create_root(&conflict, "intended.txt", "winner\n", "winner");
    let before = conflict.git(["status", "--short"]);
    assert!(!conflict.repo.join(".git/index").exists());
    let failed = conflict.command(["commit", &transaction, "-m", "feat: conflicting initial file"]);
    assert_eq!(exit_code(&failed), 3);
    assert!(stderr(&failed).contains("do not apply cleanly"));
    assert_eq!(conflict.git(["rev-parse", "HEAD"]), winner);
    assert_eq!(conflict.git(["status", "--short"]), before);
    assert_eq!(conflict.read("intended.txt"), "prepared\n");
    assert!(!conflict.repo.join(".git/index").exists());
    assert!(!conflict.repo.join(".git/index.lock").exists());
}

#[test]
fn competing_branch_creation_after_root_construction_is_retryable() {
    let harness = Harness::new("unborn-cas-race");
    harness.write("intended.txt", "prepared\n");
    let branch = harness.git(["symbolic-ref", "--short", "HEAD"]);
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    let marker = harness.root.join("race-marker");
    let input = harness.root.join("race-input");
    let candidate = harness.root.join("race-candidate");
    let real_git = git_binary();
    write_executable(
        &harness.shim.join("git"),
        "#!/bin/sh\nset -eu\ncase \" $* \" in\n  *\" update-ref --stdin \"*)\n    if [ \"${AI_COMMIT_TEST_RACE_ROOT:-}\" = 1 ] && [ ! -e \"$RACE_MARKER\" ]; then\n      : > \"$RACE_MARKER\"\n      cat > \"$RACE_INPUT\"\n      candidate=$(/usr/bin/awk '/^create refs\\/heads\\// { print $3; exit }' \"$RACE_INPUT\")\n      test -n \"$candidate\"\n      printf '%s\\n' \"$candidate\" > \"$RACE_CANDIDATE\"\n      blob=$(printf 'winner\\n' | \"$REAL_GIT\" hash-object -w --stdin)\n      tree=$(printf '100644 blob %s\\twinner.txt\\n' \"$blob\" | \"$REAL_GIT\" mktree)\n      winner=$(\"$REAL_GIT\" commit-tree \"$tree\" -m winner)\n      \"$REAL_GIT\" update-ref \"refs/heads/$RACE_BRANCH\" \"$winner\"\n      exec \"$REAL_GIT\" \"$@\" < \"$RACE_INPUT\"\n    fi\n  ;;\nesac\nexec \"$REAL_GIT\" \"$@\"\n",
    );
    let failed = harness.command_with_env(
        ["commit", &transaction, "-m", "feat: race root"],
        [
            ("AI_COMMIT_TEST_RACE_ROOT", "1"),
            ("RACE_MARKER", marker.to_str().unwrap()),
            ("RACE_INPUT", input.to_str().unwrap()),
            ("RACE_CANDIDATE", candidate.to_str().unwrap()),
            ("RACE_BRANCH", &branch),
            ("REAL_GIT", real_git.to_str().unwrap()),
        ],
    );
    assert_eq!(exit_code(&failed), 3);
    let root_candidate = fs::read_to_string(&candidate).unwrap().trim().to_owned();
    assert!(!harness.git_output(["merge-base", "--is-ancestor", &root_candidate, "HEAD"]).status.success());
    let journal: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(harness.transaction_json(&transaction)).unwrap()).unwrap();
    assert!(journal["pending_commit"].is_null());

    write_executable(&harness.shim.join("git"), &format!("#!/bin/sh\nexec '{}' \"$@\"\n", real_git.display()));
    harness.write("winner.txt", "winner\n");
    harness.success(["commit", &transaction, "-m", "feat: retry root race"]);
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "2");
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "prepared");
}

#[test]
fn root_ref_interruption_recovers_the_same_commit_and_a_descendant_index() {
    let harness = Harness::new("unborn-recovery");
    harness.write("intended.txt", "prepared\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    let interrupted = harness.command_with_env(
        ["commit", &transaction, "-m", "feat: recover root"],
        [("AI_COMMIT_TEST_FAIL_AFTER_REF_UPDATE", "1")],
    );
    assert_eq!(exit_code(&interrupted), 3);
    let root = harness.git(["rev-parse", "HEAD"]);
    let journal: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(harness.transaction_json(&transaction)).unwrap()).unwrap();
    let token = journal["index_lock_token"].as_str().expect("index lock token");
    fs::write(harness.repo.join(".git/index.lock"), format!("ai-commit-index-lock {transaction} {token}\n")).unwrap();
    harness.success(["commit", &transaction, "-m", "ignored retry message"]);
    assert_eq!(harness.git(["rev-parse", "HEAD"]), root);
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "1");
    assert!(harness.git(["status", "--short"]).is_empty());
    assert!(!harness.repo.join(".git/index.lock").exists());

    let descendant = Harness::new("unborn-recovery-descendant");
    descendant.write("intended.txt", "prepared\n");
    let (transaction, _) = descendant.prepare(&["intended.txt"]);
    let interrupted = descendant.command_with_env(
        ["commit", &transaction, "-m", "feat: recover below descendant"],
        [("AI_COMMIT_TEST_FAIL_AFTER_REF_UPDATE", "1")],
    );
    assert_eq!(exit_code(&interrupted), 3);
    let child = create_child(&descendant, "intended.txt", "descendant\n", "descendant");
    descendant.write("intended.txt", "descendant\n");
    descendant.success(["commit", &transaction, "-m", "ignored retry message"]);
    assert_eq!(descendant.git(["rev-parse", "HEAD"]), child);
    assert_eq!(descendant.git(["show", ":intended.txt"]), "descendant");
    assert_eq!(descendant.git(["rev-list", "--count", "HEAD"]), "2");
}

#[test]
fn initial_commit_pushes_a_new_branch_and_standalone_push_rejects_unborn_head() {
    let pushed = Harness::new("unborn-push");
    let remote = pushed.root.join("remote.git");
    init_bare(&remote, &pushed.home);
    let branch = pushed.git(["symbolic-ref", "--short", "HEAD"]);
    pushed.git(["remote", "add", "origin", &format!("file://{}", remote.display())]);
    pushed.write("intended.txt", "initial\n");
    let (transaction, _) = pushed.prepare(&["intended.txt"]);
    let output = pushed.success(["commit", &transaction, "-m", "feat: push initial", "--push"]);
    assert!(stdout(&output).contains(&format!("PUSHED_NEW {branch}")));
    assert_eq!(pushed.git(["rev-parse", "--abbrev-ref", "@{upstream}"]), format!("origin/{branch}"));
    assert_eq!(
        pushed.git(["rev-parse", "HEAD"]),
        git_at(&remote, &pushed.home, ["rev-parse", &format!("refs/heads/{branch}")])
    );

    let unborn = Harness::new("unborn-standalone-push");
    let remote = unborn.root.join("remote.git");
    init_bare(&remote, &unborn.home);
    let branch = unborn.git(["symbolic-ref", "--short", "HEAD"]);
    unborn.git(["remote", "add", "origin", &format!("file://{}", remote.display())]);
    let failed = unborn.command(["push"]);
    assert_eq!(exit_code(&failed), 2);
    assert!(stderr(&failed).contains("HEAD has no commit"));
    assert!(
        !git_at_output(&remote, &unborn.home, ["show-ref", "--verify", "--quiet", &format!("refs/heads/{branch}")])
            .status
            .success()
    );
}

#[test]
fn legacy_journals_without_unborn_or_optional_parent_fields_remain_readable() {
    let harness = Harness::new("unborn-compatibility");
    harness.write("base.txt", "base\n");
    harness.commit_all("base");
    harness.write("base.txt", "prepared\n");
    let (transaction, _) = harness.prepare(&["base.txt"]);
    let interrupted = harness.command_with_env(
        ["commit", &transaction, "-m", "feat: preserve old journal"],
        [("AI_COMMIT_TEST_FAIL_AFTER_REF_UPDATE", "1")],
    );
    assert_eq!(exit_code(&interrupted), 3);

    let journal_path = harness.transaction_json(&transaction);
    let mut journal: serde_json::Value = serde_json::from_str(&fs::read_to_string(&journal_path).unwrap()).unwrap();
    assert!(journal["pending_commit"]["parent"].is_string());
    journal.as_object_mut().unwrap().remove("unborn");
    fs::write(&journal_path, serde_json::to_vec_pretty(&journal).unwrap()).unwrap();

    let recovered = harness.success(["commit", &transaction, "-m", "ignored retry message"]);
    assert!(stdout(&recovered).contains(&format!("COMMITTED {transaction} ")));
    assert!(stdout(&harness.success(["show", &transaction])).contains("unborn\tfalse\n"));
}

fn prepared_id(output: &str) -> String {
    output.lines().find_map(|line| line.strip_prefix("PREPARED\t")).expect("PREPARED record").to_owned()
}

fn create_root(harness: &Harness, path: &str, contents: &str, message: &str) -> String {
    let blob = harness.git_input(["hash-object", "-w", "--stdin"], contents.as_bytes());
    let tree = harness.git_input(["mktree"], format!("100644 blob {blob}\t{path}\n").as_bytes());
    let commit = harness.git(["commit-tree", &tree, "-m", message]);
    harness.git(["update-ref", "HEAD", &commit]);
    commit
}

fn create_child(harness: &Harness, path: &str, contents: &str, message: &str) -> String {
    let blob = harness.git_input(["hash-object", "-w", "--stdin"], contents.as_bytes());
    let tree = harness.git_input(["mktree"], format!("100644 blob {blob}\t{path}\n").as_bytes());
    let parent = harness.git(["rev-parse", "HEAD"]);
    let commit = harness.git(["commit-tree", &tree, "-p", &parent, "-m", message]);
    harness.git(["update-ref", "HEAD", &commit, &parent]);
    commit
}

fn init_bare(path: &Path, home: &Path) {
    let output = Command::new("git")
        .args(["init", "--bare", "--quiet"])
        .arg(path)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

fn git_at_output<I, S>(repository: &Path, home: &Path, args: I) -> std::process::Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    Command::new("git")
        .arg("-C")
        .arg(repository)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .args(args)
        .output()
        .unwrap()
}

fn git_binary() -> std::path::PathBuf {
    let output = Command::new("sh").args(["-c", "command -v git"]).output().unwrap();
    assert!(output.status.success());
    std::path::PathBuf::from(String::from_utf8(output.stdout).unwrap().trim())
}

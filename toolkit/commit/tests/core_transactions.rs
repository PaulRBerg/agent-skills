mod common;

use std::fs;

use common::{Harness, exit_code, stderr, stdout, write_executable};

#[test]
fn intended_snapshot_preserves_later_index_and_unrelated_staging() {
    let harness = Harness::new("snapshot");
    harness.write("intended.txt", "base\n");
    harness.write("deleted.txt", "delete me\n");
    harness.write("unrelated.txt", "base\n");
    harness.write("tool.sh", "#!/bin/sh\nexit 0\n");
    harness.commit_all("base");

    harness.write("intended.txt", "prepared\n");
    fs::remove_file(harness.repo.join("deleted.txt")).unwrap();
    harness.write("dir/nested.txt", "nested\n");
    harness.write("untracked.txt", "untracked\n");
    harness.write("unrelated.txt", "base\nstaged elsewhere\n");
    harness.git(["add", "unrelated.txt"]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        symlink("intended.txt", harness.repo.join("link.txt")).unwrap();
        fs::set_permissions(harness.repo.join("tool.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    }

    let unrelated_oid = harness.git(["rev-parse", ":unrelated.txt"]);
    let index_before = harness.git(["hash-object", ".git/index"]);
    let paths = if cfg!(unix) {
        vec!["intended.txt", "deleted.txt", "dir", "untracked.txt", "tool.sh", "link.txt"]
    } else {
        vec!["intended.txt", "deleted.txt", "dir", "untracked.txt", "tool.sh"]
    };
    let (transaction, preview) = harness.prepare(&paths);
    assert!(preview.contains("CHANGE\tM\\tintended.txt"));
    assert_eq!(index_before, harness.git(["hash-object", ".git/index"]));
    let transaction_ref = format!("refs/ai-commit/transactions/{transaction}");
    let index_ref = format!("refs/ai-commit/indexes/{transaction}");
    let base_ref = format!("refs/ai-commit/bases/{transaction}");
    assert_eq!(harness.git(["cat-file", "-t", &transaction_ref]), "tree");
    assert_eq!(harness.git(["cat-file", "-t", &index_ref]), "tree");
    assert_eq!(harness.git(["cat-file", "-t", &base_ref]), "commit");

    harness.write("intended.txt", "later staged\n");
    harness.git(["add", "intended.txt"]);
    let later_oid = harness.git(["rev-parse", ":intended.txt"]);
    let output = harness.success(["commit", &transaction, "-m", "test: immutable snapshot"]);
    assert!(stdout(&output).contains(&format!("COMMITTED {transaction} ")));
    assert_eq!(harness.git(["cat-file", "-t", &transaction_ref]), "commit");

    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "prepared");
    assert_eq!(harness.git(["show", "HEAD:dir/nested.txt"]), "nested");
    assert_eq!(harness.git(["show", "HEAD:untracked.txt"]), "untracked");
    assert!(harness.git(["ls-tree", "HEAD", "deleted.txt"]).is_empty());
    assert_eq!(harness.git(["rev-parse", ":intended.txt"]), later_oid);
    assert_eq!(harness.git(["rev-parse", ":unrelated.txt"]), unrelated_oid);
    let staged = harness.git(["diff", "--cached", "--name-only"]);
    assert_eq!(staged, "intended.txt\nunrelated.txt");
    assert_eq!(harness.read("intended.txt"), "later staged\n");
    #[cfg(unix)]
    {
        assert_eq!(harness.git(["show", "HEAD:link.txt"]), "intended.txt");
        assert!(harness.git(["ls-tree", "HEAD", "tool.sh"]).starts_with("100755 blob "));
        assert!(harness.git(["ls-tree", "HEAD", "link.txt"]).starts_with("120000 blob "));
    }
}

#[test]
fn case_only_file_and_directory_renames_keep_exact_spelling() {
    let harness = Harness::new("case-rename");
    harness.git(["config", "core.ignorecase", "true"]);
    harness.write("case-file.txt", "file\n");
    harness.write("case-dir/nested.txt", "nested\n");
    harness.commit_all("base");

    fs::rename(harness.repo.join("case-file.txt"), harness.repo.join("Case-File.txt")).unwrap();
    fs::rename(harness.repo.join("case-dir"), harness.repo.join("Case-Dir")).unwrap();
    let (transaction, preview) = harness.prepare(&["case-file.txt", "Case-File.txt", "case-dir", "Case-Dir"]);
    for path in ["case-file.txt", "Case-File.txt", "case-dir/nested.txt", "Case-Dir/nested.txt"] {
        assert!(preview.contains(&format!("PATH\t{path}")), "missing {path}:\n{preview}");
    }
    harness.success(["commit", &transaction, "-m", "test: preserve case rename"]);
    assert_eq!(harness.git(["ls-tree", "-r", "--name-only", "HEAD"]), "Case-Dir/nested.txt\nCase-File.txt");
    assert!(harness.git(["status", "--short"]).is_empty());
}

#[test]
fn formatter_hook_uses_isolated_index_and_preserves_shared_staging() {
    let harness = Harness::new("formatter-hook");
    harness.write("intended.txt", "base\n");
    harness.write("unrelated.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "needs formatting\n");
    harness.write("unrelated.txt", "base\nstaged elsewhere\n");
    harness.git(["add", "unrelated.txt"]);
    let unrelated_oid = harness.git(["rev-parse", ":unrelated.txt"]);
    let hook_log = harness.root.join("hook-index.log");
    let hook = format!(
        "#!/bin/sh\nset -eu\ncase \"${{GIT_INDEX_FILE:-}}\" in ''|*.lock) exit 91;; esac\nprintf '%s\\n' \"$GIT_INDEX_FILE\" > '{}'\nprintf 'formatted\\n' > intended.txt\ngit add -- intended.txt\n",
        hook_log.display()
    );
    write_executable(&harness.repo.join(".git/hooks/pre-commit"), &hook);

    let (transaction, _) = harness.prepare(&["intended.txt"]);
    harness.success(["commit", &transaction, "-m", "test: formatter hook"]);
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "formatted");
    assert!(!fs::read_to_string(hook_log).unwrap().trim().ends_with(".lock"));
    assert_eq!(harness.git(["rev-parse", ":unrelated.txt"]), unrelated_oid);
    assert_eq!(harness.git(["diff", "--cached", "--name-only"]), "unrelated.txt");
}

#[test]
fn hook_added_paths_are_committed_reported_and_reconciled() {
    let harness = Harness::new("hook-added");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");
    write_executable(
        &harness.repo.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nset -eu\nprintf 'from hook\\n' > hook-added.txt\ngit add -- hook-added.txt\n",
    );

    let (transaction, _) = harness.prepare(&["intended.txt"]);
    let output = harness.success(["commit", &transaction, "-m", "test: hook addition"]);
    assert!(stdout(&output).contains("HOOK_ADDED hook-added.txt"));
    assert_eq!(harness.git(["show", "HEAD:hook-added.txt"]), "from hook");
    assert!(harness.git(["diff", "--cached", "--name-only"]).is_empty());
    harness.git(["switch", "--quiet", "-c", "receipt-replay"]);
    let replay = harness.success(["commit", &transaction, "-m", "ignored on replay"]);
    assert!(stdout(&replay).contains("HOOK_ADDED hook-added.txt"));
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "2");
}

#[test]
fn hook_failure_is_retryable_and_foreign_index_lock_is_preserved() {
    let harness = Harness::new("hook-failure");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    write_executable(
        &harness.repo.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nprintf 'intentional hook failure\\n' >&2\nexit 1\n",
    );
    let head_before = harness.git(["rev-parse", "HEAD"]);
    let index_before = harness.git(["hash-object", ".git/index"]);
    let failed = harness.command(["commit", &transaction, "-m", "test: fail hook"]);
    assert_eq!(exit_code(&failed), 1);
    assert!(stderr(&failed).contains("intentional hook failure"));
    assert_eq!(harness.git(["rev-parse", "HEAD"]), head_before);
    assert_eq!(harness.git(["hash-object", ".git/index"]), index_before);
    assert!(!harness.repo.join(".git/index.lock").exists());

    fs::remove_file(harness.repo.join(".git/hooks/pre-commit")).unwrap();
    fs::write(harness.repo.join(".git/index.lock"), "other owner\n").unwrap();
    let locked = harness.command(["commit", &transaction, "-m", "test: blocked lock"]);
    assert_eq!(exit_code(&locked), 3);
    assert!(stderr(&locked).contains("default Git index remains locked"));
    assert_eq!(fs::read_to_string(harness.repo.join(".git/index.lock")).unwrap(), "other owner\n");
    assert_eq!(harness.git(["rev-parse", "HEAD"]), head_before);
}

#[test]
fn inherited_index_and_invalid_repository_states_are_invocation_errors() {
    let harness = Harness::new("preflight");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");
    let inherited = harness.command_with_env(
        ["prepare", "--", "intended.txt"],
        [("GIT_INDEX_FILE", harness.repo.join(".git/index").to_string_lossy().into_owned())],
    );
    assert_eq!(exit_code(&inherited), 2);
    assert!(stderr(&inherited).contains("GIT_INDEX_FILE is already set"));

    fs::write(harness.repo.join(".git/MERGE_HEAD"), harness.git(["rev-parse", "HEAD"])).unwrap();
    let merging = harness.command(["prepare", "--", "intended.txt"]);
    assert_eq!(exit_code(&merging), 2);
    assert!(stderr(&merging).contains("MERGE_HEAD"));
}

#[test]
fn no_verify_bypasses_retryable_verification_hooks() {
    let harness = Harness::new("no-verify");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    write_executable(
        &harness.repo.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nprintf 'must be bypassed\\n' >&2\nexit 1\n",
    );
    let output = harness.success(["commit", &transaction, "-m", "test: bypass hook", "--no-verify"]);
    assert!(stdout(&output).contains("COMMITTED"));
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "changed");
}

#[test]
fn directory_expansion_handles_file_directory_replacements() {
    let harness = Harness::new("file-directory");
    harness.write("node/child.txt", "child\n");
    harness.commit_all("directory base");
    fs::remove_dir_all(harness.repo.join("node")).unwrap();
    harness.write("node", "file\n");
    let (to_file, _) = harness.prepare(&["node"]);
    harness.success(["commit", &to_file, "-m", "test: replace directory"]);
    assert_eq!(harness.git(["show", "HEAD:node"]), "file");
    assert!(harness.git(["ls-tree", "HEAD", "node/child.txt"]).is_empty());

    fs::remove_file(harness.repo.join("node")).unwrap();
    harness.write("node/next.txt", "next\n");
    let (to_directory, _) = harness.prepare(&["node"]);
    harness.success(["commit", &to_directory, "-m", "test: replace file"]);
    assert_eq!(harness.git(["show", "HEAD:node/next.txt"]), "next");
    assert!(harness.git(["ls-tree", "HEAD", "node"]).starts_with("040000 tree "));
}

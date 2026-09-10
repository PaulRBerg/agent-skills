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
fn newly_staged_ignored_paths_can_be_prepared_explicitly_and_with_all() {
    let harness = Harness::new("staged-ignored");
    harness.write("base.txt", "base\n");
    harness.commit_all("base");
    harness.write("staged.txt", "staged before ignore rule\n");
    harness.git(["add", "staged.txt"]);
    harness.write(".gitignore", "staged.txt\n");
    harness.git(["add", ".gitignore"]);
    let index_before = harness.git(["hash-object", ".git/index"]);

    let (_, explicit_preview) = harness.prepare(&["staged.txt"]);
    assert!(explicit_preview.contains("CHANGE\tA\\tstaged.txt"));

    let all = harness.success(["prepare", "--all", "--porcelain"]);
    let all_preview = stdout(&all);
    assert!(all_preview.contains("CHANGE\tA\\t.gitignore"));
    assert!(all_preview.contains("CHANGE\tA\\tstaged.txt"));
    assert_eq!(harness.git(["hash-object", ".git/index"]), index_before);
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
        "#!/bin/sh\nset -eu\ncase \"${{GIT_INDEX_FILE:-}}\" in ''|*.lock) exit 91;; esac\nprintf '%s\\n%s\\n' \"$GIT_INDEX_FILE\" \"${{AI_COMMIT_HOOK_MODE-unset}}\" > '{}'\nprintf 'formatted\\n' > intended.txt\ngit add -- intended.txt\n",
        hook_log.display()
    );
    write_executable(&harness.repo.join(".git/hooks/pre-commit"), &hook);

    let (transaction, _) = harness.prepare(&["intended.txt"]);
    harness.success(["commit", &transaction, "-m", "test: formatter hook"]);
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "formatted");
    let hook_log = fs::read_to_string(hook_log).unwrap();
    let mut hook_log = hook_log.lines();
    assert!(!hook_log.next().unwrap().ends_with(".lock"));
    assert_eq!(hook_log.next(), Some("unset"));
    assert_eq!(harness.git(["rev-parse", ":unrelated.txt"]), unrelated_oid);
    assert_eq!(harness.git(["diff", "--cached", "--name-only"]), "unrelated.txt");
}

#[test]
fn hook_added_paths_are_committed_reported_and_reconciled() {
    let harness = Harness::new("hook-added");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");
    let hook_log = harness.root.join("hook-mode.log");
    write_executable(
        &harness.repo.join(".git/hooks/pre-commit"),
        &format!(
            "#!/bin/sh\nset -eu\nprintf '%s\\n' \"${{AI_COMMIT_HOOK_MODE-unset}}\" > '{}'\nprintf 'from hook\\n' > hook-added.txt\ngit add -- hook-added.txt\n",
            hook_log.display()
        ),
    );

    let (transaction, _) = harness.prepare(&["intended.txt"]);
    let output = harness.success(["commit", &transaction, "-m", "test: hook addition"]);
    assert!(stdout(&output).contains("HOOK_ADDED hook-added.txt"));
    assert_eq!(fs::read_to_string(hook_log).unwrap(), "unset\n");
    assert_eq!(harness.git(["show", "HEAD:hook-added.txt"]), "from hook");
    assert!(harness.git(["diff", "--cached", "--name-only"]).is_empty());
    harness.git(["switch", "--quiet", "-c", "receipt-replay"]);
    let replay = harness.success(["commit", &transaction, "-m", "ignored on replay"]);
    assert!(stdout(&replay).contains("HOOK_ADDED hook-added.txt"));
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "2");
}

#[test]
fn unrelated_dirty_paths_do_not_select_snapshot_hook_mode() {
    let harness = Harness::new("unrelated-dirty-hook");
    harness.write("intended.txt", "base\n");
    harness.write("unrelated.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");
    harness.write("unrelated.txt", "unrelated dirty worktree\n");
    let hook_log = harness.root.join("hook-environment.log");
    write_executable(
        &harness.repo.join(".git/hooks/pre-commit"),
        &format!(
            "#!/bin/sh\nset -eu\nprintf '%s\\n%s\\n%s\\n%s\\n' \"$PWD\" \"${{GIT_WORK_TREE-unset}}\" \"${{AI_COMMIT_HOOK_MODE-unset}}\" \"${{AI_COMMIT_ORIGINAL_WORKTREE-unset}}\" > '{}'\n",
            hook_log.display()
        ),
    );

    let (transaction, _) = harness.prepare(&["intended.txt"]);
    harness.success(["commit", &transaction, "-m", "test: ignore unrelated dirt"]);

    assert_eq!(
        fs::read_to_string(hook_log).unwrap(),
        format!("{}\nunset\nunset\nunset\n", harness.repo.canonicalize().unwrap().display())
    );
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "changed");
    assert_eq!(harness.read("unrelated.txt"), "unrelated dirty worktree\n");
    assert_eq!(harness.git(["status", "--short"]), " M unrelated.txt");
}

#[test]
fn post_commit_uses_physical_worktree_without_snapshot_marker() {
    let harness = Harness::new("post-commit-physical");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "prepared\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    harness.write("intended.txt", "later physical worktree\n");

    write_executable(
        &harness.repo.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nset -eu\n[ \"${AI_COMMIT_HOOK_MODE:-}\" = snapshot-check ]\n",
    );
    let post_commit_log = harness.root.join("post-commit-environment.log");
    write_executable(
        &harness.repo.join(".git/hooks/post-commit"),
        &format!(
            "#!/bin/sh\nset -eu\nprintf '%s\\n%s\\n%s\\n%s\\n' \"$PWD\" \"${{GIT_WORK_TREE-unset}}\" \"${{AI_COMMIT_HOOK_MODE-unset}}\" \"${{AI_COMMIT_ORIGINAL_WORKTREE-unset}}\" > '{}'\ncat intended.txt >> '{}'\n",
            post_commit_log.display(),
            post_commit_log.display()
        ),
    );

    harness.success(["commit", &transaction, "-m", "test: physical post commit"]);

    assert_eq!(
        fs::read_to_string(post_commit_log).unwrap(),
        format!("{}\nunset\nunset\nunset\nlater physical worktree\n", harness.repo.canonicalize().unwrap().display())
    );
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "prepared");
    assert_eq!(harness.read("intended.txt"), "later physical worktree\n");
    assert_eq!(harness.git(["status", "--short"]), " M intended.txt");
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
fn inherited_object_environment_cannot_redirect_prepared_objects() {
    let harness = Harness::new("isolated-object-environment");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "prepared\n");
    let foreign_objects = harness.root.join("foreign-objects");
    fs::create_dir(&foreign_objects).unwrap();
    let repository_objects = harness.repo.join(".git/objects");

    let prepared = harness.command_with_env(
        ["prepare", "--porcelain", "--", "intended.txt"],
        [
            ("GIT_OBJECT_DIRECTORY", foreign_objects.to_string_lossy().into_owned()),
            ("GIT_ALTERNATE_OBJECT_DIRECTORIES", repository_objects.to_string_lossy().into_owned()),
        ],
    );

    assert!(prepared.status.success(), "{}", stderr(&prepared));
    assert!(fs::read_dir(&foreign_objects).unwrap().next().is_none());
    let prepared_stdout = stdout(&prepared);
    let transaction =
        prepared_stdout.lines().find_map(|line| line.strip_prefix("PREPARED\t")).expect("PREPARED record");
    assert_eq!(harness.git(["cat-file", "-t", &format!("refs/ai-commit/transactions/{transaction}")]), "tree");
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
fn prepared_validation_runs_directly_in_the_complete_prepared_tree() {
    let harness = Harness::new("prepared-validation-environment");
    harness.write("intended.txt", "base\n");
    harness.write("sibling.txt", "base sibling\n");
    harness.commit_all("base");
    harness.write("intended.txt", "prepared\n");
    harness.write("sibling.txt", "prepared sibling\n");
    let validator = harness.root.join("validator");
    let log = harness.root.join("validation.log");
    let shell_marker = harness.root.join("must-not-exist");
    let literal_argument = format!("$VALIDATION_LITERAL; touch {}", shell_marker.display());
    write_executable(
        &validator,
        "#!/bin/sh\nset -eu\ntest \"$1\" = \"$EXPECTED_LITERAL\"\ntest \"$(cat intended.txt)\" = prepared\ntest \"$(cat sibling.txt)\" = 'prepared sibling'\ntest \"$(git diff --cached --name-only)\" = 'intended.txt\nsibling.txt'\ntest -z \"$(git diff --name-only)\"\ntest \"${GIT_INDEX_FILE##*.lock}\" = \"$GIT_INDEX_FILE\"\ntest \"$AI_COMMIT_VALIDATION_MODE\" = prepared-tree\ntest \"$AI_COMMIT_ORIGINAL_WORKTREE\" = \"$EXPECTED_ORIGINAL\"\ntest -z \"${AI_COMMIT_HOOK_MODE:-}\"\ntest -z \"${GIT_PREFIX:-}\"\nprintf '%s\\t%s\\t%s\\t%s\\n' \"$PWD\" \"$GIT_DIR\" \"$GIT_WORK_TREE\" \"$GIT_INDEX_FILE\" > \"$VALIDATION_LOG\"\n",
    );
    harness.write(
        ".agents/commit.toml",
        &format!(
            "[message]\nformat = \"conventional\"\n[validation]\ncommand = [\"{}\", \"{}\"]\n",
            validator.display(),
            literal_argument
        ),
    );
    let (transaction, _) = harness.prepare(&["intended.txt", "sibling.txt"]);
    harness.write("intended.txt", "physical worktree changed after prepare\n");
    let log_text = log.to_string_lossy().into_owned();
    let original = harness.repo.canonicalize().unwrap().to_string_lossy().into_owned();
    let inherited = "inherited".to_owned();
    let inherited_prefix = "inherited/".to_owned();
    let expanded_literal = "expanded".to_owned();
    harness.success_with_env(
        ["commit", &transaction, "-m", "test: prepared validation"],
        [
            ("VALIDATION_LOG", &log_text),
            ("EXPECTED_ORIGINAL", &original),
            ("EXPECTED_LITERAL", &literal_argument),
            ("VALIDATION_LITERAL", &expanded_literal),
            ("AI_COMMIT_VALIDATION_MODE", &inherited),
            ("AI_COMMIT_ORIGINAL_WORKTREE", &inherited),
            ("AI_COMMIT_HOOK_MODE", &inherited),
            ("GIT_PREFIX", &inherited_prefix),
        ],
    );

    let validation_log = fs::read_to_string(log).unwrap();
    let fields: Vec<_> = validation_log.trim_end().split('\t').collect();
    assert_eq!(fields.len(), 4, "unexpected validation log: {fields:?}");
    assert_ne!(fields[0], original);
    assert_eq!(fields[0], fields[2]);
    assert_eq!(fields[1], harness.repo.join(".git").canonicalize().unwrap().to_string_lossy());
    assert_eq!(
        std::path::Path::new(fields[3]).file_name().and_then(|name| name.to_str()),
        Some("snapshot-validation-index")
    );
    assert_ne!(fields[3], harness.repo.join(".git/index").to_string_lossy());
    assert!(!shell_marker.exists());
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "prepared");
    assert_eq!(harness.git(["show", "HEAD:sibling.txt"]), "prepared sibling");
    assert_eq!(harness.read("intended.txt"), "physical worktree changed after prepare\n");
}

#[test]
fn prepared_validation_can_resolve_ignored_local_directories() {
    let harness = Harness::new("prepared-validation-local-directories");
    let validator = harness.root.join("validator");
    write_executable(
        &validator,
        "#!/bin/sh\nset -eu\ntest \"$PWD\" != \"$AI_COMMIT_ORIGINAL_WORKTREE\"\ntest \"$(cat node_modules/@example/tool/marker.txt)\" = dependency\ntest \"$(cat .artifacts/reviews/evidence.md)\" = evidence\ntest \"$(cat workspace/bin/tool)\" = tool\ntest \"$(git check-ignore node_modules)\" = node_modules\ntest \"$(git check-ignore .artifacts)\" = .artifacts\ntest \"$(git check-ignore workspace/bin)\" = workspace/bin\n",
    );
    harness.write(".gitignore", "node_modules/\n");
    harness.write("intended.txt", "base\n");
    harness.write("workspace/owner.txt", "tracked\n");
    harness.write(
        ".agents/commit.toml",
        &format!("[message]\nformat = \"conventional\"\n[validation]\ncommand = [\"{}\"]\n", validator.display()),
    );
    harness.commit_all("base");
    harness.write(".gitignore", "node_modules/\n.artifacts/\nworkspace/bin/\n");
    harness.write("node_modules/@example/tool/marker.txt", "dependency\n");
    harness.write(".artifacts/reviews/evidence.md", "evidence\n");
    harness.write("workspace/bin/tool", "tool\n");
    harness.write("intended.txt", "prepared\n");

    let (transaction, _) = harness.prepare(&[".gitignore", "intended.txt"]);
    harness.success(["commit", &transaction, "-m", "test: resolve ignored dependencies"]);

    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "prepared");
    assert_eq!(harness.git(["show", "HEAD:.gitignore"]), "node_modules/\n.artifacts/\nworkspace/bin/");
    assert_eq!(harness.read("node_modules/@example/tool/marker.txt"), "dependency\n");
    assert_eq!(harness.read(".artifacts/reviews/evidence.md"), "evidence\n");
    assert_eq!(harness.read("workspace/bin/tool"), "tool\n");
}

#[test]
fn snapshot_hooks_without_validation_do_not_resolve_ignored_root_node_modules() {
    let harness = Harness::new("snapshot-hook-no-node-modules");
    harness.write(".gitignore", "node_modules/\n");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("node_modules/@example/tool/marker.txt", "dependency\n");
    harness.write("intended.txt", "prepared\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    harness.write("intended.txt", "physical worktree changed after prepare\n");
    write_executable(
        &harness.repo.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nset -eu\ntest \"${AI_COMMIT_HOOK_MODE:-}\" = snapshot-check\ntest ! -e node_modules/@example/tool/marker.txt\n",
    );

    harness.success(["commit", &transaction, "-m", "test: preserve snapshot hook dependencies"]);

    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "prepared");
    assert_eq!(harness.read("node_modules/@example/tool/marker.txt"), "dependency\n");
}

#[test]
fn prepared_validation_failure_is_retryable_without_shared_state_changes() {
    let harness = Harness::new("prepared-validation-failure");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "prepared\n");
    let validator = harness.root.join("validator");
    write_executable(&validator, "#!/bin/sh\nprintf 'intentional validation failure\\n' >&2\nexit 23\n");
    harness.write(
        ".agents/commit.toml",
        &format!("[message]\nformat = \"conventional\"\n[validation]\ncommand = [\"{}\"]\n", validator.display()),
    );
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    let head_before = harness.git(["rev-parse", "HEAD"]);
    let index_before = harness.git(["hash-object", ".git/index"]);
    let transaction_ref = format!("refs/ai-commit/transactions/{transaction}");
    let ref_before = harness.git(["rev-parse", &transaction_ref]);
    let worktree_before = harness.read("intended.txt");

    let failed = harness.command(["commit", &transaction, "-m", "test: validation failure"]);
    assert_eq!(exit_code(&failed), 1, "{}", stderr(&failed));
    assert!(stderr(&failed).contains("intentional validation failure"), "{}", stderr(&failed));
    assert!(stderr(&failed).contains("prepared validation failed with exit status: 23"), "{}", stderr(&failed));
    assert!(stdout(&harness.success(["show", &transaction])).starts_with(&format!("PREPARED {transaction}\n")));
    assert_eq!(harness.git(["rev-parse", "HEAD"]), head_before);
    assert_eq!(harness.git(["hash-object", ".git/index"]), index_before);
    assert_eq!(harness.git(["rev-parse", &transaction_ref]), ref_before);
    assert_eq!(harness.read("intended.txt"), worktree_before);

    write_executable(&validator, "#!/bin/sh\nexit 0\n");
    harness.success(["commit", &transaction, "-m", "test: validation retry"]);
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "prepared");
}

#[test]
fn prepared_validation_runs_with_no_verify_while_verification_hooks_are_bypassed() {
    let harness = Harness::new("prepared-validation-no-verify");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "prepared\n");
    let validator = harness.root.join("validator");
    let validation_marker = harness.root.join("validation-ran");
    let hook_marker = harness.root.join("hook-ran");
    write_executable(&validator, "#!/bin/sh\n: > \"$VALIDATION_MARKER\"\n");
    for hook in ["pre-commit", "commit-msg"] {
        write_executable(
            &harness.repo.join(".git/hooks").join(hook),
            "#!/bin/sh\n: > \"$HOOK_MARKER\"\nprintf 'verification hook should be bypassed\\n' >&2\nexit 91\n",
        );
    }
    harness.write(
        ".agents/commit.toml",
        &format!("[message]\nformat = \"conventional\"\n[validation]\ncommand = [\"{}\"]\n", validator.display()),
    );
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    let validation_marker_text = validation_marker.to_string_lossy().into_owned();
    let hook_marker_text = hook_marker.to_string_lossy().into_owned();
    harness.success_with_env(
        ["commit", &transaction, "-m", "test: no verify validation", "--no-verify"],
        [("VALIDATION_MARKER", &validation_marker_text), ("HOOK_MARKER", &hook_marker_text)],
    );
    assert!(validation_marker.exists());
    assert!(!hook_marker.exists());
}

#[test]
fn prepared_validation_content_drift_is_rejected_without_admitting_changes() {
    for (name, mutation) in [
        ("worktree", "printf 'validation mutation\\n' > intended.txt\n"),
        ("staged", "printf 'validation mutation\\n' > intended.txt\ngit add -- intended.txt\n"),
    ] {
        let harness = Harness::new(&format!("prepared-validation-drift-{name}"));
        harness.write("intended.txt", "base\n");
        harness.commit_all("base");
        harness.write("intended.txt", "prepared\n");
        let validator = harness.root.join("validator");
        write_executable(&validator, &format!("#!/bin/sh\nset -eu\n{mutation}"));
        harness.write(
            ".agents/commit.toml",
            &format!("[message]\nformat = \"conventional\"\n[validation]\ncommand = [\"{}\"]\n", validator.display()),
        );
        let (transaction, _) = harness.prepare(&["intended.txt"]);
        let head_before = harness.git(["rev-parse", "HEAD"]);
        let index_before = harness.git(["hash-object", ".git/index"]);

        let failed = harness.command(["commit", &transaction, "-m", "test: reject validation drift"]);
        assert_eq!(exit_code(&failed), 1, "{name}: {}", stderr(&failed));
        let diagnostic = stderr(&failed);
        assert!(
            diagnostic.contains("prepared validation modified tracked or staged content: intended.txt"),
            "{name}: {diagnostic}"
        );
        assert!(diagnostic.contains("validation changes were not admitted"), "{name}: {diagnostic}");
        assert!(
            diagnostic.contains(&format!("transaction {transaction} remains prepared and retryable")),
            "{name}: {diagnostic}"
        );
        assert!(stdout(&harness.success(["show", &transaction])).starts_with(&format!("PREPARED {transaction}\n")));
        assert_eq!(harness.git(["rev-parse", "HEAD"]), head_before, "{name}");
        assert_eq!(harness.git(["hash-object", ".git/index"]), index_before, "{name}");
        assert_eq!(harness.read("intended.txt"), "prepared\n", "{name}");
        assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "base", "{name}");
    }
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

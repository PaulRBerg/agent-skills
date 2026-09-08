mod common;

use std::fs;

use common::{Harness, exit_code, stderr, stdout, write_executable};

#[test]
fn validation_failure_preserves_the_immutable_transaction_and_explains_recovery() {
    let harness = Harness::new("validate-failure-recovery");
    harness.write("evidence.json", "{\"amount\":\"1.00\"}\n");
    harness.commit_all("base");
    harness.write("evidence.json", "{\"amount\": \"1.00\"}\n");
    configure_validator(
        &harness,
        "#!/bin/sh\nset -eu\nif grep -Fq '{\"amount\": \"1.00\"}' evidence.json; then\n  printf 'synthetic JSON style fixture rejected\n' >&2\n  exit 23\nfi\n",
    );
    let (transaction, _) = harness.prepare(&["evidence.json"]);
    let head_before = harness.git(["rev-parse", "HEAD"]);
    let index_before = harness.git(["hash-object", ".git/index"]);
    let journal_before = fs::read(harness.transaction_json(&transaction)).unwrap();
    let worktree_before = harness.read("evidence.json");
    let refs_before = transaction_refs(&harness, &transaction);

    let failed = harness.command(["validate", &transaction]);

    assert_eq!(exit_code(&failed), 1, "{}", stderr(&failed));
    let diagnostic = stderr(&failed);
    assert!(diagnostic.contains("synthetic JSON style fixture rejected"), "{diagnostic}");
    assert!(diagnostic.contains("configured validation"), "{diagnostic}");
    assert!(diagnostic.contains("no commit"), "{diagnostic}");
    assert!(diagnostic.contains(&format!("ai-commit discard {transaction}")), "{diagnostic}");
    assert!(diagnostic.contains("retry"), "{diagnostic}");
    assert!(diagnostic.contains("prepare"), "{diagnostic}");
    assert_eq!(harness.git(["rev-parse", "HEAD"]), head_before);
    assert_eq!(harness.git(["hash-object", ".git/index"]), index_before);
    assert_eq!(harness.read("evidence.json"), worktree_before);
    assert_eq!(fs::read(harness.transaction_json(&transaction)).unwrap(), journal_before);
    assert_eq!(transaction_refs(&harness, &transaction), refs_before);
    assert!(stdout(&harness.success(["show", &transaction])).starts_with(&format!("PREPARED {transaction}\n")));
}

#[test]
fn validation_uses_the_prepared_snapshot_until_a_new_prepare_or_transient_retry() {
    let harness = Harness::new("validate-immutable-snapshot");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "invalid snapshot\n");
    configure_validator(
        &harness,
        "#!/bin/sh\nset -eu\ntest \"$(cat intended.txt)\" = 'invalid snapshot'\nprintf 'still invalid\n' >&2\nexit 12\n",
    );
    let (transaction, _) = harness.prepare(&["intended.txt"]);

    let first = harness.command(["validate", &transaction]);
    assert_eq!(exit_code(&first), 1, "{}", stderr(&first));
    harness.write("intended.txt", "repaired only in live worktree\n");
    let second = harness.command(["validate", &transaction]);
    assert_eq!(exit_code(&second), 1, "{}", stderr(&second));
    assert!(stderr(&second).contains("still invalid"), "{}", stderr(&second));

    harness.success(["discard", &transaction]);
    configure_validator(
        &harness,
        "#!/bin/sh\nset -eu\ntest \"$(cat intended.txt)\" = 'repaired only in live worktree'\n",
    );
    let (corrected, _) = harness.prepare(&["intended.txt"]);
    let validated = harness.success(["validate", &corrected]);
    assert_validated(&harness, &corrected, &validated);
    assert_eq!(harness.read("intended.txt"), "repaired only in live worktree\n");
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "base");

    let transient = Harness::new("validate-transient-retry");
    transient.write("intended.txt", "base\n");
    transient.commit_all("base");
    transient.write("intended.txt", "prepared\n");
    let ready = transient.root.join("dependency-restored");
    configure_validator(
        &transient,
        &format!(
            "#!/bin/sh\nset -eu\nif ! test -f '{}'; then\n  printf 'dependency unavailable\\n' >&2\n  exit 19\nfi\ntest \"$(cat intended.txt)\" = prepared\n",
            ready.display()
        ),
    );
    let (transaction, _) = transient.prepare(&["intended.txt"]);
    let failed = transient.command(["validate", &transaction]);
    assert_eq!(exit_code(&failed), 1, "{}", stderr(&failed));
    fs::write(&ready, "ready\n").unwrap();
    let validated = transient.success(["validate", &transaction]);
    assert_validated(&transient, &transaction, &validated);
}

#[test]
fn validation_success_is_exact_preflight_and_commit_revalidates() {
    let harness = Harness::new("validate-success-preflight");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "prepared\n");
    let validator = configure_validator(
        &harness,
        "#!/bin/sh\nset -eu\nprintf 'ran\\n' >> \"$VALIDATION_LOG\"\ntest \"$(cat intended.txt)\" = prepared\ncase \"$EXPECTED_SHARED_INDEX_LOCK\" in\n  absent) test ! -e \"$AI_COMMIT_ORIGINAL_WORKTREE/.git/index.lock\" ;;\n  owned) test -e \"$AI_COMMIT_ORIGINAL_WORKTREE/.git/index.lock\" ;;\n  *) exit 92 ;;\nesac\n",
    );
    let hook_marker = harness.root.join("hook-ran");
    write_executable(&harness.repo.join(".git/hooks/pre-commit"), "#!/bin/sh\n: > \"$HOOK_MARKER\"\n");
    let signing_marker = harness.root.join("signer-ran");
    let signer = harness.root.join("signer");
    write_executable(&signer, &format!("#!/bin/sh\n: > '{}'\nexit 91\n", signing_marker.display()));
    harness.git(["config", "commit.gpgsign", "true"]);
    harness.git(["config", "gpg.program", signer.to_str().unwrap()]);
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    harness.write("intended.txt", "live content after prepare\n");
    let validation_log = harness.root.join("validation-ran");
    let validation_log_text = validation_log.to_string_lossy().into_owned();
    let hook_marker_text = hook_marker.to_string_lossy().into_owned();
    let push_marker = harness.root.join("push-ran");
    let real_git = git_binary();
    write_executable(
        &harness.shim.join("git"),
        "#!/bin/sh\nset -eu\ncase \" $* \" in *\" push \"*) : > \"$PUSH_MARKER\"; exit 91;; esac\nexec \"$REAL_GIT\" \"$@\"\n",
    );

    let validated = harness.success_with_env(
        ["validate", &transaction],
        [
            ("VALIDATION_LOG", validation_log_text.as_str()),
            ("HOOK_MARKER", hook_marker_text.as_str()),
            ("EXPECTED_SHARED_INDEX_LOCK", "absent"),
            ("PUSH_MARKER", push_marker.to_str().unwrap()),
            ("REAL_GIT", real_git.to_str().unwrap()),
        ],
    );
    assert_validated(&harness, &transaction, &validated);
    assert_eq!(fs::read_to_string(&validation_log).unwrap(), "ran\n");
    assert!(!hook_marker.exists());
    assert!(!signing_marker.exists());
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "base");
    assert_eq!(harness.read("intended.txt"), "live content after prepare\n");

    harness.git(["config", "commit.gpgsign", "false"]);
    harness.success_with_env(
        ["commit", &transaction, "-m", "test: revalidate after preflight"],
        [
            ("VALIDATION_LOG", validation_log_text.as_str()),
            ("HOOK_MARKER", hook_marker_text.as_str()),
            ("EXPECTED_SHARED_INDEX_LOCK", "owned"),
            ("PUSH_MARKER", push_marker.to_str().unwrap()),
            ("REAL_GIT", real_git.to_str().unwrap()),
        ],
    );
    assert_eq!(fs::read_to_string(validation_log).unwrap(), "ran\nran\n");
    assert!(hook_marker.exists());
    assert!(!push_marker.exists());
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "prepared");
    assert!(validator.exists());
}

#[test]
fn validation_skips_without_a_command_and_uses_the_frozen_argv() {
    let skipped = Harness::new("validate-skipped");
    skipped.write("intended.txt", "base\n");
    skipped.commit_all("base");
    skipped.write("intended.txt", "prepared\n");
    let (transaction, _) = skipped.prepare(&["intended.txt"]);
    let output = skipped.success(["validate", &transaction]);
    assert_eq!(stdout(&output), format!("VALIDATION_SKIPPED {transaction} no-configured-command\n"));
    assert!(stderr(&output).contains("no validation was performed"));

    let frozen = Harness::new("validate-frozen-argv");
    frozen.write("intended.txt", "base\n");
    frozen.commit_all("base");
    frozen.write("intended.txt", "prepared\n");
    let validator = configure_validator(&frozen, "#!/bin/sh\nset -eu\n: > \"$VALIDATION_MARKER\"\n");
    let (transaction, _) = frozen.prepare(&["intended.txt"]);
    frozen.write(".agents/commit.toml", "[message]\nformat = \"conventional\"\n");
    let marker = frozen.root.join("frozen-validator-ran");
    let marker_text = marker.to_string_lossy().into_owned();
    let output = frozen.success_with_env(["validate", &transaction], [("VALIDATION_MARKER", &marker_text)]);
    assert_validated(&frozen, &transaction, &output);
    assert!(marker.exists());
    assert!(validator.exists());
}

#[test]
fn validation_preserves_the_existing_snapshot_environment_and_ignored_dependencies() {
    let harness = Harness::new("validate-environment");
    harness.write(".gitignore", "node_modules/\n");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("node_modules/@example/tool/marker.txt", "dependency\n");
    harness.write("intended.txt", "prepared\n");
    configure_validator(
        &harness,
        "#!/bin/sh\nset -eu\ntest \"$(cat intended.txt)\" = prepared\ntest \"$(cat node_modules/@example/tool/marker.txt)\" = dependency\ntest \"$AI_COMMIT_VALIDATION_MODE\" = prepared-tree\ntest \"$AI_COMMIT_ORIGINAL_WORKTREE\" = \"$EXPECTED_ORIGINAL\"\ntest \"$PWD\" != \"$AI_COMMIT_ORIGINAL_WORKTREE\"\ntest \"${GIT_INDEX_FILE##*.lock}\" = \"$GIT_INDEX_FILE\"\ntest -z \"${AI_COMMIT_HOOK_MODE:-}\"\ntest -z \"${GIT_PREFIX:-}\"\n",
    );
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    let original = harness.repo.canonicalize().unwrap().to_string_lossy().into_owned();
    fs::write(harness.repo.join(".git/index.lock"), "peer-owned lock\n").unwrap();

    let output = harness.success_with_env(
        ["validate", &transaction],
        [
            ("EXPECTED_ORIGINAL", original.as_str()),
            ("AI_COMMIT_VALIDATION_MODE", "inherited"),
            ("AI_COMMIT_ORIGINAL_WORKTREE", "inherited"),
            ("AI_COMMIT_HOOK_MODE", "inherited"),
            ("GIT_PREFIX", "inherited/"),
        ],
    );

    assert_validated(&harness, &transaction, &output);
    assert_eq!(harness.read("node_modules/@example/tool/marker.txt"), "dependency\n");
    assert_eq!(harness.read(".git/index.lock"), "peer-owned lock\n");
}

#[test]
fn validation_serializes_the_same_transaction() {
    let harness = Harness::new("validate-transaction-lock");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "prepared\n");
    configure_validator(
        &harness,
        "#!/bin/sh\nset -eu\nif env -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE \"$VALIDATE_BINARY\" validate \"$TRANSACTION_ID\" 2> nested-error; then\n  exit 91\nelse\n  test \"$?\" -eq 3\nfi\ngrep -Fq 'transaction is already in use' nested-error\n",
    );
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    let journal_before = fs::read(harness.transaction_json(&transaction)).unwrap();
    let output = harness.success_with_env(
        ["validate", &transaction],
        [("VALIDATE_BINARY", env!("CARGO_BIN_EXE_ai-commit")), ("TRANSACTION_ID", &transaction)],
    );
    assert_validated(&harness, &transaction, &output);
    assert_eq!(fs::read(harness.transaction_json(&transaction)).unwrap(), journal_before);
}

#[test]
fn validation_supports_explicit_all_staged_and_baseline_preparations() {
    let explicit = Harness::new("validate-explicit");
    explicit.write("one.txt", "base\n");
    explicit.write("two.txt", "base\n");
    explicit.commit_all("base");
    explicit.write("one.txt", "prepared\n");
    configure_validator(
        &explicit,
        "#!/bin/sh\nset -eu\ntest \"$(cat one.txt)\" = prepared\ntest \"$(cat two.txt)\" = base\n",
    );
    let (transaction, _) = explicit.prepare(&["one.txt"]);
    assert_validated(&explicit, &transaction, &explicit.success(["validate", &transaction]));

    let all = Harness::new("validate-all");
    all.write("one.txt", "base\n");
    all.commit_all("base");
    all.write("one.txt", "changed\n");
    all.write("two.txt", "untracked\n");
    configure_validator(
        &all,
        "#!/bin/sh\nset -eu\ntest \"$(cat one.txt)\" = changed\ntest \"$(cat two.txt)\" = untracked\n",
    );
    let prepared = all.success(["prepare", "--all", "--porcelain"]);
    let transaction = prepared_id(&stdout(&prepared));
    assert_validated(&all, &transaction, &all.success(["validate", &transaction]));

    let staged = Harness::new("validate-staged");
    staged.write("one.txt", "base\n");
    staged.commit_all("base");
    staged.write("one.txt", "staged snapshot\n");
    staged.git(["add", "one.txt"]);
    staged.write("one.txt", "later worktree\n");
    configure_validator(&staged, "#!/bin/sh\nset -eu\ntest \"$(cat one.txt)\" = 'staged snapshot'\n");
    let prepared = staged.success(["prepare", "--staged", "--porcelain"]);
    let transaction = prepared_id(&stdout(&prepared));
    assert_validated(&staged, &transaction, &staged.success(["validate", &transaction]));

    let baseline = Harness::new("validate-baseline");
    let base = "line 01\nline 02 original\nline 03\nline 04\nline 05\nline 06 original\nline 07\nline 08\nline 09\nline 10\nline 11 original\nline 12\nline 13\nline 14\n";
    let stale_baseline = "line 01\nline 02 stray\nline 03\nline 04\nline 05\nline 06 original\nline 07\nline 08\nline 09\nline 10\nline 11 original\nline 12\nline 13\nline 14\n";
    let worktree = "line 01\nline 02 stray\nline 03\nline 04\nline 05\nline 06 original\nline 07\nline 08\nline 09\nline 10\nline 11 owned\nline 12\nline 13\nline 14\n";
    baseline.write("intended.txt", base);
    baseline.write("peer.txt", "peer base\n");
    baseline.commit_all("base");
    baseline.write("intended.txt", stale_baseline);
    let baseline_oid = baseline.git(["hash-object", "-w", "intended.txt"]);
    baseline.write("intended.txt", worktree);
    baseline.write("peer.txt", "peer staged\n");
    baseline.git(["add", "peer.txt"]);
    let peer_before = baseline.git(["rev-parse", ":peer.txt"]);
    let index_before = baseline.git(["hash-object", ".git/index"]);
    configure_validator(
        &baseline,
        "#!/bin/sh\nset -eu\ntest \"$(sed -n '2p' intended.txt)\" = 'line 02 original'\ntest \"$(sed -n '11p' intended.txt)\" = 'line 11 owned'\n",
    );
    let specification = format!("intended.txt={baseline_oid}");
    let prepared =
        baseline.success(["prepare", "--porcelain", "--exclude-baseline", &specification, "--", "intended.txt"]);
    let transaction = prepared_id(&stdout(&prepared));
    assert_validated(&baseline, &transaction, &baseline.success(["validate", &transaction]));
    assert_eq!(baseline.read("intended.txt"), worktree);
    assert_eq!(baseline.git(["rev-parse", ":peer.txt"]), peer_before);
    assert_eq!(baseline.git(["hash-object", ".git/index"]), index_before);
}

#[test]
fn validator_drift_is_rejected_for_success_and_failure_and_snapshots_are_removed() {
    for (name, source, exit, reports_paths) in [
        ("tracked-zero", "printf 'validator mutation\\n' > intended.txt", 0, true),
        ("tracked-nonzero", "printf 'validator mutation\\n' > intended.txt", 37, true),
        (
            "private-index-zero",
            "blob=$(printf 'validator index mutation\\n' | git hash-object -w --stdin)\ngit update-index --add --cacheinfo 100644,$blob,intended.txt",
            0,
            true,
        ),
        (
            "private-index-nonzero",
            "blob=$(printf 'validator index mutation\\n' | git hash-object -w --stdin)\ngit update-index --add --cacheinfo 100644,$blob,intended.txt",
            37,
            true,
        ),
        ("corrupt-private-index-nonzero", ": > \"$GIT_INDEX_FILE\"", 37, false),
    ] {
        let harness = Harness::new(&format!("validate-drift-{name}"));
        harness.write("intended.txt", "base\n");
        harness.commit_all("base");
        harness.write("intended.txt", "prepared\n");
        let snapshot_log = harness.root.join("snapshot-path");
        configure_validator(
            &harness,
            &format!("#!/bin/sh\nset -eu\n{source}\nprintf '%s\\n' \"$PWD\" > \"$SNAPSHOT_LOG\"\nexit {exit}\n"),
        );
        let (transaction, _) = harness.prepare(&["intended.txt"]);
        let head_before = harness.git(["rev-parse", "HEAD"]);
        let index_before = harness.git(["hash-object", ".git/index"]);
        let journal_before = fs::read(harness.transaction_json(&transaction)).unwrap();
        let snapshot_log_text = snapshot_log.to_string_lossy().into_owned();

        let failed = harness.command_with_env(["validate", &transaction], [("SNAPSHOT_LOG", &snapshot_log_text)]);

        assert_eq!(exit_code(&failed), 1, "{name}: {}", stderr(&failed));
        let diagnostic = stderr(&failed);
        if reports_paths {
            assert!(
                diagnostic.contains("prepared validation modified tracked or staged content: intended.txt"),
                "{name}: {diagnostic}"
            );
        } else {
            assert!(diagnostic.contains("drift inspection also failed"), "{name}: {diagnostic}");
        }
        if exit != 0 {
            assert!(diagnostic.contains("prepared validation failed"), "{name}: {diagnostic}");
        }
        assert_eq!(harness.git(["rev-parse", "HEAD"]), head_before, "{name}");
        assert_eq!(harness.git(["hash-object", ".git/index"]), index_before, "{name}");
        assert_eq!(harness.read("intended.txt"), "prepared\n", "{name}");
        assert_eq!(fs::read(harness.transaction_json(&transaction)).unwrap(), journal_before, "{name}");
        let snapshot = fs::read_to_string(snapshot_log).unwrap();
        assert!(!std::path::Path::new(snapshot.trim()).exists(), "{name}: snapshot was retained");
    }
}

#[test]
fn validation_includes_clean_head_movement_and_rejects_conflicts_and_mid_run_movement() {
    let clean = Harness::new("validate-clean-movement");
    clean.write("intended.txt", "base\n");
    clean.commit_all("base");
    clean.write("intended.txt", "prepared\n");
    configure_validator(
        &clean,
        "#!/bin/sh\nset -eu\ntest \"$(cat intended.txt)\" = prepared\ntest \"$(cat peer.txt)\" = moved\n",
    );
    let (transaction, _) = clean.prepare(&["intended.txt"]);
    clean.write("peer.txt", "moved\n");
    clean.git(["add", "peer.txt"]);
    clean.git(["commit", "--quiet", "-m", "move peer"]);
    let output = clean.success(["validate", &transaction]);
    let candidate = validated_tree(&transaction, &output);
    assert_eq!(clean.git(["show", &format!("{candidate}:intended.txt")]), "prepared");
    assert_eq!(clean.git(["show", &format!("{candidate}:peer.txt")]), "moved");

    let conflict = Harness::new("validate-conflicting-movement");
    conflict.write("intended.txt", "base\n");
    conflict.commit_all("base");
    conflict.write("intended.txt", "prepared\n");
    configure_validator(&conflict, "#!/bin/sh\nexit 0\n");
    let (transaction, _) = conflict.prepare(&["intended.txt"]);
    conflict.write("intended.txt", "conflicting head\n");
    conflict.git(["add", "intended.txt"]);
    conflict.git(["commit", "--quiet", "-m", "conflict"]);
    let failed = conflict.command(["validate", &transaction]);
    assert_eq!(exit_code(&failed), 3, "{}", stderr(&failed));
    assert!(stderr(&failed).contains("do not apply cleanly"), "{}", stderr(&failed));

    let moving = Harness::new("validate-mid-run-movement");
    moving.write("intended.txt", "base\n");
    moving.commit_all("base");
    moving.write("intended.txt", "prepared\n");
    configure_validator(
        &moving,
        "#!/bin/sh\nset -eu\nenv -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE git -C \"$AI_COMMIT_ORIGINAL_WORKTREE\" commit --allow-empty --quiet -m 'move during validation'\n",
    );
    let (transaction, _) = moving.prepare(&["intended.txt"]);
    let failed = moving.command(["validate", &transaction]);
    assert_eq!(exit_code(&failed), 3, "{}", stderr(&failed));
    assert!(!stdout(&failed).contains("VALIDATED"), "{}", stdout(&failed));
    assert!(stderr(&failed).contains("changed"), "{}", stderr(&failed));

    // The validator fixture moves HEAD before returning its failure, without timing-dependent sleeps.
    configure_validator(
        &moving,
        "#!/bin/sh\nset -eu\nenv -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE git -C \"$AI_COMMIT_ORIGINAL_WORKTREE\" commit --allow-empty --quiet -m 'move before validation failure'\nprintf 'validator mutation\\n' > intended.txt\nexit 31\n",
    );
    let journal_before = fs::read(moving.transaction_json(&transaction)).unwrap();
    let failed = moving.command(["validate", &transaction]);
    assert_eq!(exit_code(&failed), 3, "{}", stderr(&failed));
    assert!(!stdout(&failed).contains("VALIDATED"));
    assert!(stderr(&failed).contains("branch or HEAD changed during validation"));
    assert!(stderr(&failed).contains("prepared validation failed with exit status: 31"));
    assert!(stderr(&failed).contains("prepared validation modified tracked or staged content: intended.txt"));
    assert_eq!(fs::read(moving.transaction_json(&transaction)).unwrap(), journal_before);
    configure_validator(&moving, "#!/bin/sh\nexit 0\n");
    assert_validated(&moving, &transaction, &moving.success(["validate", &transaction]));

    moving.git(["branch", "validation-peer"]);
    configure_validator(
        &moving,
        "#!/bin/sh\nset -eu\nenv -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE git -C \"$AI_COMMIT_ORIGINAL_WORKTREE\" symbolic-ref HEAD refs/heads/validation-peer\n",
    );
    let failed = moving.command(["validate", &transaction]);
    assert_eq!(exit_code(&failed), 3, "{}", stderr(&failed));
    assert!(!stdout(&failed).contains("VALIDATED"));

    let detached = Harness::new("validate-mid-run-detached");
    detached.write("intended.txt", "base\n");
    detached.commit_all("base");
    detached.write("intended.txt", "prepared\n");
    configure_validator(
        &detached,
        "#!/bin/sh\nset -eu\nenv -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE git -C \"$AI_COMMIT_ORIGINAL_WORKTREE\" checkout --detach --quiet\n",
    );
    let (transaction, _) = detached.prepare(&["intended.txt"]);
    let failed = detached.command(["validate", &transaction]);
    assert_eq!(exit_code(&failed), 3, "{}", stderr(&failed));
    assert!(!stdout(&failed).contains("VALIDATED"), "{}", stdout(&failed));
    assert!(stderr(&failed).contains("changed"), "{}", stderr(&failed));
}

#[test]
fn validation_handles_unborn_and_terminal_or_pending_transactions_without_mutation() {
    let unborn = Harness::new("validate-unborn");
    unborn.write("initial.txt", "initial\n");
    configure_validator(&unborn, "#!/bin/sh\nset -eu\ntest \"$(cat initial.txt)\" = initial\n");
    let (transaction, _) = unborn.prepare(&["initial.txt"]);
    let output = unborn.success(["validate", &transaction]);
    assert_validated(&unborn, &transaction, &output);
    assert!(!unborn.git_output(["rev-parse", "--verify", "HEAD"]).status.success());
    assert!(!unborn.repo.join(".git/index.lock").exists());

    let terminal = Harness::new("validate-terminal");
    terminal.write("intended.txt", "base\n");
    terminal.commit_all("base");
    terminal.write("intended.txt", "prepared\n");
    let (discarded, _) = terminal.prepare(&["intended.txt"]);
    terminal.success(["discard", &discarded]);
    let rejected = terminal.command(["validate", &discarded]);
    assert_eq!(exit_code(&rejected), 2, "{}", stderr(&rejected));
    assert!(stderr(&rejected).contains("discarded"), "{}", stderr(&rejected));

    terminal.write("intended.txt", "committed\n");
    let (expired, _) = terminal.prepare(&["intended.txt"]);
    terminal.success(["commit", &expired, "-m", "test: expired receipt remains visible"]);
    let journal_path = terminal.transaction_json(&expired);
    let mut journal: serde_json::Value = serde_json::from_slice(&fs::read(&journal_path).unwrap()).unwrap();
    journal["terminal_at"] = 0.into();
    fs::write(&journal_path, serde_json::to_vec_pretty(&journal).unwrap()).unwrap();
    let journal_before = fs::read(&journal_path).unwrap();
    let expired_rejected = terminal.command(["validate", &expired]);
    assert_eq!(exit_code(&expired_rejected), 2, "{}", stderr(&expired_rejected));
    assert!(stderr(&expired_rejected).contains("already created a commit"), "{}", stderr(&expired_rejected));
    assert_eq!(fs::read(&journal_path).unwrap(), journal_before);
    let refs_before = transaction_refs(&terminal, &expired);
    journal["status"] = "pushed".into();
    fs::write(&journal_path, serde_json::to_vec_pretty(&journal).unwrap()).unwrap();
    let pushed_before = fs::read(&journal_path).unwrap();
    let pushed_rejected = terminal.command(["validate", &expired]);
    assert_eq!(exit_code(&pushed_rejected), 2, "{}", stderr(&pushed_rejected));
    assert!(stderr(&pushed_rejected).contains("already pushed"));
    assert_eq!(fs::read(&journal_path).unwrap(), pushed_before);
    assert_eq!(transaction_refs(&terminal, &expired), refs_before);

    terminal.write("intended.txt", "prepared again\n");
    let (pending, _) = terminal.prepare(&["intended.txt"]);
    let interrupted = terminal.command_with_env(
        ["commit", &pending, "-m", "test: leave pending"],
        [("AI_COMMIT_TEST_FAIL_AFTER_REF_UPDATE", "1")],
    );
    assert_eq!(exit_code(&interrupted), 3, "{}", stderr(&interrupted));
    let journal_before = fs::read(terminal.transaction_json(&pending)).unwrap();
    let pending_oid: serde_json::Value = serde_json::from_slice(&journal_before).unwrap();
    let pending_oid = pending_oid["pending_commit"]["commit_oid"].as_str().unwrap().to_owned();
    let shown = terminal.success(["show", &pending]);
    assert!(stdout(&shown).ends_with(&format!("pending-commit\t{pending_oid}\n")));
    let blocked = terminal.command(["validate", &pending]);
    assert_eq!(exit_code(&blocked), 3, "{}", stderr(&blocked));
    assert!(stderr(&blocked).contains(&format!("commit {pending}")), "{}", stderr(&blocked));
    assert_eq!(fs::read(terminal.transaction_json(&pending)).unwrap(), journal_before);
}

#[test]
fn snapshot_hook_failure_reports_drift_and_precommit_recovery_together() {
    let harness = Harness::new("snapshot-hook-failure-and-drift");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "prepared\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    harness.write("intended.txt", "later live content\n");
    write_executable(
        &harness.repo.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nset -eu\nprintf 'hook mutation\\n' > intended.txt\nprintf 'synthetic hook failure\\n' >&2\nexit 33\n",
    );
    let head_before = harness.git(["rev-parse", "HEAD"]);
    let index_before = fs::read(harness.repo.join(".git/index")).unwrap();
    let failed = harness.command(["commit", &transaction, "-m", "test: combined snapshot failure"]);
    assert_eq!(exit_code(&failed), 1, "{}", stderr(&failed));
    let diagnostic = stderr(&failed);
    assert!(diagnostic.contains("synthetic hook failure"));
    assert!(diagnostic.contains("snapshot-check hook modified prepared content: intended.txt"));
    assert!(diagnostic.contains("this attempt created no commit"));
    assert!(diagnostic.contains(&format!("ai-commit discard {transaction}")));
    assert!(diagnostic.contains("transient dependency or environment"));
    assert_eq!(harness.git(["rev-parse", "HEAD"]), head_before);
    assert_eq!(fs::read(harness.repo.join(".git/index")).unwrap(), index_before);
    assert_eq!(harness.read("intended.txt"), "later live content\n");
}

fn configure_validator(harness: &Harness, source: &str) -> std::path::PathBuf {
    let validator = harness.root.join("validator");
    write_executable(&validator, source);
    harness.write(
        ".agents/commit.toml",
        &format!("[message]\nformat = \"conventional\"\n[validation]\ncommand = [\"{}\"]\n", validator.display()),
    );
    validator
}

fn transaction_refs(harness: &Harness, transaction: &str) -> Vec<String> {
    ["transactions", "bases", "indexes"]
        .into_iter()
        .map(|namespace| harness.git(["rev-parse", &format!("refs/ai-commit/{namespace}/{transaction}")]))
        .collect()
}

fn assert_validated(harness: &Harness, transaction: &str, output: &std::process::Output) {
    let transaction_tree = harness.git(["rev-parse", &format!("refs/ai-commit/transactions/{transaction}")]);
    assert_eq!(stdout(output), format!("VALIDATED {transaction} {transaction_tree}\n"));
}

fn validated_tree(transaction: &str, output: &std::process::Output) -> String {
    let record = stdout(output);
    let fields: Vec<_> = record.trim_end().split(' ').collect();
    assert_eq!(fields.len(), 3, "unexpected validation record: {record}");
    assert_eq!(fields[0], "VALIDATED");
    assert_eq!(fields[1], transaction);
    fields[2].to_owned()
}

fn prepared_id(output: &str) -> String {
    output.lines().find_map(|line| line.strip_prefix("PREPARED\t")).expect("PREPARED record").to_owned()
}

fn git_binary() -> std::path::PathBuf {
    let output = std::process::Command::new("sh").args(["-c", "command -v git"]).output().unwrap();
    assert!(output.status.success());
    std::path::PathBuf::from(String::from_utf8(output.stdout).unwrap().trim())
}

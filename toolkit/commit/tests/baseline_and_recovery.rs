mod common;

use std::fs;

use common::{Harness, exit_code, stderr, stdout, write_executable};

const BASE: &str = "line 01\nline 02 original\nline 03\nline 04\nline 05\nline 06 original\nline 07\nline 08\nline 09\nline 10\nline 11 original\nline 12\nline 13\nline 14\n";
const BASELINE: &str = "line 01\nline 02 stray\nline 03\nline 04\nline 05\nline 06 original\nline 07\nline 08\nline 09\nline 10\nline 11 original\nline 12\nline 13\nline 14\n";
const WORKTREE: &str = "line 01\nline 02 stray\nline 03\nline 04\nline 05\nline 06 original\nline 07\nline 08\nline 09\nline 10\nline 11 agent\nline 12\nline 13\nline 14\n";

#[test]
fn baseline_exclusion_commits_only_baseline_to_worktree_delta() {
    let harness = Harness::new("baseline");
    harness.write("intended.txt", BASE);
    harness.commit_all("base");
    harness.write("intended.txt", BASELINE);
    let baseline_oid = harness.git(["hash-object", "-w", "intended.txt"]);
    harness.write("intended.txt", WORKTREE);
    let specification = format!("intended.txt={baseline_oid}");
    let output =
        harness.success(["prepare", "--porcelain", "--exclude-baseline", &specification, "--", "intended.txt"]);
    let transaction = prepared_id(&stdout(&output));
    harness.success(["commit", &transaction, "-m", "test: exclude baseline"]);
    let committed = harness.git(["show", "HEAD:intended.txt"]);
    assert!(committed.contains("line 02 original"));
    assert!(committed.contains("line 11 agent"));
    assert_eq!(harness.read("intended.txt"), WORKTREE);
    assert_eq!(harness.git(["status", "--short"]), " M intended.txt");
}

#[test]
fn baseline_exclusion_runs_strict_hook_against_snapshot() {
    let harness = Harness::new("baseline-snapshot-hook");
    harness.write("intended.txt", BASE);
    harness.write("sibling.txt", "available to hooks\n");
    harness.commit_all("base");
    harness.write("intended.txt", BASELINE);
    let baseline_oid = harness.git(["hash-object", "-w", "intended.txt"]);
    harness.write("intended.txt", WORKTREE);
    let hook_log = harness.root.join("snapshot-hook.log");
    write_executable(
        &harness.repo.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nset -eu\nstaged=$(git diff --cached --name-only)\nunstaged=$(git diff --name-only)\nfor file in $staged; do\n  case \"\n$unstaged\n\" in *\"\n$file\n\"*) printf 'partially staged files are unsafe in the shared worktree: %s\\n' \"$file\" >&2; exit 88;; esac\ndone\ntest -f sibling.txt\nprintf '%s\\t%s\\t%s\\t%s\\t%s\\n' \"$(pwd -P)\" \"${GIT_WORK_TREE:-}\" \"${GIT_INDEX_FILE:-}\" \"${AI_COMMIT_HOOK_MODE:-}\" \"${AI_COMMIT_ORIGINAL_WORKTREE:-}\" > \"$HOOK_LOG\"\n",
    );

    let specification = format!("intended.txt={baseline_oid}");
    let prepared =
        harness.success(["prepare", "--porcelain", "--exclude-baseline", &specification, "--", "intended.txt"]);
    let transaction = prepared_id(&stdout(&prepared));
    let hook_log_text = hook_log.to_string_lossy().into_owned();
    let committed = harness
        .command_with_env(["commit", &transaction, "-m", "test: strict snapshot hook"], [("HOOK_LOG", &hook_log_text)]);
    assert!(committed.status.success(), "{}", stderr(&committed));

    let fields: Vec<_> = fs::read_to_string(&hook_log).unwrap().trim_end().split('\t').map(str::to_owned).collect();
    assert_eq!(fields.len(), 5, "unexpected hook log: {fields:?}");
    assert_eq!(fields[0], fields[1]);
    assert!(!fields[2].is_empty());
    assert!(!fields[2].ends_with(".lock"));
    assert_eq!(fields[3], "snapshot-check");
    assert_eq!(fields[4], harness.repo.canonicalize().unwrap().to_string_lossy());
    let snapshot = std::path::Path::new(&fields[1]);
    assert!(snapshot.starts_with(harness.repo.join(".git").canonicalize().unwrap()));
    assert!(!snapshot.exists(), "snapshot worktree was not cleaned: {}", snapshot.display());

    let committed_file = harness.git(["show", "HEAD:intended.txt"]);
    assert!(committed_file.contains("line 02 original"));
    assert!(committed_file.contains("line 11 agent"));
    assert_eq!(harness.read("intended.txt"), WORKTREE);
    assert_eq!(harness.git(["status", "--short"]), " M intended.txt");
}

#[test]
fn snapshot_mode_selection_only_considers_intended_paths() {
    let unrelated = Harness::new("snapshot-unrelated-dirty");
    unrelated.write("intended.txt", "base\n");
    unrelated.write("unrelated.txt", "base\n");
    unrelated.commit_all("base");
    unrelated.write("intended.txt", "prepared\n");
    let (transaction, _) = unrelated.prepare(&["intended.txt"]);
    unrelated.write("unrelated.txt", "dirty later\n");
    let hook_log = unrelated.root.join("ordinary-hook.log");
    write_executable(
        &unrelated.repo.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nset -eu\nprintf '%s\\t%s\\n' \"${AI_COMMIT_HOOK_MODE:-}\" \"$(pwd -P)\" > \"$HOOK_LOG\"\n",
    );
    let hook_log_text = hook_log.to_string_lossy().into_owned();
    let output = unrelated
        .command_with_env(["commit", &transaction, "-m", "test: unrelated dirt"], [("HOOK_LOG", &hook_log_text)]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        fs::read_to_string(&hook_log).unwrap(),
        format!("\t{}\n", unrelated.repo.canonicalize().unwrap().display())
    );
    assert_eq!(unrelated.read("unrelated.txt"), "dirty later\n");

    let changed = Harness::new("snapshot-intended-changed");
    changed.write("intended.txt", "base\n");
    changed.commit_all("base");
    changed.write("intended.txt", "prepared\n");
    let (transaction, _) = changed.prepare(&["intended.txt"]);
    changed.write("intended.txt", "changed after prepare\n");
    let hook_log = changed.root.join("snapshot-hook.log");
    write_executable(
        &changed.repo.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nset -eu\nprintf '%s\\t%s\\n' \"${AI_COMMIT_HOOK_MODE:-}\" \"${GIT_WORK_TREE:-}\" > \"$HOOK_LOG\"\n",
    );
    let hook_log_text = hook_log.to_string_lossy().into_owned();
    let output = changed.command_with_env(
        ["commit", &transaction, "-m", "test: intended path changed"],
        [("HOOK_LOG", &hook_log_text)],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let log = fs::read_to_string(&hook_log).unwrap();
    let (mode, snapshot) = log.trim_end().split_once('\t').unwrap();
    assert_eq!(mode, "snapshot-check");
    assert_ne!(snapshot, changed.repo.canonicalize().unwrap().to_string_lossy());
    assert!(!std::path::Path::new(snapshot).exists());
    assert_eq!(changed.git(["show", "HEAD:intended.txt"]), "prepared");
    assert_eq!(changed.read("intended.txt"), "changed after prepare\n");
}

#[test]
fn snapshot_hook_content_drift_is_rejected_without_shared_state_changes() {
    let cases = [
        ("staged", "printf 'hook staged\\n' > intended.txt\ngit add -- intended.txt\n", "intended.txt"),
        ("added", "printf 'hook added\\n' > hook-added.txt\ngit add -- hook-added.txt\n", "hook-added.txt"),
        ("deleted", "rm -- intended.txt\ngit add -u -- intended.txt\n", "intended.txt"),
        ("unstaged", "printf 'hook unstaged\\n' > intended.txt\n", "intended.txt"),
    ];

    for (name, mutation, affected) in cases {
        let harness = Harness::new(&format!("snapshot-drift-{name}"));
        harness.write("intended.txt", BASE);
        harness.commit_all("base");
        harness.write("intended.txt", BASELINE);
        let baseline_oid = harness.git(["hash-object", "-w", "intended.txt"]);
        harness.write("intended.txt", WORKTREE);
        let specification = format!("intended.txt={baseline_oid}");
        let prepared =
            harness.success(["prepare", "--porcelain", "--exclude-baseline", &specification, "--", "intended.txt"]);
        let transaction = prepared_id(&stdout(&prepared));
        let hook_log = harness.root.join("snapshot-path.log");
        let hook = format!("#!/bin/sh\nset -eu\nprintf '%s\\n' \"$GIT_WORK_TREE\" > \"$HOOK_LOG\"\n{mutation}");
        write_executable(&harness.repo.join(".git/hooks/pre-commit"), &hook);
        let head_before = harness.git(["rev-parse", "HEAD"]);
        let index_before = harness.git(["hash-object", ".git/index"]);
        let status_before = harness.git(["status", "--short"]);
        let hook_log_text = hook_log.to_string_lossy().into_owned();

        let failed = harness.command_with_env(
            ["commit", &transaction, "-m", "test: reject snapshot drift"],
            [("HOOK_LOG", &hook_log_text)],
        );
        assert_eq!(exit_code(&failed), 1, "{name}: {}", stderr(&failed));
        let diagnostic = stderr(&failed);
        assert!(diagnostic.contains("snapshot-check hook modified prepared content"), "{name}: {diagnostic}");
        assert!(diagnostic.contains(affected), "{name}: {diagnostic}");
        assert!(diagnostic.contains("unchanged retry will repeat"), "{name}: {diagnostic}");
        assert!(diagnostic.contains(&format!("ai-commit discard {transaction}")), "{name}: {diagnostic}");
        assert!(diagnostic.contains("excluded baseline"), "{name}: {diagnostic}");
        assert!(diagnostic.contains("owner"), "{name}: {diagnostic}");
        assert!(stdout(&harness.success(["show", &transaction])).starts_with(&format!("PREPARED {transaction}\n")));
        assert_eq!(harness.git(["rev-parse", "HEAD"]), head_before, "{name}");
        assert_eq!(harness.git(["hash-object", ".git/index"]), index_before, "{name}");
        assert_eq!(harness.git(["status", "--short"]), status_before, "{name}");
        assert_eq!(harness.read("intended.txt"), WORKTREE, "{name}");
        assert!(!harness.repo.join("hook-added.txt").exists(), "{name}");
        let snapshot = fs::read_to_string(&hook_log).unwrap();
        assert!(!std::path::Path::new(snapshot.trim()).exists(), "{name}: snapshot was not cleaned");
    }
}

#[test]
fn snapshot_hook_failure_is_retryable_and_cleans_materialized_worktree() {
    let harness = Harness::new("snapshot-hook-failure");
    harness.write("intended.txt", BASE);
    harness.commit_all("base");
    harness.write("intended.txt", BASELINE);
    let baseline_oid = harness.git(["hash-object", "-w", "intended.txt"]);
    harness.write("intended.txt", WORKTREE);
    let specification = format!("intended.txt={baseline_oid}");
    let prepared =
        harness.success(["prepare", "--porcelain", "--exclude-baseline", &specification, "--", "intended.txt"]);
    let transaction = prepared_id(&stdout(&prepared));
    let hook_log = harness.root.join("failed-snapshot.log");
    write_executable(
        &harness.repo.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$GIT_WORK_TREE\" > \"$HOOK_LOG\"\nprintf 'intentional snapshot hook failure\\n' >&2\nexit 1\n",
    );
    let head_before = harness.git(["rev-parse", "HEAD"]);
    let index_before = harness.git(["hash-object", ".git/index"]);
    let hook_log_text = hook_log.to_string_lossy().into_owned();
    let failed = harness.command_with_env(
        ["commit", &transaction, "-m", "test: snapshot hook failure"],
        [("HOOK_LOG", &hook_log_text)],
    );
    assert_eq!(exit_code(&failed), 1);
    assert!(stderr(&failed).contains("intentional snapshot hook failure"), "{}", stderr(&failed));
    assert!(stdout(&harness.success(["show", &transaction])).starts_with(&format!("PREPARED {transaction}\n")));
    assert_eq!(harness.git(["rev-parse", "HEAD"]), head_before);
    assert_eq!(harness.git(["hash-object", ".git/index"]), index_before);
    assert_eq!(harness.read("intended.txt"), WORKTREE);
    let snapshot = fs::read_to_string(&hook_log).unwrap();
    assert!(!std::path::Path::new(snapshot.trim()).exists(), "snapshot was not cleaned");
}

#[test]
fn snapshot_materialization_failure_cleans_temporary_state_and_is_retryable() {
    let harness = Harness::new("snapshot-materialization-failure");
    harness.write("intended.txt", BASE);
    harness.commit_all("base");
    harness.write("intended.txt", BASELINE);
    let baseline_oid = harness.git(["hash-object", "-w", "intended.txt"]);
    harness.write("intended.txt", WORKTREE);
    let specification = format!("intended.txt={baseline_oid}");
    let prepared =
        harness.success(["prepare", "--porcelain", "--exclude-baseline", &specification, "--", "intended.txt"]);
    let transaction = prepared_id(&stdout(&prepared));
    let head_before = harness.git(["rev-parse", "HEAD"]);
    let index_before = harness.git(["hash-object", ".git/index"]);

    let failed = harness.command_with_env(
        ["commit", &transaction, "-m", "test: injected materialization failure"],
        [("AI_COMMIT_TEST_FAIL_SNAPSHOT_MATERIALIZATION", "1")],
    );
    assert_eq!(exit_code(&failed), 3);
    assert!(stderr(&failed).contains("snapshot"));
    assert!(stdout(&harness.success(["show", &transaction])).starts_with(&format!("PREPARED {transaction}\n")));
    assert_eq!(harness.git(["rev-parse", "HEAD"]), head_before);
    assert_eq!(harness.git(["hash-object", ".git/index"]), index_before);
    assert_eq!(harness.read("intended.txt"), WORKTREE);
    let snapshots: Vec<_> = fs::read_dir(harness.repo.join(".git"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| name.to_string_lossy().starts_with("ai-commit-hook-"))
        .collect();
    assert!(snapshots.is_empty(), "snapshot temporary state remained: {snapshots:?}");

    harness.success(["commit", &transaction, "-m", "test: retry materialization"]);
    let committed = harness.git(["show", "HEAD:intended.txt"]);
    assert!(committed.contains("line 02 original"));
    assert!(committed.contains("line 11 agent"));
    assert_eq!(harness.read("intended.txt"), WORKTREE);
}

#[test]
fn auto_baseline_is_applied_and_disclosed_in_both_output_modes() {
    let harness = Harness::new("auto-baseline");
    harness.write("intended.txt", BASE);
    harness.commit_all("base");
    harness.write("intended.txt", BASELINE);
    let baseline_oid = harness.git(["hash-object", "-w", "intended.txt"]);
    harness.write("intended.txt", WORKTREE);
    write_executable(
        &harness.shim.join("ai-coord"),
        &format!("#!/bin/sh\n[ \"$1\" = baseline ] && printf 'intended.txt\\t{baseline_oid}\\n'\n"),
    );

    let output = harness.success(["prepare", "--porcelain", "--", "intended.txt"]);
    assert!(stdout(&output).contains(&format!("AUTO_BASELINE\tintended.txt\t{baseline_oid}\n")));
    let transaction = prepared_id(&stdout(&output));
    harness.success(["commit", &transaction, "-m", "test: auto baseline"]);
    let committed = harness.git(["show", "HEAD:intended.txt"]);
    assert!(committed.contains("line 02 original"));
    assert!(committed.contains("line 11 agent"));

    harness.git(["reset", "--soft", "HEAD^"]);
    let output = harness.success(["prepare", "--", "intended.txt"]);
    assert!(stdout(&output).contains(&format!("## auto-applied baselines\nintended.txt={baseline_oid}\n")));
}

#[test]
fn explicit_baseline_wins_and_malformed_auto_records_are_ignored() {
    let harness = Harness::new("auto-precedence");
    harness.write("intended.txt", BASE);
    harness.commit_all("base");
    harness.write("intended.txt", BASELINE);
    let explicit_oid = harness.git(["hash-object", "-w", "intended.txt"]);
    harness.write("intended.txt", &BASE.replace("line 02 original", "line 02 other stray"));
    let auto_oid = harness.git(["hash-object", "-w", "intended.txt"]);
    harness.write("intended.txt", WORKTREE);
    write_executable(
        &harness.shim.join("ai-coord"),
        &format!(
            "#!/bin/sh\n[ \"$1\" = baseline ] && printf 'malformed\\nintended.txt\\t{auto_oid}\\nextra.txt\\tbad\\textra\\n'\n"
        ),
    );

    let specification = format!("intended.txt={explicit_oid}");
    let output =
        harness.success(["prepare", "--porcelain", "--exclude-baseline", &specification, "--", "intended.txt"]);
    assert!(!stdout(&output).contains("AUTO_BASELINE\t"));
    let transaction = prepared_id(&stdout(&output));
    harness.success(["commit", &transaction, "-m", "test: explicit precedence"]);
    let committed = harness.git(["show", "HEAD:intended.txt"]);
    assert!(committed.contains("line 02 original"));
    assert!(committed.contains("line 11 agent"));
}

#[test]
fn staged_and_opt_out_skip_auto_query_and_missing_binary_is_tolerated() {
    let harness = Harness::new("auto-skip");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");
    harness.git(["add", "intended.txt"]);
    let marker = harness.root.join("baseline-called");
    write_executable(
        &harness.shim.join("ai-coord"),
        "#!/bin/sh\n[ \"$1\" = baseline ] && : > \"$AI_COORD_MARKER\"\nexit 1\n",
    );
    let marker_text = marker.to_string_lossy().into_owned();

    let staged = harness.command_with_env(["prepare", "--staged", "--porcelain"], [("AI_COORD_MARKER", &marker_text)]);
    assert!(staged.status.success(), "{}", stderr(&staged));
    assert!(!marker.exists());
    let opted_out = harness.command_with_env(
        ["prepare", "--all", "--no-auto-baseline", "--porcelain"],
        [("AI_COORD_MARKER", &marker_text)],
    );
    assert!(opted_out.status.success(), "{}", stderr(&opted_out));
    assert!(!marker.exists());

    let missing = harness.command_with_env(["prepare", "--all", "--porcelain"], [("PATH", "/usr/bin:/bin")]);
    assert!(missing.status.success(), "{}", stderr(&missing));
}

#[test]
fn non_overlapping_head_movement_is_applied_and_conflict_is_safe() {
    let clean = Harness::new("head-disjoint");
    clean.write("intended.txt", BASE);
    clean.commit_all("base");
    clean.write("intended.txt", BASELINE);
    let baseline_oid = clean.git(["hash-object", "-w", "intended.txt"]);
    clean.write("intended.txt", WORKTREE);
    let specification = format!("intended.txt={baseline_oid}");
    let prepared =
        clean.success(["prepare", "--porcelain", "--exclude-baseline", &specification, "--", "intended.txt"]);
    let transaction = prepared_id(&stdout(&prepared));
    let moved = BASE.replace("line 06 original", "line 06 moved HEAD");
    let moved_head = advance_head(&clean, &moved, "move HEAD elsewhere");
    clean.success(["commit", &transaction, "-m", "test: apply to moved head"]);
    assert_eq!(clean.git(["rev-parse", "HEAD^"]), moved_head);
    let committed = clean.git(["show", "HEAD:intended.txt"]);
    assert!(committed.contains("line 06 moved HEAD"));
    assert!(committed.contains("line 11 agent"));

    let conflict = Harness::new("head-conflict");
    conflict.write("intended.txt", BASE);
    conflict.commit_all("base");
    conflict.write("intended.txt", BASELINE);
    let baseline_oid = conflict.git(["hash-object", "-w", "intended.txt"]);
    conflict.write("intended.txt", WORKTREE);
    let specification = format!("intended.txt={baseline_oid}");
    let prepared =
        conflict.success(["prepare", "--porcelain", "--exclude-baseline", &specification, "--", "intended.txt"]);
    let transaction = prepared_id(&stdout(&prepared));
    let moved = BASE.replace("line 11 original", "line 11 moved HEAD");
    let moved_head = advance_head(&conflict, &moved, "move HEAD into conflict");
    let status_before = conflict.git(["status", "--short"]);
    let index_before = conflict.git(["hash-object", ".git/index"]);
    let failed = conflict.command(["commit", &transaction, "-m", "test: conflict"]);
    assert_eq!(exit_code(&failed), 3);
    assert!(stderr(&failed).contains("do not apply cleanly"));
    assert_eq!(conflict.git(["rev-parse", "HEAD"]), moved_head);
    assert_eq!(conflict.git(["hash-object", ".git/index"]), index_before);
    assert_eq!(conflict.git(["status", "--short"]), status_before);
    assert_eq!(conflict.read("intended.txt"), WORKTREE);
    assert!(!conflict.repo.join(".git/index.lock").exists());
}

#[test]
fn malformed_out_of_scope_and_non_blob_baselines_are_rejected() {
    let harness = Harness::new("baseline-invalid");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");
    let invalid = harness.command(["prepare", "--exclude-baseline", "intended.txt=not-an-oid", "--", "intended.txt"]);
    assert_eq!(exit_code(&invalid), 2);
    assert!(stderr(&invalid).contains("invalid baseline blob OID"));

    let blob = harness.git(["rev-parse", "HEAD:intended.txt"]);
    let outside = format!("other.txt={blob}");
    let invalid = harness.command(["prepare", "--exclude-baseline", &outside, "--", "intended.txt"]);
    assert_eq!(exit_code(&invalid), 2);
    assert!(stderr(&invalid).contains("not among intended paths"));

    let tree = harness.git(["rev-parse", "HEAD^{tree}"]);
    let non_blob = format!("intended.txt={tree}");
    let invalid = harness.command(["prepare", "--exclude-baseline", &non_blob, "--", "intended.txt"]);
    assert_eq!(exit_code(&invalid), 2);
    assert!(stderr(&invalid).contains("not a blob"));
}

#[test]
fn post_ref_interruption_recovers_without_duplicate_commit() {
    let harness = Harness::new("recovery");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "prepared\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    let interrupted = harness.command_with_env(
        ["commit", &transaction, "-m", "test: recover journal"],
        [("AI_COMMIT_TEST_FAIL_AFTER_REF_UPDATE", "1")],
    );
    assert_eq!(exit_code(&interrupted), 3);
    assert!(stderr(&interrupted).contains("was created"));
    let created = harness.git(["rev-parse", "HEAD"]);
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "2");
    let discard_pending = harness.command(["discard", &transaction]);
    assert_eq!(exit_code(&discard_pending), 3);
    assert!(stderr(&discard_pending).contains("pending commit"));

    let journal: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(harness.transaction_json(&transaction)).unwrap()).unwrap();
    let token = journal["index_lock_token"].as_str().expect("persisted index lock token");
    fs::write(harness.repo.join(".git/index.lock"), format!("ai-commit-index-lock {transaction} {token}\n")).unwrap();

    harness.write("intended.txt", "later worktree\n");
    let recovered = harness.success(["commit", &transaction, "-m", "ignored retry message"]);
    assert!(stdout(&recovered).contains(&format!("COMMITTED {transaction} {created}")));
    assert_eq!(harness.git(["rev-parse", "HEAD"]), created);
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "2");
    assert_eq!(harness.git(["show", "HEAD:intended.txt"]), "prepared");
}

#[test]
fn recovery_after_descendant_commit_reconciles_index_to_current_head() {
    let harness = Harness::new("recovery-descendant");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "prepared\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    let interrupted = harness.command_with_env(
        ["commit", &transaction, "-m", "test: recover below descendant"],
        [("AI_COMMIT_TEST_FAIL_AFTER_REF_UPDATE", "1")],
    );
    assert_eq!(exit_code(&interrupted), 3);
    let created = harness.git(["rev-parse", "HEAD"]);
    let descendant = advance_head(&harness, "descendant\n", "advance after interrupted commit");

    let recovered = harness.success(["commit", &transaction, "-m", "ignored retry message"]);
    assert!(stdout(&recovered).contains(&format!("COMMITTED {transaction} {created}")));
    assert_eq!(harness.git(["rev-parse", "HEAD"]), descendant);
    assert_eq!(harness.git(["show", ":intended.txt"]), "descendant");
    assert_eq!(harness.git(["rev-list", "--count", "HEAD"]), "3");
}

#[test]
fn show_discard_and_receipt_retention_keep_prepared_transactions() {
    let harness = Harness::new("receipts");
    harness.write("one.txt", "base\n");
    harness.write("two.txt", "base\n");
    harness.write("three.txt", "base\n");
    harness.commit_all("base");
    harness.write("one.txt", "changed\n");
    harness.write("two.txt", "changed\n");
    harness.write("three.txt", "changed\n");
    let (discarded, _) = harness.prepare(&["one.txt"]);
    let (expiring, _) = harness.prepare(&["two.txt"]);
    let (persistent, _) = harness.prepare(&["three.txt"]);
    let shown = harness.success(["show", &discarded]);
    assert!(stdout(&shown).starts_with(&format!("PREPARED {discarded}\n")));
    harness.success(["discard", &discarded]);
    let replay = harness.success(["discard", &discarded]);
    assert_eq!(stdout(&replay), format!("DISCARDED {discarded}\n"));
    let rejected_commit = harness.command(["commit", &discarded, "-m", "must not commit"]);
    assert_eq!(exit_code(&rejected_commit), 2);
    let shown = harness.success(["show", &discarded]);
    assert!(stdout(&shown).starts_with(&format!("DISCARDED {discarded}\n")));
    let reference = format!("refs/ai-commit/transactions/{discarded}");
    assert!(!harness.git_output(["show-ref", "--verify", "--quiet", &reference]).status.success());

    harness.success(["commit", &expiring, "-m", "test: expiring receipt"]);
    let path = harness.transaction_json(&expiring);
    let mut receipt: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    receipt["terminal_at"] = 0.into();
    fs::write(&path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    let expired = harness.command(["show", &expiring]);
    assert_eq!(exit_code(&expired), 2);
    assert!(stderr(&expired).contains("unknown transaction"));
    for namespace in ["transactions", "bases", "indexes"] {
        let reference = format!("refs/ai-commit/{namespace}/{expiring}");
        assert!(
            !harness.git_output(["show-ref", "--verify", "--quiet", &reference]).status.success(),
            "expired ref remained: {reference}"
        );
    }
    let persistent_show = harness.success(["show", &persistent]);
    assert!(stdout(&persistent_show).starts_with(&format!("PREPARED {persistent}\n")));
}

fn prepared_id(output: &str) -> String {
    output.lines().find_map(|line| line.strip_prefix("PREPARED\t")).expect("PREPARED record").to_owned()
}

fn advance_head(harness: &Harness, contents: &str, message: &str) -> String {
    let blob = harness.git_input(["hash-object", "-w", "--stdin"], contents.as_bytes());
    let tree_input = format!("100644 blob {blob}\tintended.txt\n");
    let tree = harness.git_input(["mktree"], tree_input.as_bytes());
    let parent = harness.git(["rev-parse", "HEAD"]);
    let commit = harness.git(["commit-tree", &tree, "-p", &parent, "-m", message]);
    harness.git(["update-ref", "HEAD", &commit, &parent]);
    commit
}

mod common;

use std::fs;

use common::{Harness, exit_code, stderr, stdout};

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

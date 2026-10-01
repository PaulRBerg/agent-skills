use std::{collections::BTreeSet, fs, path::Path};

use tempfile::Builder;

pub(crate) use crate::snapshot::ValidationSnapshot;
use crate::{
    cli::TransactionArgs,
    error::{AppError, Result},
    git::{Repository, copy_file, decode_nul_paths, git_error, literal_pathspec},
    prepare::PATH_BATCH_SIZE,
    state::{Store, Transaction, TransactionStatus},
};

pub(crate) struct Candidate {
    pub parent: Option<String>,
    pub base: String,
    pub tree: String,
}

pub fn run(args: TransactionArgs, store: &Store) -> Result<()> {
    Repository::ensure_default_index_env()?;
    let _transaction_lock = store.lock(&args.transaction_id)?;
    let transaction = store.load(&args.transaction_id)?;
    if transaction.pending_commit.is_some() {
        return Err(AppError::retry(format!(
            "transaction {} has a pending commit; retry `ai-commit commit {}` with the same message arguments for \
             same-ID recovery",
            transaction.id, transaction.id
        )));
    }
    match transaction.status {
        TransactionStatus::Prepared => {}
        TransactionStatus::Discarded => {
            return Err(AppError::usage(format!("terminal transaction {} was discarded", transaction.id)));
        }
        TransactionStatus::Committed => {
            return Err(AppError::usage(format!("terminal transaction {} already created a commit", transaction.id)));
        }
        TransactionStatus::Pushed => {
            return Err(AppError::usage(format!("terminal transaction {} was already pushed", transaction.id)));
        }
    }

    let repository = transaction_repository(&transaction)?;
    ensure_branch(&repository, &transaction.branch)?;
    repository.ensure_idle()?;
    let temporary = Builder::new().prefix("validate-").tempdir_in(store.temporary())?;
    let candidate_index = temporary.path().join("candidate-index");
    let candidate = build_candidate(&repository, &transaction, &candidate_index)?;

    let validation_result = if let Some(command) = transaction.validation_command.as_deref() {
        let snapshot = ValidationSnapshot::materialize(
            &repository,
            &candidate_index,
            &candidate.tree,
            &temporary.path().join("validation-index"),
        )?;
        run_configured(&repository, command, &snapshot, &candidate.tree, &transaction.id)
    } else {
        Ok(())
    };

    if let Err(mut movement_error) = ensure_candidate_is_current(&repository, &transaction, candidate.parent.as_deref())
    {
        if let Err(validation_error) = validation_result {
            movement_error.message = format!(
                "{}\nvalidation result for the checked candidate: {}",
                movement_error.message, validation_error.message
            );
        }
        return Err(movement_error);
    }
    validation_result?;
    if transaction.validation_command.is_some() {
        println!("VALIDATED {} {}", transaction.id, candidate.tree);
    } else {
        println!("VALIDATION_SKIPPED {} no-configured-command", transaction.id);
        eprintln!("no configured validation command; no validation was performed");
    }
    Ok(())
}

pub(crate) fn transaction_repository(transaction: &Transaction) -> Result<Repository> {
    let repository = Repository::from_root(&transaction.repository_root)?;
    if repository.root != transaction.repository_root {
        return Err(AppError::usage("transaction repository no longer resolves to its prepared physical root"));
    }
    Ok(repository)
}

pub(crate) fn ensure_branch(repository: &Repository, expected: &str) -> Result<()> {
    let current = repository.branch()?;
    if current != expected {
        return Err(AppError::retry(format!(
            "transaction was prepared on branch {expected}, but current branch is {current}"
        )));
    }
    Ok(())
}

pub(crate) fn build_candidate(
    repository: &Repository,
    transaction: &Transaction,
    candidate_index: &Path,
) -> Result<Candidate> {
    let parent = repository.head_oid()?;
    let base = match &parent {
        Some(head) => head.clone(),
        None => repository.empty_tree()?,
    };
    if base == transaction.base_head {
        repository.checked(["read-tree", &transaction.prepared_tree], Some(candidate_index))?;
    } else {
        repository.checked(["read-tree", &base], Some(candidate_index))?;
        apply_prepared_delta(repository, transaction, &base, candidate_index)?;
    }
    let tree = repository.text(["write-tree"], Some(candidate_index))?;
    Ok(Candidate { parent, base, tree })
}

pub(crate) fn intended_paths_differ_from_worktree(
    repository: &Repository,
    index: &Path,
    intended_paths: &[String],
    comparison_index: &Path,
) -> Result<bool> {
    let tree = repository.text(["write-tree"], Some(index))?;
    let entries = repository.tree_file_entries(&tree, intended_paths)?;
    for path in intended_paths {
        if entries.contains_key(path) {
            continue;
        }
        match fs::symlink_metadata(repository.root.join(path)) {
            Ok(_) => return Ok(true),
            Err(error) if matches!(error.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory) => {}
            Err(error) => {
                return Err(AppError::operational(format!(
                    "cannot compare prepared path {path} with the physical worktree: {error}"
                )));
            }
        }
    }
    copy_file(index, comparison_index)?;
    let capture_paths = intended_paths.iter().filter(|path| entries.contains_key(*path)).collect::<Vec<_>>();
    for batch in capture_paths.chunks(PATH_BATCH_SIZE) {
        let mut capture_arguments = vec!["add".to_owned(), "-A".to_owned(), "--".to_owned()];
        capture_arguments.extend(batch.iter().map(|path| literal_pathspec(path)));
        let captured = repository.raw_in_worktree(capture_arguments, Some(comparison_index), &repository.root)?;
        if !captured.status.success() {
            return Err(git_error(captured));
        }
    }
    let worktree_tree = repository.text(["write-tree"], Some(comparison_index))?;
    Ok(tree != worktree_tree)
}

pub(crate) fn run_configured(
    repository: &Repository,
    command: &[String],
    snapshot: &ValidationSnapshot,
    candidate_tree: &str,
    transaction_id: &str,
) -> Result<()> {
    let status = repository
        .run_prepared_validation(command, snapshot.validation_index(), snapshot.root())
        .map_err(|error| append_validation_recovery(error, transaction_id))?;
    let drift = snapshot_drift_paths(repository, snapshot.validation_index(), snapshot, candidate_tree);
    let failure = match (status.success(), drift) {
        (true, Ok(drift)) if drift.is_empty() => return Ok(()),
        (false, Ok(drift)) if drift.is_empty() => format!("prepared validation failed with {status}"),
        (true, Ok(drift)) => format!(
            "prepared validation modified tracked or staged content: {}; validation changes were not admitted",
            drift.join(", ")
        ),
        (false, Ok(drift)) => format!(
            "prepared validation failed with {status}\nprepared validation modified tracked or staged content: {}; \
             validation changes were not admitted",
            drift.join(", ")
        ),
        (false, Err(error)) => format!(
            "prepared validation failed with {status}\nprepared validation drift inspection also failed: {error}"
        ),
        (true, Err(error)) => format!("prepared validation drift inspection failed: {error}"),
    };
    Err(AppError::operational(format!("{failure}\n{}", validation_recovery_guidance(transaction_id))))
}

pub(crate) fn snapshot_drift_paths(
    repository: &Repository,
    index: &Path,
    snapshot: &ValidationSnapshot,
    candidate_tree: &str,
) -> Result<Vec<String>> {
    let current_tree = repository.text(["write-tree"], Some(index))?;
    // Refresh stat data first so rewriting identical bytes (`touch`, `sed -i`) is not drift.
    repository.checked_in_worktree(
        ["update-index", "-q", "--refresh"],
        Some(snapshot.validation_index()),
        snapshot.root(),
    )?;
    let mut paths = diff_paths(repository, candidate_tree, &current_tree)?.into_iter().collect::<BTreeSet<_>>();
    paths.extend(worktree_diff_paths(repository, snapshot.validation_index(), snapshot.root(), &[])?);
    Ok(paths.into_iter().collect())
}

pub(crate) fn finish_snapshot_hook(
    hook_error: Option<AppError>,
    drift: Result<Vec<String>>,
    transaction_id: &str,
) -> Result<()> {
    match (hook_error, drift) {
        (None, Ok(paths)) if paths.is_empty() => Ok(()),
        (None, Ok(paths)) => Err(snapshot_drift_error(transaction_id, &paths)),
        (Some(error), Ok(paths)) if paths.is_empty() => Err(append_snapshot_recovery(error, transaction_id)),
        (Some(mut error), Ok(paths)) => {
            error.message = format!(
                "{}\nsnapshot-check hook modified prepared content: {}; hook changes were not admitted\n{}",
                error.message,
                paths.join(", "),
                snapshot_recovery_guidance(transaction_id, true)
            );
            Err(error)
        }
        (Some(mut error), Err(drift_error)) => {
            error.message = format!(
                "{}\nsnapshot-check hook drift inspection also failed: {drift_error}\n{}",
                error.message,
                snapshot_recovery_guidance(transaction_id, false)
            );
            Err(error)
        }
        (None, Err(mut error)) => {
            error.message = format!("snapshot-check hook drift inspection failed: {}", error.message);
            Err(append_snapshot_recovery(error, transaction_id))
        }
    }
}

fn snapshot_drift_error(transaction_id: &str, paths: &[String]) -> AppError {
    AppError::operational(format!(
        "snapshot-check hook modified prepared content: {}; hook changes were not admitted\n{}",
        paths.join(", "),
        snapshot_recovery_guidance(transaction_id, true)
    ))
}

pub(crate) fn diff_paths(repository: &Repository, old: &str, new: &str) -> Result<Vec<String>> {
    let bytes = repository
        .bytes(["diff", "--no-ext-diff", "--no-textconv", "--name-only", "--no-renames", "-z", old, new, "--"], None)?;
    decode_nul_paths(&bytes)
}

fn transaction_is_current(repository: &Repository, transaction: &Transaction, parent: Option<&str>) -> Result<bool> {
    let branch = match repository.branch() {
        Ok(branch) => branch,
        Err(_) => return Ok(false),
    };
    Ok(branch == transaction.branch && repository.head_oid()?.as_deref() == parent)
}

fn ensure_candidate_is_current(repository: &Repository, transaction: &Transaction, parent: Option<&str>) -> Result<()> {
    if transaction_is_current(repository, transaction, parent)? {
        return Ok(());
    }
    Err(AppError::retry(format!(
        "branch or HEAD changed during validation; the checked candidate is no longer current; retry `ai-commit \
         validate {}`",
        transaction.id
    )))
}

fn apply_prepared_delta(
    repository: &Repository,
    transaction: &Transaction,
    current_base: &str,
    candidate_index: &Path,
) -> Result<()> {
    let patch = repository.bytes(
        [
            "diff",
            "--binary",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            &transaction.base_head,
            &transaction.prepared_tree,
            "--",
        ],
        None,
    )?;
    let output = repository.with_input(
        ["apply", "--cached", "--3way", "--whitespace=nowarn", "-"],
        &patch,
        Some(candidate_index),
    )?;
    if !output.status.success() {
        let detail = git_error(output).message;
        return Err(AppError::retry(format!(
            "prepared changes do not apply cleanly to current branch base {}: {detail}",
            short_oid(current_base)
        )));
    }
    Ok(())
}

fn worktree_diff_paths(
    repository: &Repository,
    index: &Path,
    worktree: &Path,
    paths: &[String],
) -> Result<Vec<String>> {
    let mut arguments = vec![
        "diff-files".to_owned(),
        "--name-only".to_owned(),
        "--no-renames".to_owned(),
        "-z".to_owned(),
        "--".to_owned(),
    ];
    arguments.extend(paths.iter().map(|path| literal_pathspec(path)));
    let bytes = repository.bytes_in_worktree(arguments, Some(index), worktree)?;
    decode_nul_paths(&bytes)
}

fn append_validation_recovery(mut error: AppError, transaction_id: &str) -> AppError {
    error.message = format!("{}\n{}", error.message, validation_recovery_guidance(transaction_id));
    error
}

fn append_snapshot_recovery(mut error: AppError, transaction_id: &str) -> AppError {
    error.message = format!("{}\n{}", error.message, snapshot_recovery_guidance(transaction_id, false));
    error
}

fn validation_recovery_guidance(transaction_id: &str) -> String {
    format!("configured validation stopped before commit creation; {}", recovery_actions(transaction_id))
}

fn snapshot_recovery_guidance(transaction_id: &str, drift_detected: bool) -> String {
    let drift = if drift_detected { "an unchanged retry will repeat the detected content drift. " } else { "" };
    format!(
        "the snapshot-check hook stopped before commit creation; {drift}{} If satisfying the hook would change \
         baseline-owned bytes, wait for or contact the owner instead.",
        recovery_actions(transaction_id)
    )
}

fn recovery_actions(transaction_id: &str) -> String {
    format!(
        "this attempt created no commit; immutable transaction {transaction_id} remains prepared and retryable. If \
         only a transient dependency or environment failure was repaired, retry validation or commit with the same \
         transaction ID. If worktree content or configuration must change, review the failure, preserve excluded \
         baseline bytes, run `ai-commit discard {transaction_id}` only after confirming the preparation is \
         uncommitted, then prepare the corrected explicit owned scope again."
    )
}

fn short_oid(oid: &str) -> &str {
    oid.get(..12).unwrap_or(oid)
}

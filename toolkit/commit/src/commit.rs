use std::{
    collections::BTreeSet,
    env,
    fs::{self, File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use rand::random;
use tempfile::Builder;

use crate::{
    cli::CommitArgs,
    error::{AppError, Result},
    git::{RefUpdate, Repository, copy_file, decode_nul_paths, git_error},
    push::{self, PushOutcome},
    state::{PendingCommit, Store, Transaction, TransactionStatus, now_seconds},
};

pub fn run(args: CommitArgs, store: &Store) -> Result<()> {
    validate_message_arguments(&args.messages)?;
    Repository::ensure_default_index_env()?;
    let _transaction_lock = store.lock(&args.transaction_id)?;
    let mut transaction = store.load(&args.transaction_id)?;
    match transaction.status {
        TransactionStatus::Discarded => {
            return Err(AppError::usage(format!("transaction {} was discarded", transaction.id)));
        }
        TransactionStatus::Pushed => {
            let oid = transaction.commit_oid.as_deref().unwrap_or("unknown");
            println!("PUSHED {} {oid}", transaction.id);
            return Ok(());
        }
        TransactionStatus::Prepared | TransactionStatus::Committed => {}
    }

    if transaction.status == TransactionStatus::Committed &&
        transaction.reconciled &&
        !args.push &&
        !transaction.push_requested
    {
        print_commit_receipt(&transaction);
        return Ok(());
    }

    let repository = Repository::from_root(&transaction.repository_root)?;
    if repository.root != transaction.repository_root {
        return Err(AppError::usage("transaction repository no longer resolves to its prepared physical root"));
    }

    if args.push && !transaction.push_requested {
        transaction.push_requested = true;
        store.save(&transaction)?;
    }

    ensure_branch(&repository, &transaction.branch)?;
    if transaction.status == TransactionStatus::Committed && transaction.reconciled {
        print_commit_receipt(&transaction);
        return maybe_push(&repository, &mut transaction, store, args.push);
    }

    repository.ensure_idle()?;
    let temporary = Builder::new().prefix("commit-").tempdir_in(store.temporary())?;
    if transaction.index_lock_token.is_none() {
        transaction.index_lock_token = Some(format!("{:016x}{:016x}", random::<u64>(), random::<u64>()));
        store.save(&transaction)?;
    }
    let lock_marker = format!(
        "ai-commit-index-lock {} {}\n",
        transaction.id,
        transaction.index_lock_token.as_deref().expect("index lock token")
    );
    let mut index_lock = IndexLock::acquire(&repository, &lock_marker)?;

    let needs_recovery = transaction.status == TransactionStatus::Committed || transaction.pending_commit.is_some();
    if needs_recovery &&
        recover_after_ref_update(&repository, &mut transaction, store, &mut index_lock, temporary.path())?
    {
        print_commit_receipt(&transaction);
        return maybe_push(&repository, &mut transaction, store, args.push);
    }

    let current_parent = repository.head_oid()?;
    let current_base = match &current_parent {
        Some(head) => head.clone(),
        None => repository.empty_tree()?,
    };
    let commit_index = temporary.path().join("commit-index");
    repository.checked(["read-tree", &current_base], Some(&commit_index))?;
    apply_prepared_delta(&repository, &transaction, &current_base, &commit_index)?;
    let before_hook_tree = repository.text(["write-tree"], Some(&commit_index))?;
    let message_file = temporary.path().join("commit-message");
    write_message(&message_file, &args.messages)?;

    if !args.no_verify {
        run_hook(&repository, &commit_index, "pre-commit", &[])?;
    }
    let message_path = message_file.to_string_lossy().into_owned();
    run_hook(&repository, &commit_index, "prepare-commit-msg", &[&message_path, "message"])?;
    if !args.no_verify {
        run_hook(&repository, &commit_index, "commit-msg", &[&message_path])?;
    }
    ensure_nonempty_message(&message_file)?;
    let commit_tree = repository.text(["write-tree"], Some(&commit_index))?;
    if commit_tree == current_base {
        return Err(AppError::operational("hooks left no changes to commit; transaction remains retryable"));
    }
    let final_paths = diff_paths(&repository, &current_base, &commit_tree)?;
    let hook_added = hook_added_paths(&repository, &before_hook_tree, &commit_tree, &transaction.paths)?;
    let commit_oid = create_commit(
        &repository,
        &commit_index,
        &commit_tree,
        current_parent.as_deref(),
        &message_file,
        args.no_gpg_sign,
    )?;

    transaction.pending_commit = Some(PendingCommit {
        commit_oid: commit_oid.clone(),
        commit_tree: commit_tree.clone(),
        hook_added,
        parent: current_parent.clone(),
    });
    store.save(&transaction)?;
    index_lock.ensure_owned()?;

    let branch_ref = format!("refs/heads/{}", transaction.branch);
    let transaction_ref = transaction.reference();
    let mut updates = Vec::with_capacity(2);
    match current_parent.as_deref() {
        Some(parent) => {
            updates.push(RefUpdate::Update { reference: &branch_ref, new_oid: &commit_oid, old_oid: parent })
        }
        None => updates.push(RefUpdate::Create { reference: &branch_ref, new_oid: &commit_oid }),
    }
    updates.push(RefUpdate::Update {
        reference: &transaction_ref,
        new_oid: &commit_oid,
        old_oid: &transaction.prepared_tree,
    });
    if let Err(error) = repository.update_refs(&updates) {
        transaction.pending_commit = None;
        store.save(&transaction)?;
        return Err(AppError::retry(format!(
            "branch or transaction ref moved before commit ref update; transaction remains retryable: {error}"
        )));
    }
    if env::var_os("AI_COMMIT_TEST_FAIL_AFTER_REF_UPDATE").is_some() {
        return Err(AppError::retry(format!(
            "commit {commit_oid} was created before an injected interruption; retry commit {}",
            transaction.id
        )));
    }

    mark_committed(&mut transaction, &commit_oid);
    store.save(&transaction).map_err(|error| {
        AppError::retry(format!(
            "commit {commit_oid} was created, but its receipt could not be advanced: {error}; retry commit {}",
            transaction.id
        ))
    })?;
    reconcile_shared_index(&repository, &transaction, &commit_tree, &final_paths, &mut index_lock, temporary.path())
        .map_err(|error| {
            AppError::retry(format!(
                "commit {commit_oid} was created, but shared-index reconciliation failed: {error}; retry commit {}",
                transaction.id
            ))
        })?;
    transaction.reconciled = true;
    transaction.hook_added =
        transaction.pending_commit.as_ref().map(|pending| pending.hook_added.clone()).unwrap_or_default();
    transaction.pending_commit = None;
    transaction.index_lock_token = None;
    transaction.terminal_at = Some(now_seconds());
    store.save(&transaction).map_err(|error| {
        AppError::retry(format!(
            "commit {commit_oid} was created and reconciled, but its terminal receipt could not be saved: {error}; retry commit {}",
            transaction.id
        ))
    })?;

    let _ = run_hook(&repository, &commit_index, "post-commit", &[]);
    drop(index_lock);
    print_commit_receipt(&transaction);
    maybe_push(&repository, &mut transaction, store, args.push)
}

fn recover_after_ref_update(
    repository: &Repository,
    transaction: &mut Transaction,
    store: &Store,
    index_lock: &mut IndexLock,
    temporary: &Path,
) -> Result<bool> {
    let pending = if let Some(pending) = transaction.pending_commit.clone() {
        pending
    } else if let Some(commit_oid) = transaction.commit_oid.clone() {
        let commit_tree = repository.text(["rev-parse", &format!("{commit_oid}^{{tree}}")], None)?;
        let parent = commit_parent(repository, &commit_oid)?;
        PendingCommit { commit_oid, commit_tree, hook_added: transaction.hook_added.clone(), parent }
    } else {
        return Ok(false);
    };
    let mut head = repository.head_oid()?;
    let exists_on_branch = match head.as_deref() {
        Some(head) => head == pending.commit_oid || is_ancestor(repository, &pending.commit_oid, head)?,
        None => false,
    };
    if !exists_on_branch {
        if head.as_deref() == pending.parent.as_deref() {
            let branch_ref = format!("refs/heads/{}", transaction.branch);
            let transaction_ref = transaction.reference();
            let mut updates = Vec::with_capacity(2);
            match pending.parent.as_deref() {
                Some(parent) => updates.push(RefUpdate::Update {
                    reference: &branch_ref,
                    new_oid: &pending.commit_oid,
                    old_oid: parent,
                }),
                None => updates.push(RefUpdate::Create { reference: &branch_ref, new_oid: &pending.commit_oid }),
            }
            updates.push(RefUpdate::Update {
                reference: &transaction_ref,
                new_oid: &pending.commit_oid,
                old_oid: &transaction.prepared_tree,
            });
            if repository.update_refs(&updates).is_err() {
                return Err(AppError::retry("pending commit could not be restored because the branch moved"));
            }
            head = Some(pending.commit_oid.clone());
        } else if transaction.status == TransactionStatus::Prepared {
            transaction.pending_commit = None;
            repository.checked(["update-ref", &transaction.reference(), &transaction.prepared_tree], None)?;
            store.save(transaction)?;
            return Ok(false);
        } else {
            return Err(AppError::retry(format!(
                "commit {} exists but is no longer reachable from branch {}; reconcile the branch before retrying",
                pending.commit_oid, transaction.branch
            )));
        }
    }

    repository.checked(["update-ref", &transaction.reference(), &pending.commit_oid], None)?;
    mark_committed(transaction, &pending.commit_oid);
    transaction.pending_commit = Some(pending.clone());
    store.save(transaction)?;
    if !transaction.reconciled {
        let parent_base = match pending.parent.as_deref() {
            Some(parent) => parent.to_owned(),
            None => repository.empty_tree()?,
        };
        let final_paths = diff_paths(repository, &parent_base, &pending.commit_tree)?;
        let head = head.as_deref().expect("pending commit was restored or is reachable");
        let head_tree = repository.text(["rev-parse", &format!("{head}^{{tree}}")], None)?;
        reconcile_shared_index(repository, transaction, &head_tree, &final_paths, index_lock, temporary).map_err(
            |error| {
                AppError::retry(format!(
                    "commit {} exists, but shared-index reconciliation still failed: {error}",
                    pending.commit_oid
                ))
            },
        )?;
        transaction.reconciled = true;
    }
    transaction.hook_added = pending.hook_added;
    transaction.pending_commit = None;
    transaction.index_lock_token = None;
    transaction.terminal_at = Some(now_seconds());
    store.save(transaction).map_err(|error| {
        AppError::retry(format!(
            "commit {} was recovered and reconciled, but its terminal receipt could not be saved: {error}; retry commit {}",
            pending.commit_oid, transaction.id
        ))
    })?;
    Ok(true)
}

fn mark_committed(transaction: &mut Transaction, commit_oid: &str) {
    transaction.status = TransactionStatus::Committed;
    transaction.commit_oid = Some(commit_oid.to_owned());
}

fn ensure_branch(repository: &Repository, expected: &str) -> Result<()> {
    let current = repository.branch()?;
    if current != expected {
        return Err(AppError::retry(format!(
            "transaction was prepared on branch {expected}, but current branch is {current}"
        )));
    }
    Ok(())
}

fn apply_prepared_delta(
    repository: &Repository,
    transaction: &Transaction,
    current_base: &str,
    commit_index: &Path,
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
        Some(commit_index),
    )?;
    if !output.status.success() {
        let detail = git_error(output).message;
        return Err(AppError::retry(format!(
            "prepared changes do not apply cleanly to current branch base {current_base}: {detail}"
        )));
    }
    Ok(())
}

fn write_message(path: &Path, messages: &[String]) -> Result<()> {
    let mut file = File::create(path)?;
    for (index, message) in messages.iter().enumerate() {
        if index > 0 {
            file.write_all(b"\n\n")?;
        }
        file.write_all(message.as_bytes())?;
    }
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

fn validate_message_arguments(messages: &[String]) -> Result<()> {
    if messages.is_empty() {
        return Err(AppError::usage("at least one -m/--message is required"));
    }
    for message in messages {
        if message.contains('\0') {
            return Err(AppError::usage("commit messages may not contain NUL bytes"));
        }
        if message.contains("\\n") {
            return Err(AppError::usage(
                "commit messages may not contain a literal \\n escape; use an actual newline within the -m value",
            ));
        }
    }
    Ok(())
}

fn ensure_nonempty_message(path: &Path) -> Result<()> {
    let message = fs::read_to_string(path)?;
    if message.trim().is_empty() {
        return Err(AppError::usage("commit message is empty after commit-msg hooks"));
    }
    Ok(())
}

fn run_hook(repository: &Repository, index: &Path, hook: &str, arguments: &[&str]) -> Result<()> {
    let mut command_arguments = vec!["hook", "run", "--ignore-missing", hook];
    if !arguments.is_empty() {
        command_arguments.push("--");
        command_arguments.extend_from_slice(arguments);
    }
    let output = repository.raw(command_arguments, Some(index))?;
    if output.status.success() { Ok(()) } else { Err(git_error(output)) }
}

fn create_commit(
    repository: &Repository,
    index: &Path,
    tree: &str,
    parent: Option<&str>,
    message_file: &Path,
    no_gpg_sign: bool,
) -> Result<String> {
    let mut arguments =
        vec!["commit-tree".to_owned(), tree.to_owned(), "-F".to_owned(), message_file.to_string_lossy().into_owned()];
    if let Some(parent) = parent {
        arguments.splice(2..2, ["-p".to_owned(), parent.to_owned()]);
    }
    if !no_gpg_sign && signing_enabled(repository)? {
        arguments.push("-S".to_owned());
    }
    repository.text(arguments, Some(index))
}

fn signing_enabled(repository: &Repository) -> Result<bool> {
    let output = repository.raw(["config", "--bool", "--get", "commit.gpgsign"], None)?;
    if output.status.success() {
        let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        return match value.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(AppError::usage(format!("invalid commit.gpgsign value: {value}"))),
        };
    }
    if output.status.code() == Some(1) { Ok(false) } else { Err(git_error(output)) }
}

fn hook_added_paths(
    repository: &Repository,
    before_hook_tree: &str,
    commit_tree: &str,
    intended: &[String],
) -> Result<Vec<String>> {
    let changed = diff_paths(repository, before_hook_tree, commit_tree)?;
    Ok(changed.into_iter().filter(|path| !intended.contains(path)).collect())
}

fn diff_paths(repository: &Repository, old: &str, new: &str) -> Result<Vec<String>> {
    let bytes = repository
        .bytes(["diff", "--no-ext-diff", "--no-textconv", "--name-only", "--no-renames", "-z", old, new, "--"], None)?;
    decode_nul_paths(&bytes)
}

fn is_ancestor(repository: &Repository, ancestor: &str, descendant: &str) -> Result<bool> {
    let output = repository.raw(["merge-base", "--is-ancestor", ancestor, descendant], None)?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(git_error(output)),
    }
}

fn commit_parent(repository: &Repository, commit: &str) -> Result<Option<String>> {
    let parents = repository.text(["rev-list", "--parents", "-n", "1", commit], None)?;
    let mut fields = parents.split_ascii_whitespace();
    let oid = fields.next().ok_or_else(|| AppError::operational("cannot parse commit parents"))?;
    if oid != commit {
        return Err(AppError::operational("cannot parse commit parents"));
    }
    let parent = fields.next().map(str::to_owned);
    if fields.next().is_some() {
        return Err(AppError::operational("prepared commit has more than one parent"));
    }
    Ok(parent)
}

fn reconcile_shared_index(
    repository: &Repository,
    transaction: &Transaction,
    commit_tree: &str,
    final_paths: &[String],
    index_lock: &mut IndexLock,
    temporary: &Path,
) -> Result<()> {
    let reconciliation_index = temporary.join("reconciliation-index");
    if index_lock.has_index {
        copy_file(&index_lock.index_path, &reconciliation_index)?;
    } else {
        repository.checked(["read-tree", commit_tree], Some(&reconciliation_index))?;
    }
    let current_index_tree = repository.text(["write-tree"], Some(&reconciliation_index))?;
    let current_entries = repository.tree_file_entries(&current_index_tree, final_paths)?;
    let prepared_entries = repository.tree_file_entries(&transaction.shared_index_tree, final_paths)?;
    let commit_entries = repository.tree_file_entries(commit_tree, final_paths)?;
    let mut eligible = BTreeSet::new();
    for path in final_paths {
        if current_entries.get(path) == prepared_entries.get(path) {
            eligible.insert(path.clone());
        }
    }
    let zero_oid = "0".repeat(current_index_tree.len());
    let mut index_info = Vec::new();
    for path in &eligible {
        index_info.extend_from_slice(format!("0 {zero_oid}\t{path}\0").as_bytes());
    }
    for path in &eligible {
        if let Some(entry) = commit_entries.get(path) {
            index_info.extend_from_slice(format!("{} {} {}\t{path}\0", entry.mode, entry.kind, entry.oid).as_bytes());
        }
    }
    if !index_info.is_empty() {
        let output =
            repository.with_input(["update-index", "-z", "--index-info"], &index_info, Some(&reconciliation_index))?;
        if !output.status.success() {
            return Err(git_error(output));
        }
    }
    index_lock.publish_from(&reconciliation_index)?;
    Ok(())
}

fn print_commit_receipt(transaction: &Transaction) {
    for path in &transaction.hook_added {
        println!("HOOK_ADDED {path}");
    }
    if let Some(oid) = &transaction.commit_oid {
        println!("COMMITTED {} {oid}", transaction.id);
    }
}

fn maybe_push(repository: &Repository, transaction: &mut Transaction, store: &Store, requested: bool) -> Result<()> {
    if !requested && !transaction.push_requested {
        return Ok(());
    }
    transaction.push_requested = true;
    transaction.terminal_at = None;
    store.save(transaction)?;
    match push::execute(repository)? {
        PushOutcome::Behind { branch, count } => {
            println!("BEHIND {branch} {count}");
            Err(AppError::retry(""))
        }
        outcome => {
            let outcome_name = match &outcome {
                PushOutcome::Pushed { branch } => format!("PUSHED {branch}"),
                PushOutcome::PushedNew { branch } => format!("PUSHED_NEW {branch}"),
                PushOutcome::Behind { .. } => unreachable!(),
            };
            transaction.status = TransactionStatus::Pushed;
            transaction.push_requested = false;
            transaction.terminal_at = Some(now_seconds());
            store.save(transaction).map_err(|error| {
                AppError::operational(format!(
                    "{outcome_name}, but transaction receipt {} could not be updated: {error}",
                    transaction.id
                ))
            })?;
            outcome.print();
            Ok(())
        }
    }
}

struct IndexLock {
    index_path: PathBuf,
    lock_path: PathBuf,
    has_index: bool,
    file: Option<File>,
}

impl IndexLock {
    fn acquire(repository: &Repository, marker: &str) -> Result<Self> {
        let index_path = repository.git_path("index")?;
        let mut lock_name = index_path.as_os_str().to_os_string();
        lock_name.push(".lock");
        let lock_path = PathBuf::from(lock_name);
        for attempt in 1..=5 {
            match OpenOptions::new().read(true).write(true).create_new(true).open(&lock_path) {
                Ok(mut file) => {
                    let has_index = index_path.is_file();
                    let preserved_permissions = if has_index {
                        fs::metadata(&index_path)
                            .and_then(|metadata| fs::set_permissions(&lock_path, metadata.permissions()))
                    } else {
                        Ok(())
                    };
                    if let Err(error) = preserved_permissions
                        .and_then(|()| file.write_all(marker.as_bytes()))
                        .and_then(|()| file.sync_all())
                    {
                        drop(file);
                        let _ = fs::remove_file(&lock_path);
                        return Err(AppError::operational(format!(
                            "cannot preserve default Git index permissions: {error}"
                        )));
                    }
                    return Ok(Self { index_path, lock_path, has_index, file: Some(file) });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && attempt < 5 => {
                    if fs::read(&lock_path).ok().as_deref() == Some(marker.as_bytes()) {
                        fs::remove_file(&lock_path).map_err(|remove_error| {
                            AppError::operational(format!(
                                "cannot recover this transaction's stale index lock {}: {remove_error}",
                                lock_path.display()
                            ))
                        })?;
                        continue;
                    }
                    thread::sleep(Duration::from_millis(200));
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    return Err(AppError::retry(format!(
                        "default Git index remains locked after {attempt} attempts: {}",
                        lock_path.display()
                    )));
                }
                Err(error) => {
                    return Err(AppError::operational(format!(
                        "cannot create default Git index lock {}: {error}",
                        lock_path.display()
                    )));
                }
            }
        }
        unreachable!()
    }

    fn publish_from(&mut self, source: &Path) -> Result<()> {
        self.ensure_owned()?;
        let bytes = fs::read(source)?;
        let file = self.file.as_mut().ok_or_else(|| AppError::operational("default Git index lock is not owned"))?;
        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&self.lock_path, &self.index_path)?;
        self.file = None;
        Ok(())
    }

    fn ensure_owned(&self) -> Result<()> {
        let file = self.file.as_ref().ok_or_else(|| AppError::operational("default Git index lock is not owned"))?;
        if !same_file(file, &self.lock_path) {
            return Err(AppError::retry(format!(
                "default Git index lock ownership changed: {}",
                self.lock_path.display()
            )));
        }
        Ok(())
    }
}

impl Drop for IndexLock {
    fn drop(&mut self) {
        if self.file.as_ref().is_some_and(|file| same_file(file, &self.lock_path)) {
            let _ = fs::remove_file(&self.lock_path);
        }
    }
}

#[cfg(unix)]
fn same_file(file: &File, path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;

    let Ok(held) = file.metadata() else {
        return false;
    };
    let Ok(current) = fs::metadata(path) else {
        return false;
    };
    held.dev() == current.dev() && held.ino() == current.ino()
}

#[cfg(not(unix))]
fn same_file(_file: &File, _path: &Path) -> bool {
    false
}

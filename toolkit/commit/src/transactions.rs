use std::{ffi::OsString, fs, path::PathBuf};

use crate::{
    error::{AppError, Result},
    git::Repository,
    state::{Store, TransactionStatus, now_seconds},
};

pub fn show(store: &Store, id: &str) -> Result<()> {
    let transaction = store.load(id)?;
    let outcome = match transaction.status {
        TransactionStatus::Prepared => "PREPARED",
        TransactionStatus::Committed => "COMMITTED",
        TransactionStatus::Pushed => "PUSHED",
        TransactionStatus::Discarded => "DISCARDED",
    };
    println!("{outcome} {}", transaction.id);
    println!("repository\t{}", transaction.repository_root.display());
    println!("branch\t{}", transaction.branch);
    println!("base-head\t{}", transaction.base_head);
    println!("unborn\t{}", transaction.unborn);
    println!("prepared-tree\t{}", transaction.prepared_tree);
    println!("message-format\t{}", transaction.message_format.label());
    if let Some(trailer) = transaction.trailer {
        println!("trailer\t{trailer}");
    }
    if let Some(oid) = transaction.commit_oid {
        println!("commit\t{oid}");
    }
    println!("reconciled\t{}", transaction.reconciled);
    for path in transaction.hook_added {
        println!("hook-added\t{path}");
    }
    for path in transaction.paths {
        println!("path\t{path}");
    }
    Ok(())
}

pub fn discard(store: &Store, id: &str) -> Result<()> {
    let _lock = store.lock(id)?;
    let mut transaction = store.load(id)?;
    match transaction.status {
        TransactionStatus::Discarded => {
            println!("DISCARDED {}", transaction.id);
            return Ok(());
        }
        TransactionStatus::Prepared => {}
        TransactionStatus::Committed | TransactionStatus::Pushed => {
            return Err(AppError::usage(format!("transaction {} already created a commit", transaction.id)));
        }
    }
    let repository = Repository::from_root(&transaction.repository_root)?;
    if transaction.pending_commit.is_some() {
        return Err(AppError::retry(format!(
            "transaction {} has a pending commit; run commit to recover it before discarding",
            transaction.id
        )));
    }
    remove_verified_stale_index_lock(&repository, &transaction)?;
    repository.delete_refs(&transaction.references())?;
    transaction.status = TransactionStatus::Discarded;
    transaction.terminal_at = Some(now_seconds());
    transaction.pending_commit = None;
    transaction.index_lock_token = None;
    store.save(&transaction)?;
    println!("DISCARDED {}", transaction.id);
    Ok(())
}

fn remove_verified_stale_index_lock(repository: &Repository, transaction: &crate::state::Transaction) -> Result<()> {
    let Some(token) = &transaction.index_lock_token else {
        return Ok(());
    };
    let index = repository.git_path("index")?;
    let mut lock_name = OsString::from(index.as_os_str());
    lock_name.push(".lock");
    let lock = PathBuf::from(lock_name);
    let marker = format!("ai-commit-index-lock {} {token}\n", transaction.id);
    if fs::read(&lock).ok().as_deref() == Some(marker.as_bytes()) {
        fs::remove_file(&lock)?;
    }
    Ok(())
}

use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use fs4::FileExt;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::{
    error::{AppError, ErrorKind, Result},
    git::Repository,
};

pub const RECEIPT_RETENTION_SECONDS: u64 = 7 * 24 * 60 * 60;
const RECEIPT_CLEANUP_BATCH_SIZE: usize = 64;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageFormat {
    Conventional,
    Natural,
}

impl MessageFormat {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Conventional => "conventional",
            Self::Natural => "natural",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Prepared,
    Committed,
    Pushed,
    Discarded,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PendingCommit {
    pub commit_oid: String,
    pub commit_tree: String,
    pub hook_added: Vec<String>,
    #[serde(default)]
    pub parent: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Transaction {
    pub id: String,
    pub repository_root: PathBuf,
    pub branch: String,
    pub base_head: String,
    #[serde(default)]
    pub unborn: bool,
    pub prepared_tree: String,
    pub shared_index_tree: String,
    pub message_format: MessageFormat,
    pub trailer: Option<String>,
    pub paths: Vec<String>,
    pub name_status: String,
    pub shortstat: String,
    pub created_at: u64,
    pub status: TransactionStatus,
    pub pending_commit: Option<PendingCommit>,
    pub commit_oid: Option<String>,
    #[serde(default)]
    pub hook_added: Vec<String>,
    #[serde(default)]
    pub index_lock_token: Option<String>,
    pub reconciled: bool,
    pub push_requested: bool,
    pub terminal_at: Option<u64>,
}

impl Transaction {
    pub fn reference(&self) -> String {
        format!("refs/ai-commit/transactions/{}", self.id)
    }

    pub fn base_reference(&self) -> String {
        format!("refs/ai-commit/bases/{}", self.id)
    }

    pub fn index_reference(&self) -> String {
        format!("refs/ai-commit/indexes/{}", self.id)
    }

    pub fn references(&self) -> [String; 3] {
        [self.reference(), self.base_reference(), self.index_reference()]
    }
}

#[derive(Clone, Debug)]
pub struct Store {
    transactions: PathBuf,
    temporary: PathBuf,
}

impl Store {
    pub fn discover() -> Result<Self> {
        let root = state_root()?;
        let transactions = root.join("transactions");
        let temporary = root.join("tmp");
        create_private_dir(&root)?;
        create_private_dir(&transactions)?;
        create_private_dir(&temporary)?;
        Ok(Self { transactions, temporary })
    }

    pub fn temporary(&self) -> &Path {
        &self.temporary
    }

    pub fn transaction_path(&self, id: &str) -> Result<PathBuf> {
        validate_id(id)?;
        Ok(self.transactions.join(format!("{id}.json")))
    }

    pub fn load(&self, id: &str) -> Result<Transaction> {
        let path = self.transaction_path(id)?;
        let mut source = String::new();
        File::open(&path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    AppError::usage(format!("unknown transaction: {id}"))
                } else {
                    AppError::operational(format!("cannot read transaction {id}: {error}"))
                }
            })?
            .read_to_string(&mut source)?;
        serde_json::from_str(&source)
            .map_err(|error| AppError::operational(format!("invalid transaction journal {}: {error}", path.display())))
    }

    pub fn save(&self, transaction: &Transaction) -> Result<()> {
        let path = self.transaction_path(&transaction.id)?;
        let bytes = serde_json::to_vec_pretty(transaction)
            .map_err(|error| AppError::operational(format!("cannot serialize transaction: {error}")))?;
        let mut temporary = NamedTempFile::new_in(&self.transactions)?;
        temporary.as_file_mut().write_all(&bytes)?;
        temporary.as_file_mut().write_all(b"\n")?;
        temporary.as_file_mut().sync_all()?;
        temporary.persist(&path).map_err(|error| {
            AppError::operational(format!("cannot persist transaction {}: {}", transaction.id, error.error))
        })?;
        File::open(&self.transactions)?.sync_all()?;
        Ok(())
    }

    pub fn lock(&self, id: &str) -> Result<TransactionLock> {
        validate_id(id)?;
        let path = self.transactions.join(format!("{id}.lock"));
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|error| AppError::operational(format!("cannot open transaction lock {id}: {error}")))?;
        match FileExt::try_lock(&file) {
            Ok(()) => {
                file.set_len(0)?;
                writeln!(file, "{}", std::process::id())?;
                file.sync_all()?;
                Ok(TransactionLock { file })
            }
            Err(fs4::TryLockError::WouldBlock) => Err(AppError::retry(format!("transaction is already in use: {id}"))),
            Err(fs4::TryLockError::Error(error)) => {
                Err(AppError::operational(format!("cannot lock transaction {id}: {error}")))
            }
        }
    }

    pub fn cleanup_receipts(&self) -> Result<()> {
        let now = now_seconds();
        let entries = match fs::read_dir(&self.transactions) {
            Ok(entries) => entries,
            Err(error) => return Err(AppError::operational(format!("cannot scan transaction receipts: {error}"))),
        };
        let mut cleaned = 0;
        for entry in entries.take(10_000) {
            if cleaned >= RECEIPT_CLEANUP_BATCH_SIZE {
                break;
            }
            let entry = entry?;
            if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Ok(source) = fs::read_to_string(entry.path()) else {
                continue;
            };
            let Ok(transaction) = serde_json::from_str::<Transaction>(&source) else {
                continue;
            };
            let Some(terminal_at) = transaction.terminal_at else {
                continue;
            };
            if now.saturating_sub(terminal_at) < RECEIPT_RETENTION_SECONDS {
                continue;
            }
            let _lock = match self.lock(&transaction.id) {
                Ok(lock) => lock,
                Err(error) if error.kind == ErrorKind::Retry => continue,
                Err(error) => return Err(error),
            };
            let refs_deleted = match Repository::from_root(&transaction.repository_root) {
                Ok(repository) => repository.delete_refs(&transaction.references()).is_ok(),
                Err(_) => true,
            };
            if !refs_deleted {
                continue;
            }
            let _ = fs::remove_file(entry.path());
            cleaned += 1;
        }
        Ok(())
    }
}

pub struct TransactionLock {
    file: File,
}

impl Drop for TransactionLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

pub fn validate_id(id: &str) -> Result<()> {
    if id.len() != 16 || !id.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()) {
        return Err(AppError::usage("transaction IDs must be 16 lowercase hexadecimal characters"));
    }
    Ok(())
}

pub fn now_seconds() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn state_root() -> Result<PathBuf> {
    if let Some(value) = env::var_os("AI_COMMIT_STATE_DIR") {
        if value.is_empty() {
            return Err(AppError::usage("AI_COMMIT_STATE_DIR may not be empty"));
        }
        return Ok(PathBuf::from(value));
    }
    if let Some(value) = env::var_os("XDG_STATE_HOME") {
        if value.is_empty() {
            return Err(AppError::usage("XDG_STATE_HOME may not be empty"));
        }
        return Ok(PathBuf::from(value).join("ai-commit"));
    }
    let home = env::var_os("HOME").ok_or_else(|| AppError::usage("HOME is required when no state override is set"))?;
    Ok(PathBuf::from(home).join(".local/state/ai-commit"))
}

fn create_private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .map_err(|error| AppError::operational(format!("cannot create state directory {}: {error}", path.display())))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

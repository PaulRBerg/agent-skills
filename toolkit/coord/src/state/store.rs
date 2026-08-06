use std::{
    env, fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::{
    domain::{Client, SessionState, WorkState},
    error::{AppError, Result},
};

use super::schema;

pub(crate) const MESSAGE_TTL: f64 = 48.0 * 60.0 * 60.0;
pub(crate) const NOTE_TTL: f64 = 7.0 * 24.0 * 60.0 * 60.0;
pub(crate) const MAX_INBOX_MESSAGES: usize = 50;
pub(super) const MAX_ERROR_CODE_CHARS: usize = 80;

pub(crate) struct Store {
    pub(super) connection: Connection,
    path: PathBuf,
}

impl Store {
    pub(crate) fn open_default() -> Result<Self> {
        Self::open(private_state_dir()?.join("state.db"))
    }

    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let parent = path.parent().ok_or_else(|| AppError::operational("state database path has no parent"))?;
        create_private_dir(parent)?;

        let mut connection = Connection::open(&path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.busy_timeout(Duration::from_secs(5))?;
        schema::initialize(&mut connection, &path)?;

        connection.busy_timeout(Duration::from_millis(250))?;
        enable_wal(&connection)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        set_private_file_permissions(&path)?;

        Ok(Self { connection, path })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn generation(&self) -> Result<i64> {
        Ok(self.connection.query_row("SELECT value FROM metadata WHERE key = 'generation'", [], |row| row.get(0))?)
    }

    pub(crate) fn prune(&mut self, current: f64) -> Result<()> {
        self.immediate(|transaction| {
            transaction.execute("DELETE FROM messages WHERE created_at < ?1", [current - MESSAGE_TTL])?;
            transaction.execute("DELETE FROM notes WHERE created_at < ?1", [current - NOTE_TTL])?;
            Ok(())
        })
    }

    pub(super) fn immediate<T>(&mut self, operation: impl FnOnce(&Transaction<'_>) -> Result<T>) -> Result<T> {
        let transaction = self.connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let result = operation(&transaction)?;
        transaction.commit()?;
        Ok(result)
    }
}

pub(crate) fn private_state_dir() -> Result<PathBuf> {
    let directory = if let Some(value) = env::var_os("AI_COORD_STATE_DIR").filter(|v| !v.is_empty()) {
        PathBuf::from(value)
    } else {
        let base = if let Some(value) = env::var_os("XDG_STATE_HOME").filter(|v| !v.is_empty()) {
            PathBuf::from(value)
        } else {
            let home = env::var_os("HOME")
                .filter(|v| !v.is_empty())
                .ok_or_else(|| AppError::operational("HOME is not set"))?;
            PathBuf::from(home).join(".local/state")
        };
        base.join("ai-coord")
    };
    create_private_dir(&directory)?;
    Ok(directory)
}

fn create_private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn set_private_file_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn enable_wal(connection: &Connection) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match connection.query_row("PRAGMA journal_mode = WAL", [], |row| row.get::<_, String>(0)) {
            Ok(mode) if mode.eq_ignore_ascii_case("wal") => return Ok(()),
            Ok(mode) => {
                return Err(AppError::operational(format!("could not enable SQLite WAL mode: {mode}")));
            }
            Err(error) if is_locked(&error) && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error.into()),
        }
    }
}

fn is_locked(error: &rusqlite::Error) -> bool {
    error.to_string().to_ascii_lowercase().contains("locked")
}

pub(super) fn bump_generation(transaction: &Transaction<'_>) -> Result<()> {
    transaction.execute("UPDATE metadata SET value = value + 1 WHERE key = 'generation'", [])?;
    Ok(())
}

pub(super) const fn client_name(client: Client) -> &'static str {
    match client {
        Client::Codex => "codex",
        Client::Claude => "claude",
    }
}

pub(super) fn parse_client(value: String) -> rusqlite::Result<Client> {
    match value.as_str() {
        "codex" => Ok(Client::Codex),
        "claude" => Ok(Client::Claude),
        _ => Err(invalid_value(format!("invalid client {value:?}"))),
    }
}

pub(super) const fn session_state_name(state: SessionState) -> &'static str {
    match state {
        SessionState::Idle => "idle",
        SessionState::InFlight => "in_flight",
        SessionState::Waiting => "waiting",
        SessionState::Working => "working",
        SessionState::Unknown => "unknown",
    }
}

pub(super) fn parse_session_state(value: String) -> rusqlite::Result<SessionState> {
    match value.as_str() {
        "idle" => Ok(SessionState::Idle),
        "in_flight" => Ok(SessionState::InFlight),
        "waiting" => Ok(SessionState::Waiting),
        "working" => Ok(SessionState::Working),
        "unknown" => Ok(SessionState::Unknown),
        _ => Err(invalid_value(format!("invalid session state {value:?}"))),
    }
}

pub(super) const fn work_state_name(state: WorkState) -> &'static str {
    match state {
        WorkState::Active => "active",
        WorkState::Draft => "draft",
        WorkState::Queued => "queued",
    }
}

pub(super) fn parse_work_state(value: String) -> rusqlite::Result<WorkState> {
    match value.as_str() {
        "active" => Ok(WorkState::Active),
        "draft" => Ok(WorkState::Draft),
        "queued" => Ok(WorkState::Queued),
        _ => Err(invalid_value(format!("invalid work state {value:?}"))),
    }
}

fn invalid_value(message: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, message)),
    )
}

pub(super) fn new_id() -> String {
    let bytes = rand::random::<[u8; 4]>();
    let mut result = String::with_capacity(8);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(result, "{byte:02x}");
    }
    result
}

pub(super) fn sanitize(text: &str, limit: usize) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut characters = collapsed.chars();
    let prefix = characters.by_ref().take(limit).collect::<String>();
    if characters.next().is_none() {
        return prefix;
    }
    let mut shortened = prefix.chars().take(limit.saturating_sub(1)).collect::<String>();
    while shortened.chars().last().is_some_and(char::is_whitespace) {
        shortened.pop();
    }
    shortened.push('…');
    shortened
}

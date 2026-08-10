//! SQLite-backed transient session state.
//!
//! State failures must not prevent a hook from notifying the user, so the public operational
//! methods log SQLite and filesystem errors and return an empty result instead of propagating
//! them. Configuration loading remains outside this module.

use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::Local;
use rusqlite::{Connection, params};
use serde::Serialize;

use crate::{
    config::{AppConfig, cleanup_marker_path, export_dir, runtime_config},
    error::Result,
};

const SCHEMA_VERSION: i32 = 1;
const AUTO_CLEANUP_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    prompt TEXT,
    cwd TEXT,
    job_number INTEGER,
    stopped_at DATETIME,
    last_wait_at DATETIME,
    duration_seconds INTEGER
);

CREATE INDEX IF NOT EXISTS idx_session_id ON sessions(session_id);
CREATE INDEX IF NOT EXISTS idx_created_at ON sessions(created_at);

CREATE TRIGGER IF NOT EXISTS auto_job_number
AFTER INSERT ON sessions
FOR EACH ROW
WHEN NEW.job_number IS NULL
BEGIN
    UPDATE sessions
    SET job_number = (
        SELECT COALESCE(MAX(job_number), 0) + 1
        FROM sessions
        WHERE session_id = NEW.session_id
    )
    WHERE id = NEW.id;
END;
"#;

/// A single persisted session turn, using the schema-v1 column names in JSON exports.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SessionRecord {
    pub id: i64,
    pub session_id: String,
    pub created_at: Option<String>,
    pub prompt: Option<String>,
    pub cwd: Option<String>,
    pub job_number: Option<i64>,
    pub stopped_at: Option<String>,
    pub last_wait_at: Option<String>,
    pub duration_seconds: Option<i64>,
}

/// Details from the newest completed turn in a Claude session.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JobInfo {
    pub job_number: Option<i64>,
    pub duration_seconds: Option<i64>,
    pub prompt: Option<String>,
}

/// Results from exporting and removing expired session rows.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CleanupStats {
    pub rows_deleted: usize,
    pub space_freed_kb: u64,
    pub rows_exported: usize,
}

/// Synchronous access to the schema-v1 SQLite session database.
#[derive(Clone, Debug)]
pub struct SessionStore {
    database_path: PathBuf,
    auto_cleanup_enabled: bool,
    export_before_cleanup: bool,
    retention_days: u32,
}

/// Compatibility name matching the former Python implementation.
pub type SessionTracker = SessionStore;

impl Default for SessionStore {
    fn default() -> Self {
        Self::new(runtime_config().as_ref())
    }
}

impl SessionStore {
    /// Creates a store for the YAML-configured database and ensures schema-v1 exists.
    pub fn new(config: &AppConfig) -> Self {
        let store = Self {
            database_path: config.database.path.clone(),
            auto_cleanup_enabled: config.cleanup.auto_cleanup_enabled,
            export_before_cleanup: config.cleanup.export_before_cleanup,
            retention_days: config.cleanup.retention_days,
        };
        store.initialize();
        store
    }

    /// Creates a store for an explicit database path, primarily for isolated callers and tests.
    pub fn from_database_path(path: impl Into<PathBuf>) -> Self {
        let store = Self {
            database_path: path.into(),
            auto_cleanup_enabled: true,
            export_before_cleanup: true,
            retention_days: 30,
        };
        store.initialize();
        store
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    /// Records a submitted prompt as a new active turn.
    pub fn track_prompt(&self, session_id: &str, prompt: &str, cwd: &str) {
        let Some(connection) = self.connection("track prompt") else {
            return;
        };
        if let Err(error) = connection.execute(
            "INSERT INTO sessions (session_id, prompt, cwd) VALUES (?1, ?2, ?3)",
            params![session_id, prompt, cwd],
        ) {
            tracing::error!(%error, "failed to track prompt");
        }
    }

    /// Marks the newest active turn for a session as complete.
    pub fn mark_stopped(&self, session_id: &str) {
        let Some(connection) = self.connection("mark stopped") else {
            return;
        };
        match connection.execute(
            "UPDATE sessions
             SET stopped_at = CURRENT_TIMESTAMP,
                 duration_seconds = CAST((julianday(CURRENT_TIMESTAMP) - julianday(created_at)) * 86400 AS INTEGER)
             WHERE id = (
                 SELECT id FROM sessions
                 WHERE session_id = ?1 AND stopped_at IS NULL
                 ORDER BY id DESC
                 LIMIT 1
             )",
            [session_id],
        ) {
            Ok(0) => tracing::warn!(session_id, "no active session found to mark stopped"),
            Ok(_) => tracing::info!(session_id, "marked session stopped"),
            Err(error) => tracing::error!(%error, "failed to mark session stopped"),
        }
    }

    /// Records that the newest active turn is waiting for user input.
    pub fn mark_waiting(&self, session_id: &str) {
        let Some(connection) = self.connection("mark waiting") else {
            return;
        };
        if let Err(error) = connection.execute(
            "UPDATE sessions
             SET last_wait_at = CURRENT_TIMESTAMP
             WHERE id = (
                 SELECT id FROM sessions
                 WHERE session_id = ?1 AND stopped_at IS NULL
                 ORDER BY id DESC
                 LIMIT 1
             )",
            [session_id],
        ) {
            tracing::error!(%error, "failed to mark session waiting");
        }
    }

    /// Returns the newest completed turn's job number, duration, and prompt.
    pub fn get_job_info(&self, session_id: &str) -> JobInfo {
        let Some(connection) = self.connection("get job info") else {
            return JobInfo::default();
        };
        match connection.query_row(
            "SELECT job_number, duration_seconds, prompt
             FROM sessions
             WHERE session_id = ?1 AND stopped_at IS NOT NULL
             ORDER BY id DESC
             LIMIT 1",
            [session_id],
            |row| Ok(JobInfo { job_number: row.get(0)?, duration_seconds: row.get(1)?, prompt: row.get(2)? }),
        ) {
            Ok(info) => info,
            Err(rusqlite::Error::QueryReturnedNoRows) => JobInfo::default(),
            Err(error) => {
                tracing::error!(%error, "failed to get job info");
                JobInfo::default()
            }
        }
    }

    /// Returns the job number of the newest active turn, if one exists.
    pub fn get_active_job_number(&self, session_id: &str) -> Option<i64> {
        self.query_optional("get active job number", session_id, "job_number")
    }

    /// Returns the prompt of the newest active turn, if one exists.
    pub fn get_active_prompt(&self, session_id: &str) -> Option<String> {
        let connection = self.connection("get active prompt")?;
        match connection.query_row(
            "SELECT prompt FROM sessions
             WHERE session_id = ?1 AND stopped_at IS NULL
             ORDER BY id DESC
             LIMIT 1",
            [session_id],
            |row| row.get(0),
        ) {
            Ok(prompt) => prompt,
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(error) => {
                tracing::error!(%error, "failed to get active prompt");
                None
            }
        }
    }

    /// Concise aliases for event adapters.
    pub fn job_info(&self, session_id: &str) -> JobInfo {
        self.get_job_info(session_id)
    }

    pub fn active_job_number(&self, session_id: &str) -> Option<i64> {
        self.get_active_job_number(session_id)
    }

    pub fn active_prompt(&self, session_id: &str) -> Option<String> {
        self.get_active_prompt(session_id)
    }

    /// Exports every schema column in newest-first order. `days` limits rows to the recent window.
    pub fn export_to_json(&self, output_path: &Path, days: Option<u32>) -> usize {
        let Some(connection) = self.connection("export sessions") else {
            return 0;
        };
        let records = match self.read_records(&connection, days) {
            Ok(records) => records,
            Err(error) => {
                tracing::error!(%error, "failed to read sessions for export");
                return 0;
            }
        };

        let result: Result<()> = (|| {
            let parent = output_path.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."));
            fs::create_dir_all(parent)?;
            let file = fs::File::create(output_path)?;
            serde_json::to_writer_pretty(file, &records)?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                tracing::info!(path = %output_path.display(), rows = records.len(), "exported sessions");
                records.len()
            }
            Err(error) => {
                tracing::error!(%error, path = %output_path.display(), "failed to export sessions");
                0
            }
        }
    }

    /// Deletes rows older than the retention period, optionally writing a full JSON backup first.
    pub fn cleanup_old_data(&self, retention_days: u32, export_before: bool) -> CleanupStats {
        let mut stats = CleanupStats::default();
        if export_before {
            let timestamp = Local::now().format("%Y%m%d_%H%M%S");
            let path = export_dir().join(format!("sessions_before_cleanup_{timestamp}.json"));
            stats.rows_exported = self.export_to_json(&path, None);
        }

        let size_before = file_size(&self.database_path);
        let Some(connection) = self.connection("clean up sessions") else {
            return stats;
        };
        // `created_at` is schema-v1 SQLite text (CURRENT_TIMESTAMP), so use SQLite's matching
        // datetime representation instead of binding a Unix integer.
        let modifier = format!("-{retention_days} days");
        match connection.execute("DELETE FROM sessions WHERE created_at < datetime('now', ?1)", [&modifier]) {
            Ok(rows_deleted) => stats.rows_deleted = rows_deleted,
            Err(error) => {
                tracing::error!(%error, "failed to delete expired sessions");
                return stats;
            }
        }

        if stats.rows_deleted > 0 {
            if let Err(error) = connection.execute_batch("VACUUM") {
                tracing::error!(%error, "failed to vacuum session database");
            }
            stats.space_freed_kb = size_before.saturating_sub(file_size(&self.database_path)) / 1024;
        }
        tracing::info!(?stats, "session cleanup complete");
        stats
    }

    /// Runs configured retention cleanup no more than once every 24 hours.
    pub fn cleanup_if_due(&self) -> Option<CleanupStats> {
        if !self.auto_cleanup_enabled || !should_run_auto_cleanup() {
            return None;
        }
        let stats = self.cleanup_old_data(self.retention_days, self.export_before_cleanup);
        mark_cleanup_done();
        Some(stats)
    }

    fn initialize(&self) {
        let Some(connection) = self.connection("initialize session database") else {
            return;
        };
        let version = match connection.pragma_query_value(None, "user_version", |row| row.get::<_, i32>(0)) {
            Ok(version) => version,
            Err(error) => {
                tracing::error!(%error, "failed to read session database schema version");
                return;
            }
        };
        if version >= SCHEMA_VERSION {
            return;
        }
        if let Err(error) = connection.execute_batch(SCHEMA) {
            tracing::error!(%error, "failed to initialize session database schema");
            return;
        }
        if let Err(error) = connection.pragma_update(None, "user_version", SCHEMA_VERSION) {
            tracing::error!(%error, "failed to record session database schema version");
        }
    }

    fn connection(&self, operation: &str) -> Option<Connection> {
        let parent = self.database_path.parent().filter(|path| !path.as_os_str().is_empty());
        if let Some(parent) = parent &&
            let Err(error) = fs::create_dir_all(parent)
        {
            tracing::error!(%error, path = %parent.display(), operation, "failed to create session database directory");
            return None;
        }
        match Connection::open(&self.database_path).and_then(|connection| {
            connection.execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA synchronous=NORMAL;
                 PRAGMA temp_store=MEMORY;
                 PRAGMA busy_timeout=3000;",
            )?;
            Ok(connection)
        }) {
            Ok(connection) => Some(connection),
            Err(error) => {
                tracing::error!(%error, path = %self.database_path.display(), operation, "session database unavailable");
                None
            }
        }
    }

    fn query_optional(&self, operation: &str, session_id: &str, column: &str) -> Option<i64> {
        let connection = self.connection(operation)?;
        let query = format!(
            "SELECT {column} FROM sessions
             WHERE session_id = ?1 AND stopped_at IS NULL
             ORDER BY id DESC
             LIMIT 1"
        );
        match connection.query_row(&query, [session_id], |row| row.get(0)) {
            Ok(value) => value,
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(error) => {
                tracing::error!(%error, "failed to query active session");
                None
            }
        }
    }

    fn read_records(&self, connection: &Connection, days: Option<u32>) -> rusqlite::Result<Vec<SessionRecord>> {
        const SELECT: &str =
            "SELECT id, session_id, created_at, prompt, cwd, job_number, stopped_at, last_wait_at, duration_seconds
                              FROM sessions";
        let mut statement = match days {
            Some(_) => connection.prepare(&format!(
                "{SELECT} WHERE created_at >= datetime('now', ?1) ORDER BY created_at DESC, id DESC"
            ))?,
            None => connection.prepare(&format!("{SELECT} ORDER BY created_at DESC, id DESC"))?,
        };
        let modifier = days.map(|days| format!("-{days} days"));
        let rows = match modifier.as_deref() {
            Some(modifier) => statement.query_map([modifier], record_from_row)?,
            None => statement.query_map([], record_from_row)?,
        };
        rows.collect()
    }
}

impl crate::events::SessionState for SessionStore {
    fn track_prompt(&self, session_id: &str, prompt: &str, cwd: &str) {
        Self::track_prompt(self, session_id, prompt, cwd);
    }

    fn mark_stopped(&self, session_id: &str) {
        Self::mark_stopped(self, session_id);
    }

    fn mark_waiting(&self, session_id: &str) {
        Self::mark_waiting(self, session_id);
    }

    fn active_prompt(&self, session_id: &str) -> Option<String> {
        Self::active_prompt(self, session_id)
    }

    fn job_info(&self, session_id: &str) -> crate::events::JobInfo {
        let info = Self::job_info(self, session_id);
        crate::events::JobInfo {
            job_number: info.job_number,
            duration_seconds: info.duration_seconds,
            prompt: info.prompt,
        }
    }

    fn cleanup_if_due(&self) {
        let _ = Self::cleanup_if_due(self);
    }
}

/// Returns whether the XDG cleanup marker is absent or at least 24 hours old.
pub fn should_run_auto_cleanup() -> bool {
    should_run_auto_cleanup_at(&cleanup_marker_path())
}

/// Path-injectable form of [`should_run_auto_cleanup`] for tests and embedders.
pub fn should_run_auto_cleanup_at(marker_path: &Path) -> bool {
    fs::metadata(marker_path)
        .and_then(|metadata| metadata.modified())
        .map_or(true, |modified| modified.elapsed().is_ok_and(|elapsed| elapsed >= AUTO_CLEANUP_INTERVAL))
}

/// Updates the XDG cleanup marker after an automatic cleanup attempt.
pub fn mark_cleanup_done() {
    mark_cleanup_done_at(&cleanup_marker_path());
}

/// Path-injectable form of [`mark_cleanup_done`] for tests and embedders.
pub fn mark_cleanup_done_at(marker_path: &Path) {
    let parent = marker_path.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let result: std::io::Result<()> = (|| {
        fs::create_dir_all(parent)?;
        fs::File::create(marker_path)?;
        Ok(())
    })();
    if let Err(error) = result {
        tracing::warn!(%error, path = %marker_path.display(), "failed to update cleanup marker");
    }
}

fn record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionRecord> {
    Ok(SessionRecord {
        id: row.get(0)?,
        session_id: row.get(1)?,
        created_at: row.get(2)?,
        prompt: row.get(3)?,
        cwd: row.get(4)?,
        job_number: row.get(5)?,
        stopped_at: row.get(6)?,
        last_wait_at: row.get(7)?,
        duration_seconds: row.get(8)?,
    })
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|metadata| metadata.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn tracks_active_turns_and_latest_completion() {
        let directory = tempdir().unwrap();
        let store = SessionStore::from_database_path(directory.path().join("sessions.db"));

        store.track_prompt("session-1", "first", "/tmp");
        assert_eq!(store.active_job_number("session-1"), Some(1));
        assert_eq!(store.active_prompt("session-1").as_deref(), Some("first"));
        store.mark_waiting("session-1");
        store.mark_stopped("session-1");

        assert_eq!(
            store.job_info("session-1"),
            JobInfo { job_number: Some(1), duration_seconds: Some(0), prompt: Some("first".to_owned()) }
        );
        assert_eq!(store.active_prompt("session-1"), None);
    }

    #[test]
    fn cleanup_uses_sqlite_text_timestamps() {
        let directory = tempdir().unwrap();
        let store = SessionStore::from_database_path(directory.path().join("sessions.db"));
        let connection = Connection::open(store.database_path()).unwrap();
        connection
            .execute("INSERT INTO sessions (session_id, prompt, cwd, created_at) VALUES ('old', 'old', '/', datetime('now', '-31 days'))", [])
            .unwrap();
        connection
            .execute("INSERT INTO sessions (session_id, prompt, cwd, created_at) VALUES ('new', 'new', '/', datetime('now', '-5 days'))", [])
            .unwrap();

        let stats = store.cleanup_old_data(30, false);
        assert_eq!(stats.rows_deleted, 1);
        assert_eq!(store.active_prompt("old"), None);
        assert_eq!(store.active_prompt("new").as_deref(), Some("new"));
    }

    #[test]
    fn export_contains_all_schema_columns_newest_first() {
        let directory = tempdir().unwrap();
        let store = SessionStore::from_database_path(directory.path().join("sessions.db"));
        store.track_prompt("session-1", "first", "/one");
        store.track_prompt("session-2", "second", "/two");
        let output = directory.path().join("exports/sessions.json");

        assert_eq!(store.export_to_json(&output, None), 2);
        let value: serde_json::Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
        let sessions = value.as_array().unwrap();
        assert_eq!(sessions[0]["prompt"], "second");
        assert_eq!(sessions[0].as_object().unwrap().len(), 9);
    }

    #[test]
    fn cleanup_marker_runs_initially_and_skips_recent_cleanup() {
        let directory = tempdir().unwrap();
        let marker = directory.path().join(".last_cleanup");
        assert!(should_run_auto_cleanup_at(&marker));
        mark_cleanup_done_at(&marker);
        assert!(!should_run_auto_cleanup_at(&marker));
        assert!(marker.exists());
    }
}

use rusqlite::{OptionalExtension, Row, params};
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;

use crate::{
    domain::{Identity, ProcessFingerprint},
    error::{AppError, Result},
};

use super::{
    EndedObservation, SessionRow, SessionUpdate, Store,
    store::{bump_generation, client_name, parse_client, parse_session_state, session_state_name},
};

impl Store {
    pub(crate) fn upsert_session(&mut self, update: &SessionUpdate) -> Result<SessionRow> {
        self.immediate(|transaction| {
            let old_permission_mode = if update.update_permission_mode {
                transaction
                    .query_row(
                        "SELECT permission_mode FROM sessions
                         WHERE client = ?1 AND session_id = ?2",
                        params![client_name(update.identity.client), update.identity.session_id],
                        |row| row.get::<_, Option<String>>(0),
                    )
                    .optional()?
                    .flatten()
            } else {
                None
            };
            let (pid, start_token) = fingerprint_values(update.fingerprint.as_ref());
            transaction.execute(
                "INSERT INTO sessions(
                    client, session_id, cwd, repo_root, state, name, label, waiting_for,
                    permission_mode, pid, process_start_token, source, started_at, last_seen,
                    revision
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 1)
                 ON CONFLICT(client, session_id) DO UPDATE SET
                    cwd = excluded.cwd,
                    repo_root = excluded.repo_root,
                    state = excluded.state,
                    name = COALESCE(excluded.name, sessions.name),
                    label = COALESCE(excluded.label, sessions.label),
                    waiting_for = excluded.waiting_for,
                    permission_mode = CASE WHEN ?15 THEN excluded.permission_mode
                                           ELSE sessions.permission_mode END,
                    pid = CASE WHEN excluded.pid IS NULL THEN sessions.pid ELSE excluded.pid END,
                    process_start_token = CASE
                        WHEN excluded.pid IS NULL THEN sessions.process_start_token
                        ELSE excluded.process_start_token END,
                    source = excluded.source,
                    last_seen = excluded.last_seen,
                    revision = sessions.revision + 1",
                params![
                    client_name(update.identity.client),
                    update.identity.session_id,
                    update.cwd,
                    update.repo_root,
                    session_state_name(update.state),
                    update.name,
                    update.label,
                    update.waiting_for,
                    update.permission_mode,
                    pid,
                    start_token,
                    update.source,
                    update.started_at.unwrap_or(update.current),
                    update.current,
                    update.update_permission_mode,
                ],
            )?;
            if update.update_permission_mode && old_permission_mode.as_deref() != update.permission_mode.as_deref() {
                bump_generation(transaction)?;
            }
            Ok(transaction.query_row(
                &session_select("WHERE client = ?1 AND session_id = ?2"),
                params![client_name(update.identity.client), update.identity.session_id],
                session_from_row,
            )?)
        })
    }

    pub(crate) fn set_session_label(&mut self, identity: &Identity, label: Option<&str>) -> Result<()> {
        self.immediate(|transaction| {
            transaction.execute(
                "UPDATE sessions SET label = ?1 WHERE client = ?2 AND session_id = ?3",
                params![label, client_name(identity.client), identity.session_id],
            )?;
            Ok(())
        })
    }

    pub(crate) fn set_session_callsign(&mut self, identity: &Identity, callsign: &str) -> Result<()> {
        let key = callsign_key(callsign);
        self.immediate(|transaction| {
            let current = transaction
                .query_row(
                    "SELECT callsign FROM sessions WHERE client = ?1 AND session_id = ?2",
                    params![client_name(identity.client), identity.session_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .optional()?;
            let Some(current) = current else {
                return Err(AppError::usage("session is not registered"));
            };
            if current.as_deref() == Some(callsign) {
                return Ok(());
            }
            let occupied = transaction.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM sessions
                    WHERE callsign_key = ?1 AND NOT (client = ?2 AND session_id = ?3)
                 )",
                params![key, client_name(identity.client), identity.session_id],
                |row| row.get::<_, bool>(0),
            )?;
            if occupied {
                return Err(AppError::usage("callsign is already in use"));
            }
            transaction.execute(
                "UPDATE sessions SET callsign = ?1, callsign_key = ?2
                 WHERE client = ?3 AND session_id = ?4",
                params![callsign, key, client_name(identity.client), identity.session_id],
            )?;
            bump_generation(transaction)
        })
    }

    /// End a session from an authoritative SessionEnd event.
    pub(crate) fn end_session(&mut self, identity: &Identity) -> Result<()> {
        self.immediate(|transaction| {
            remove_session(transaction, identity)?;
            bump_generation(transaction)
        })
    }

    /// Remove sessions proven dead by observations of the same stored row revision.
    ///
    /// An upsert racing with a liveness probe increments `revision`; the stale probe then
    /// cannot remove the refreshed row. Unknown or merely old sessions are never removed.
    pub(crate) fn reconcile_ended(&mut self, observations: &[EndedObservation]) -> Result<usize> {
        self.immediate(|transaction| {
            let mut removed = 0;
            for observation in observations {
                let stored = transaction
                    .query_row(
                        "SELECT pid, process_start_token, revision FROM sessions
                         WHERE client = ?1 AND session_id = ?2",
                        params![client_name(observation.identity.client), observation.identity.session_id],
                        |row| {
                            let pid = row.get::<_, Option<u32>>(0)?;
                            let start_token = row.get::<_, Option<String>>(1)?;
                            let fingerprint = pid.map(|pid| ProcessFingerprint { pid, start_token });
                            Ok((fingerprint, row.get::<_, i64>(2)?))
                        },
                    )
                    .optional()?;
                if stored.as_ref() == Some(&(observation.expected_fingerprint.clone(), observation.expected_revision)) {
                    remove_session(transaction, &observation.identity)?;
                    removed += 1;
                }
            }
            if removed > 0 {
                bump_generation(transaction)?;
            }
            Ok(removed)
        })
    }

    pub(crate) fn session(&self, identity: &Identity) -> Result<Option<SessionRow>> {
        Ok(self
            .connection
            .query_row(
                &session_select("WHERE client = ?1 AND session_id = ?2"),
                params![client_name(identity.client), identity.session_id],
                session_from_row,
            )
            .optional()?)
    }

    pub(crate) fn sessions(&self) -> Result<Vec<SessionRow>> {
        let mut statement = self.connection.prepare(&session_select("ORDER BY client, started_at, session_id"))?;
        Ok(statement.query_map([], session_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn identities_for_processes(&self, references: &[ProcessFingerprint]) -> Result<Vec<Identity>> {
        if references.is_empty() {
            return Ok(Vec::new());
        }
        let sessions = self.sessions()?;
        let exact = sessions
            .iter()
            .filter_map(|session| {
                let fingerprint = session.fingerprint.as_ref()?;
                fingerprint.start_token.as_ref()?;
                references.iter().any(|reference| reference == fingerprint).then(|| session.identity.clone())
            })
            .collect::<Vec<_>>();
        if !exact.is_empty() {
            return Ok(exact);
        }
        Ok(sessions
            .into_iter()
            .filter(|session| {
                session.fingerprint.as_ref().is_some_and(|fingerprint| {
                    fingerprint.start_token.is_none() &&
                        references.iter().any(|reference| reference.pid == fingerprint.pid)
                })
            })
            .map(|session| session.identity)
            .collect())
    }
}

fn remove_session(transaction: &rusqlite::Transaction<'_>, identity: &Identity) -> Result<()> {
    let values = params![client_name(identity.client), identity.session_id];
    transaction.execute("DELETE FROM claims WHERE client = ?1 AND session_id = ?2", values)?;
    transaction.execute(
        "DELETE FROM delegates WHERE parent_client = ?1 AND parent_session_id = ?2",
        params![client_name(identity.client), identity.session_id],
    )?;
    transaction.execute(
        "DELETE FROM sessions WHERE client = ?1 AND session_id = ?2",
        params![client_name(identity.client), identity.session_id],
    )?;
    Ok(())
}

fn fingerprint_values(fingerprint: Option<&ProcessFingerprint>) -> (Option<u32>, Option<&str>) {
    fingerprint.map_or((None, None), |value| (Some(value.pid), value.start_token.as_deref()))
}

fn session_select(suffix: &str) -> String {
    format!(
        "SELECT client, session_id, cwd, repo_root, state, callsign, name, label,
                waiting_for, permission_mode, pid, process_start_token, source, started_at,
                last_seen, revision
         FROM sessions {suffix}"
    )
}

fn session_from_row(row: &Row<'_>) -> rusqlite::Result<SessionRow> {
    let pid = row.get::<_, Option<u32>>(10)?;
    let start_token = row.get::<_, Option<String>>(11)?;
    Ok(SessionRow {
        identity: Identity { client: parse_client(row.get(0)?)?, session_id: row.get(1)? },
        cwd: row.get(2)?,
        repo_root: row.get(3)?,
        state: parse_session_state(row.get(4)?)?,
        callsign: row.get(5)?,
        name: row.get(6)?,
        label: row.get(7)?,
        waiting_for: row.get(8)?,
        permission_mode: row.get(9)?,
        fingerprint: pid.map(|pid| ProcessFingerprint { pid, start_token }),
        source: row.get(12)?,
        started_at: row.get(13)?,
        last_seen: row.get(14)?,
        revision: row.get(15)?,
    })
}

fn callsign_key(callsign: &str) -> String {
    let whitespace_collapsed = callsign.split_whitespace().collect::<Vec<_>>().join(" ");
    whitespace_collapsed
        .nfc()
        .case_fold()
        .nfc()
        .filter(|character| !matches!(character, '\u{fe0e}' | '\u{fe0f}'))
        .collect()
}

#[cfg(test)]
pub(super) fn codex_identity(session_id: &str) -> Identity {
    Identity { client: crate::domain::Client::Codex, session_id: session_id.to_owned() }
}

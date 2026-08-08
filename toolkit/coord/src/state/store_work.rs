use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use crate::{
    domain::{Identity, Scope, ScopeKind, WorkState},
    error::{AppError, Result},
};

use super::{
    BaselineRow, DirtObservationRow, ResidualOwnerRow, Store, WorkRow, WorkUpdate,
    store::{bump_generation, client_name, parse_client, parse_work_state, work_state_name},
    store_communications::add_message,
};

/// State-owned facade for one atomic work arbitration.
///
/// Callers collect slow provider and Git evidence before entering this facade,
/// then re-read every mutable work decision through it before writing.
pub(crate) struct WorkTransaction<'store> {
    transaction: Transaction<'store>,
}

impl Store {
    pub(crate) fn with_work_transaction<T>(
        &mut self,
        operation: impl FnOnce(&WorkTransaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let transaction = self.connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let work = WorkTransaction { transaction };
        let result = operation(&work)?;
        work.transaction.commit()?;
        Ok(result)
    }
}

impl WorkTransaction<'_> {
    pub(crate) fn callsign(&self, identity: &Identity) -> Result<Option<String>> {
        Ok(self
            .transaction
            .query_row(
                "SELECT callsign FROM sessions WHERE client = ?1 AND session_id = ?2",
                params![client_name(identity.client), identity.session_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten())
    }

    pub(crate) fn work(&self, identity: &Identity) -> Result<Option<WorkRow>> {
        work_from(&self.transaction, identity)
    }

    pub(crate) fn works(&self, repo_root: &str) -> Result<Vec<WorkRow>> {
        works_from(&self.transaction, Some(repo_root))
    }

    pub(crate) fn baselines(&self, identity: &Identity) -> Result<Vec<BaselineRow>> {
        baselines_from(&self.transaction, identity)
    }

    pub(crate) fn residual_owners(&self, repo_root: &str) -> Result<Vec<ResidualOwnerRow>> {
        residual_owners_from(&self.transaction, repo_root)
    }

    pub(crate) fn save_work(&self, update: &WorkUpdate) -> Result<i64> {
        save_work(&self.transaction, update)
    }

    /// Allocate a strictly increasing submission timestamp without giving
    /// drafts any queue age before promotion.
    pub(crate) fn next_submission_time(&self, current: f64) -> Result<f64> {
        let previous = self.transaction.query_row(
            "SELECT value FROM metadata WHERE key = 'submission_clock_micros'",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        let requested = (current * 1_000_000.0).floor().clamp(0.0, i64::MAX as f64) as i64;
        let allocated = requested.max(previous.saturating_add(1));
        self.transaction
            .execute("UPDATE metadata SET value = ?1 WHERE key = 'submission_clock_micros'", [allocated])?;
        Ok(allocated as f64 / 1_000_000.0)
    }

    pub(crate) fn send_message(
        &self,
        sender: &Identity,
        recipient: &Identity,
        text: &str,
        repo_root: Option<&str>,
        current: f64,
    ) -> Result<String> {
        add_message(&self.transaction, sender, recipient, text, repo_root, current)
    }
}

impl Store {
    /// Atomically replaces one work item, all scopes, optional baselines, and residual ownership.
    pub(crate) fn save_work(&mut self, update: &WorkUpdate) -> Result<i64> {
        self.immediate(|transaction| save_work(transaction, update))
    }

    /// Create or atomically replace this session's non-authoritative draft.
    pub(crate) fn save_draft(
        &mut self,
        identity: &Identity,
        repo_root: &str,
        label: &str,
        scopes: &[Scope],
        current: f64,
    ) -> Result<WorkRow> {
        self.immediate(|transaction| {
            let existing = work_from(transaction, identity)?;
            if existing.as_ref().is_some_and(|work| work.state != WorkState::Draft) {
                return Err(AppError::operational("queued or active work exists; run ai-coord done before drafting"));
            }
            save_work(
                transaction,
                &WorkUpdate {
                    identity: identity.clone(),
                    repo_root: repo_root.to_owned(),
                    label: label.to_owned(),
                    state: WorkState::Draft,
                    blocked_reason: None,
                    scopes: scopes.to_vec(),
                    baselines: Some(Vec::new()),
                    residual_paths: Vec::new(),
                    draft_created_at: Some(current),
                    submitted_at: None,
                    updated_at: current,
                    expected_revision: existing.map(|work| work.revision),
                },
            )?;
            work_from(transaction, identity)?.ok_or_else(|| AppError::retry("draft disappeared during replacement"))
        })
    }

    pub(crate) fn work(&self, identity: &Identity) -> Result<Option<WorkRow>> {
        work_from(&self.connection, identity)
    }

    pub(crate) fn works(&self, repo_root: Option<&str>) -> Result<Vec<WorkRow>> {
        works_from(&self.connection, repo_root)
    }

    pub(crate) fn residual_owners(&self, repo_root: &str) -> Result<Vec<ResidualOwnerRow>> {
        residual_owners_from(&self.connection, repo_root)
    }

    pub(crate) fn baselines(&self, identity: &Identity) -> Result<Vec<BaselineRow>> {
        baselines_from(&self.connection, identity)
    }

    pub(crate) fn delete_work(&mut self, identity: &Identity) -> Result<bool> {
        self.immediate(|transaction| {
            let removed = transaction.execute(
                "DELETE FROM work_items WHERE client = ?1 AND session_id = ?2",
                params![client_name(identity.client), identity.session_id],
            )? > 0;
            if removed {
                bump_generation(transaction)?;
            }
            Ok(removed)
        })
    }

    pub(crate) fn observe_dirt(
        &mut self,
        repo_root: &str,
        blob_hashes: &[(String, String)],
        current: f64,
    ) -> Result<Vec<DirtObservationRow>> {
        self.immediate(|transaction| {
            let desired = blob_hashes.iter().map(|(path, _)| path.as_str()).collect::<HashSet<_>>();
            let existing = {
                let mut statement = transaction.prepare("SELECT path FROM dirt_observations WHERE repo_root = ?1")?;
                statement
                    .query_map([repo_root], |row| row.get::<_, String>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            };
            for path in existing {
                if !desired.contains(path.as_str()) {
                    transaction.execute(
                        "DELETE FROM dirt_observations WHERE repo_root = ?1 AND path = ?2",
                        params![repo_root, path],
                    )?;
                }
            }
            for (path, blob_hash) in blob_hashes {
                transaction.execute(
                    "INSERT INTO dirt_observations(
                        repo_root, path, blob_hash, first_seen, last_seen
                     ) VALUES (?1, ?2, ?3, ?4, ?4)
                     ON CONFLICT(repo_root, path) DO UPDATE SET
                        blob_hash = excluded.blob_hash,
                        first_seen = CASE
                            WHEN dirt_observations.blob_hash = excluded.blob_hash
                            THEN dirt_observations.first_seen ELSE excluded.first_seen END,
                        last_seen = excluded.last_seen",
                    params![repo_root, path, blob_hash, current],
                )?;
            }
            dirt_observations_from(transaction, repo_root)
        })
    }

    pub(crate) fn record_residual_owners(
        &mut self,
        repo_root: &str,
        paths: &[String],
        identity: &Identity,
        current: f64,
    ) -> Result<()> {
        if paths.is_empty() {
            return Ok(());
        }
        self.immediate(|transaction| {
            for path in paths {
                transaction.execute(
                    "INSERT INTO residual_owners(
                        repo_root, path, client, session_id, released_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(repo_root, path) DO UPDATE SET
                        client = excluded.client,
                        session_id = excluded.session_id,
                        released_at = excluded.released_at",
                    params![repo_root, path, client_name(identity.client), identity.session_id, current],
                )?;
            }
            Ok(())
        })
    }

    pub(crate) fn replace_baselines(&mut self, identity: &Identity, baselines: &[BaselineRow]) -> Result<()> {
        self.immediate(|transaction| {
            let work_id = transaction
                .query_row(
                    "SELECT id FROM work_items
                     WHERE client = ?1 AND session_id = ?2 AND state = 'active'",
                    params![client_name(identity.client), identity.session_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;
            if let Some(work_id) = work_id {
                transaction.execute("DELETE FROM work_baselines WHERE work_id = ?1", [work_id])?;
                insert_baselines(transaction, work_id, baselines)?;
            }
            Ok(())
        })
    }
}

fn save_work(transaction: &Transaction<'_>, update: &WorkUpdate) -> Result<i64> {
    if update.scopes.is_empty() {
        return Err(AppError::usage("at least one scope is required"));
    }
    match update.expected_revision {
        Some(revision) => {
            let changed = transaction.execute(
                "UPDATE work_items SET
                    repo_root = ?1, label = ?2, state = ?3, blocked_reason = ?4,
                    draft_created_at = ?5, submitted_at = ?6, updated_at = ?7,
                    revision = revision + 1
                 WHERE client = ?8 AND session_id = ?9 AND revision = ?10",
                params![
                    update.repo_root,
                    update.label,
                    work_state_name(update.state),
                    update.blocked_reason,
                    update.draft_created_at,
                    update.submitted_at,
                    update.updated_at,
                    client_name(update.identity.client),
                    update.identity.session_id,
                    revision,
                ],
            )?;
            if changed != 1 {
                return Err(AppError::retry("work item changed during update"));
            }
        }
        None => {
            let exists = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM work_items WHERE client = ?1 AND session_id = ?2)",
                params![client_name(update.identity.client), update.identity.session_id],
                |row| row.get::<_, bool>(0),
            )?;
            if exists {
                return Err(AppError::retry("work item appeared during update"));
            }
            transaction.execute(
                "INSERT INTO work_items(
                    client, session_id, repo_root, label, state, blocked_reason,
                    draft_created_at, submitted_at, updated_at, revision
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1)",
                params![
                    client_name(update.identity.client),
                    update.identity.session_id,
                    update.repo_root,
                    update.label,
                    work_state_name(update.state),
                    update.blocked_reason,
                    update.draft_created_at,
                    update.submitted_at,
                    update.updated_at,
                ],
            )?;
        }
    }
    let work_id = transaction.query_row(
        "SELECT id FROM work_items WHERE client = ?1 AND session_id = ?2",
        params![client_name(update.identity.client), update.identity.session_id],
        |row| row.get::<_, i64>(0),
    )?;
    transaction.execute("DELETE FROM work_scopes WHERE work_id = ?1", [work_id])?;
    for scope in &update.scopes {
        transaction.execute(
            "INSERT INTO work_scopes(work_id, path, kind) VALUES (?1, ?2, ?3)",
            params![work_id, scope.path, scope_kind_name(scope.kind)],
        )?;
    }
    if let Some(baselines) = &update.baselines {
        transaction.execute("DELETE FROM work_baselines WHERE work_id = ?1", [work_id])?;
        insert_baselines(transaction, work_id, baselines)?;
    }
    for path in &update.residual_paths {
        transaction.execute(
            "INSERT INTO residual_owners(
                repo_root, path, client, session_id, released_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(repo_root, path) DO UPDATE SET
                client = excluded.client,
                session_id = excluded.session_id,
                released_at = excluded.released_at",
            params![
                update.repo_root,
                path,
                client_name(update.identity.client),
                update.identity.session_id,
                update.updated_at,
            ],
        )?;
    }
    bump_generation(transaction)?;
    Ok(work_id)
}

fn work_from(connection: &Connection, identity: &Identity) -> Result<Option<WorkRow>> {
    let base = connection
        .query_row(
            &work_select("WHERE client = ?1 AND session_id = ?2"),
            params![client_name(identity.client), identity.session_id],
            work_base_from_row,
        )
        .optional()?;
    base.map(|base| finish_work(connection, base)).transpose()
}

fn works_from(connection: &Connection, repo_root: Option<&str>) -> Result<Vec<WorkRow>> {
    let (query, arguments) = match repo_root {
        Some(repo_root) => {
            (work_select("WHERE repo_root = ?1 ORDER BY COALESCE(submitted_at, draft_created_at), id"), vec![repo_root])
        }
        None => (work_select("ORDER BY COALESCE(submitted_at, draft_created_at), id"), Vec::new()),
    };
    let mut statement = connection.prepare(&query)?;
    let bases = statement
        .query_map(rusqlite::params_from_iter(arguments), work_base_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    bases.into_iter().map(|base| finish_work(connection, base)).collect()
}

fn residual_owners_from(connection: &Connection, repo_root: &str) -> Result<Vec<ResidualOwnerRow>> {
    let mut statement = connection.prepare(
        "SELECT repo_root, path, client, session_id, released_at
         FROM residual_owners WHERE repo_root = ?1 ORDER BY path",
    )?;
    Ok(statement.query_map([repo_root], residual_owner_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn baselines_from(connection: &Connection, identity: &Identity) -> Result<Vec<BaselineRow>> {
    let mut statement = connection.prepare(
        "SELECT work_baselines.path, work_baselines.oid
         FROM work_baselines
         JOIN work_items ON work_items.id = work_baselines.work_id
         WHERE work_items.client = ?1 AND work_items.session_id = ?2 AND work_items.state = 'active'
         ORDER BY work_baselines.path",
    )?;
    Ok(statement
        .query_map(params![client_name(identity.client), identity.session_id], |row| {
            Ok(BaselineRow { path: row.get(0)?, oid: row.get(1)? })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

struct WorkBase {
    id: i64,
    identity: Identity,
    repo_root: String,
    label: String,
    state: WorkState,
    blocked_reason: Option<String>,
    draft_created_at: Option<f64>,
    submitted_at: Option<f64>,
    updated_at: f64,
    revision: i64,
}

fn work_select(suffix: &str) -> String {
    format!(
        "SELECT id, client, session_id, repo_root, label, state, blocked_reason,
                draft_created_at, submitted_at, updated_at, revision FROM work_items {suffix}"
    )
}

fn work_base_from_row(row: &Row<'_>) -> rusqlite::Result<WorkBase> {
    Ok(WorkBase {
        id: row.get(0)?,
        identity: Identity { client: parse_client(row.get(1)?)?, session_id: row.get(2)? },
        repo_root: row.get(3)?,
        label: row.get(4)?,
        state: parse_work_state(row.get(5)?)?,
        blocked_reason: row.get(6)?,
        draft_created_at: row.get(7)?,
        submitted_at: row.get(8)?,
        updated_at: row.get(9)?,
        revision: row.get(10)?,
    })
}

fn finish_work(connection: &Connection, base: WorkBase) -> Result<WorkRow> {
    let mut statement = connection.prepare("SELECT path, kind FROM work_scopes WHERE work_id = ?1 ORDER BY path")?;
    let scopes = statement
        .query_map([base.id], |row| Ok(Scope { path: row.get(0)?, kind: parse_scope_kind(row.get(1)?)? }))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(WorkRow {
        id: base.id,
        identity: base.identity,
        repo_root: base.repo_root,
        label: base.label,
        state: base.state,
        blocked_reason: base.blocked_reason,
        scopes,
        draft_created_at: base.draft_created_at,
        submitted_at: base.submitted_at,
        updated_at: base.updated_at,
        revision: base.revision,
    })
}

fn insert_baselines(transaction: &Transaction<'_>, work_id: i64, baselines: &[BaselineRow]) -> Result<()> {
    for baseline in baselines {
        transaction.execute(
            "INSERT INTO work_baselines(work_id, path, oid) VALUES (?1, ?2, ?3)",
            params![work_id, baseline.path, baseline.oid],
        )?;
    }
    Ok(())
}

fn dirt_observations_from(connection: &Connection, repo_root: &str) -> Result<Vec<DirtObservationRow>> {
    let mut statement = connection.prepare(
        "SELECT repo_root, path, blob_hash, first_seen, last_seen
         FROM dirt_observations WHERE repo_root = ?1 ORDER BY path",
    )?;
    Ok(statement
        .query_map([repo_root], |row| {
            Ok(DirtObservationRow {
                repo_root: row.get(0)?,
                path: row.get(1)?,
                blob_hash: row.get(2)?,
                first_seen: row.get(3)?,
                last_seen: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

fn residual_owner_from_row(row: &Row<'_>) -> rusqlite::Result<ResidualOwnerRow> {
    Ok(ResidualOwnerRow {
        repo_root: row.get(0)?,
        path: row.get(1)?,
        identity: Identity { client: parse_client(row.get(2)?)?, session_id: row.get(3)? },
        released_at: row.get(4)?,
    })
}

const fn scope_kind_name(kind: ScopeKind) -> &'static str {
    match kind {
        ScopeKind::Exact => "exact",
        ScopeKind::Recursive => "recursive",
    }
}

fn parse_scope_kind(value: String) -> rusqlite::Result<ScopeKind> {
    match value.as_str() {
        "exact" => Ok(ScopeKind::Exact),
        "recursive" => Ok(ScopeKind::Recursive),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("invalid scope kind {value:?}"))),
        )),
    }
}

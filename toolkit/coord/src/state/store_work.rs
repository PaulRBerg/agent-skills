use std::collections::{BTreeMap, HashSet};

use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use crate::{
    domain::{Identity, Scope, ScopeKind, WorkState},
    error::{AppError, Result},
};

use super::{
    BaselineRow, DirtObservationRow, EndedObservation, ResidualOwnerRow, SessionRow, Store, WorkClaimRow,
    WorkClaimUpdate, WorkRow, WorkUpdate,
    store::{bump_generation, client_name, invalid_value, parse_client, parse_work_state, work_state_name},
    store_communications::add_message,
    store_sessions::{end_session_if_revision, reconcile_ended, session_from_row, session_select},
};

/// State-owned facade for one atomic work arbitration.
///
/// Callers collect slow provider and Git evidence before entering this facade,
/// then re-read every mutable work decision through it before writing.
pub(crate) struct WorkTransaction<'store> {
    pub(super) transaction: Transaction<'store>,
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

    /// Paths this session has observed touched in `repo_root` at or after `since`,
    /// for the soft-claim hard/soft partition (see `crate::work::soft`).
    pub(crate) fn touched_in_repo(&self, identity: &Identity, repo_root: &str, since: f64) -> Result<Vec<String>> {
        let mut statement = self.transaction.prepare(
            "SELECT path FROM touched_paths
             WHERE client = ?1 AND session_id = ?2 AND repo_root = ?3 AND touched_at >= ?4
             ORDER BY path",
        )?;
        Ok(statement
            .query_map(params![client_name(identity.client), identity.session_id, repo_root, since], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// The holder's complete session row, for the soft-claim idle test (state and
    /// last_seen; see `crate::work::soft`).
    pub(crate) fn session(&self, identity: &Identity) -> Result<Option<SessionRow>> {
        Ok(self
            .transaction
            .query_row(
                &session_select("WHERE client = ?1 AND session_id = ?2"),
                params![client_name(identity.client), identity.session_id],
                session_from_row,
            )
            .optional()?)
    }

    pub(crate) fn work(&self, identity: &Identity) -> Result<Option<WorkRow>> {
        work_from(&self.transaction, identity)
    }

    pub(crate) fn works(&self) -> Result<Vec<WorkRow>> {
        works_from(&self.transaction, None)
    }

    pub(crate) fn end_session_if_revision(&self, identity: &Identity, expected_revision: i64) -> Result<bool> {
        end_session_if_revision(&self.transaction, identity, expected_revision)
    }

    /// Remove only sessions whose fingerprint and revision still match the death
    /// observations, returning the identities this transaction actually removed.
    pub(crate) fn reconcile_ended(&self, observations: &[EndedObservation]) -> Result<Vec<Identity>> {
        reconcile_ended(&self.transaction, observations)
    }

    pub(crate) fn baselines_in_repo(&self, identity: &Identity, repo_root: &str) -> Result<Vec<BaselineRow>> {
        baselines_in_repo_from(&self.transaction, identity, repo_root)
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

    pub(crate) fn observe_dirt(
        &self,
        repo_root: &str,
        blob_hashes: &[(String, String)],
        current: f64,
    ) -> Result<Vec<DirtObservationRow>> {
        let dirty_paths = blob_hashes.iter().map(|(path, _)| path.clone()).collect::<Vec<_>>();
        observe_dirt_subset_from(&self.transaction, repo_root, &dirty_paths, blob_hashes, current)
    }

    pub(crate) fn observe_dirt_subset(
        &self,
        repo_root: &str,
        dirty_paths: &[String],
        blob_hashes: &[(String, String)],
        current: f64,
    ) -> Result<Vec<DirtObservationRow>> {
        observe_dirt_subset_from(&self.transaction, repo_root, dirty_paths, blob_hashes, current)
    }

    pub(crate) fn record_residual_owners(
        &self,
        repo_root: &str,
        paths: &[String],
        identity: &Identity,
        current: f64,
    ) -> Result<()> {
        record_residual_owners_from(&self.transaction, repo_root, paths, identity, current)
    }

    pub(crate) fn delete_work(&self, identity: &Identity) -> Result<bool> {
        let removed = self.transaction.execute(
            "DELETE FROM work_items WHERE client = ?1 AND session_id = ?2",
            params![client_name(identity.client), identity.session_id],
        )? > 0;
        if removed {
            bump_generation(&self.transaction)?;
        }
        Ok(removed)
    }

    pub(crate) fn draft_for_session(&self, identity: &Identity) -> Result<Option<super::DraftRow>> {
        super::store_drafts::draft_for_session_from(&self.transaction, identity)
    }

    pub(crate) fn draft_by_id(&self, id: i64) -> Result<Option<super::DraftRow>> {
        super::store_drafts::draft_by_id_from(&self.transaction, id)
    }

    pub(crate) fn delete_draft(&self, id: i64) -> Result<bool> {
        super::store_drafts::delete_draft(&self.transaction, id)
    }
}

impl Store {
    pub(crate) fn work(&self, identity: &Identity) -> Result<Option<WorkRow>> {
        work_from(&self.connection, identity)
    }

    pub(crate) fn works(&self) -> Result<Vec<WorkRow>> {
        works_from(&self.connection, None)
    }

    pub(crate) fn works_in_repo(&self, repo_root: &str) -> Result<Vec<WorkRow>> {
        works_from(&self.connection, Some(repo_root))
    }

    pub(crate) fn residual_owners(&self, repo_root: &str) -> Result<Vec<ResidualOwnerRow>> {
        residual_owners_from(&self.connection, repo_root)
    }

    pub(crate) fn baselines_in_repo(&self, identity: &Identity, repo_root: &str) -> Result<Vec<BaselineRow>> {
        baselines_in_repo_from(&self.connection, identity, repo_root)
    }

    pub(crate) fn observe_dirt(
        &mut self,
        repo_root: &str,
        blob_hashes: &[(String, String)],
        current: f64,
    ) -> Result<Vec<DirtObservationRow>> {
        let dirty_paths = blob_hashes.iter().map(|(path, _)| path.clone()).collect::<Vec<_>>();
        self.observe_dirt_subset(repo_root, &dirty_paths, blob_hashes, current)
    }

    /// Reconciles observations against the complete Git dirt set while only
    /// refreshing hashes that are relevant to the caller's current scopes.
    pub(crate) fn observe_dirt_subset(
        &mut self,
        repo_root: &str,
        dirty_paths: &[String],
        blob_hashes: &[(String, String)],
        current: f64,
    ) -> Result<Vec<DirtObservationRow>> {
        self.immediate(|transaction| {
            observe_dirt_subset_from(transaction, repo_root, dirty_paths, blob_hashes, current)
        })
    }
}

fn save_work(transaction: &Transaction<'_>, update: &WorkUpdate) -> Result<i64> {
    let claims = normalized_claims(&update.claims)?;
    match update.expected_revision {
        Some(revision) => {
            let changed = transaction.execute(
                "UPDATE work_items SET
                    label = ?1, state = ?2, blocked_reason = ?3,
                    submitted_at = ?4, updated_at = ?5,
                    revision = revision + 1
                 WHERE client = ?6 AND session_id = ?7 AND revision = ?8",
                params![
                    update.label,
                    work_state_name(update.state),
                    update.blocked_reason,
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
                "SELECT EXISTS(
                    SELECT 1 FROM work_items WHERE client = ?1 AND session_id = ?2
                 )",
                params![client_name(update.identity.client), update.identity.session_id],
                |row| row.get::<_, bool>(0),
            )?;
            if exists {
                return Err(AppError::retry("work item appeared during update"));
            }
            transaction.execute(
                "INSERT INTO work_items(
                    client, session_id, label, state, blocked_reason,
                    submitted_at, updated_at, revision
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
                params![
                    client_name(update.identity.client),
                    update.identity.session_id,
                    update.label,
                    work_state_name(update.state),
                    update.blocked_reason,
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
    let mut retained = existing_claim_ids(transaction, work_id)?;
    for claim in &claims {
        let claim_id = if let Some(claim_id) = retained.remove(&claim.repo_root) {
            transaction.execute(
                "UPDATE work_claims SET blocked_reason = ?1 WHERE id = ?2",
                params![claim.blocked_reason, claim_id],
            )?;
            claim_id
        } else {
            transaction.execute(
                "INSERT INTO work_claims(work_id, repo_root, blocked_reason) VALUES (?1, ?2, ?3)",
                params![work_id, claim.repo_root, claim.blocked_reason],
            )?;
            transaction.last_insert_rowid()
        };

        transaction.execute("DELETE FROM work_scopes WHERE claim_id = ?1", [claim_id])?;
        for scope in &claim.scopes {
            transaction.execute(
                "INSERT INTO work_scopes(claim_id, path, kind) VALUES (?1, ?2, ?3)",
                params![claim_id, scope.path, scope_kind_name(scope.kind)],
            )?;
        }
        if let Some(baselines) = &claim.baselines {
            transaction.execute("DELETE FROM work_baselines WHERE claim_id = ?1", [claim_id])?;
            insert_baselines(transaction, claim_id, baselines)?;
        }
    }
    for claim_id in retained.into_values() {
        transaction.execute("DELETE FROM work_claims WHERE id = ?1", [claim_id])?;
    }
    bump_generation(transaction)?;
    Ok(work_id)
}

fn normalized_claims(claims: &[WorkClaimUpdate]) -> Result<Vec<WorkClaimUpdate>> {
    if claims.is_empty() {
        return Err(AppError::usage("at least one repository claim is required"));
    }
    let mut claims = claims.to_vec();
    for claim in &mut claims {
        if claim.repo_root.is_empty() {
            return Err(AppError::usage("repository claim root must not be empty"));
        }
        if claim.scopes.is_empty() {
            return Err(AppError::usage(format!("at least one scope is required for {}", claim.repo_root)));
        }
        claim.scopes.sort_by(|left, right| {
            left.path.cmp(&right.path).then_with(|| scope_kind_name(left.kind).cmp(scope_kind_name(right.kind)))
        });
        if claim.scopes.windows(2).any(|pair| pair[0].path == pair[1].path) {
            return Err(AppError::usage(format!("duplicate scope path in {}", claim.repo_root)));
        }
        if let Some(baselines) = &mut claim.baselines {
            baselines.sort_by(|left, right| left.path.cmp(&right.path));
            if baselines.windows(2).any(|pair| pair[0].path == pair[1].path) {
                return Err(AppError::usage(format!("duplicate baseline path in {}", claim.repo_root)));
            }
        }
    }
    claims.sort_by(|left, right| left.repo_root.cmp(&right.repo_root));
    if claims.windows(2).any(|pair| pair[0].repo_root == pair[1].repo_root) {
        return Err(AppError::usage("duplicate repository claim root"));
    }
    Ok(claims)
}

fn existing_claim_ids(connection: &Connection, work_id: i64) -> Result<BTreeMap<String, i64>> {
    let mut statement = connection.prepare("SELECT repo_root, id FROM work_claims WHERE work_id = ?1")?;
    Ok(statement.query_map([work_id], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<_>>()?)
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
        Some(repo_root) => (
            work_select(
                "JOIN work_claims ON work_claims.work_id = work_items.id
                 WHERE work_claims.repo_root = ?1
                 ORDER BY submitted_at, work_items.id",
            ),
            vec![repo_root],
        ),
        None => (work_select("ORDER BY submitted_at, work_items.id"), Vec::new()),
    };
    let mut statement = connection.prepare(&query)?;
    let bases = statement
        .query_map(rusqlite::params_from_iter(arguments), work_base_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    bases.into_iter().map(|base| finish_work(connection, base)).collect()
}

fn observe_dirt_subset_from(
    connection: &Connection,
    repo_root: &str,
    dirty_paths: &[String],
    blob_hashes: &[(String, String)],
    current: f64,
) -> Result<Vec<DirtObservationRow>> {
    let desired = dirty_paths.iter().map(String::as_str).collect::<HashSet<_>>();
    let existing = {
        let mut statement = connection.prepare("SELECT path FROM dirt_observations WHERE repo_root = ?1")?;
        statement.query_map([repo_root], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for path in existing {
        if !desired.contains(path.as_str()) {
            connection.execute(
                "DELETE FROM dirt_observations WHERE repo_root = ?1 AND path = ?2",
                params![repo_root, path],
            )?;
        }
    }
    for (path, blob_hash) in blob_hashes {
        connection.execute(
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
    dirt_observations_from(connection, repo_root)
}

fn record_residual_owners_from(
    connection: &Connection,
    repo_root: &str,
    paths: &[String],
    identity: &Identity,
    current: f64,
) -> Result<()> {
    for path in paths {
        connection.execute(
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
}

fn residual_owners_from(connection: &Connection, repo_root: &str) -> Result<Vec<ResidualOwnerRow>> {
    let mut statement = connection.prepare(
        "SELECT repo_root, path, client, session_id, released_at
         FROM residual_owners WHERE repo_root = ?1 ORDER BY path",
    )?;
    Ok(statement.query_map([repo_root], residual_owner_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn baselines_in_repo_from(connection: &Connection, identity: &Identity, repo_root: &str) -> Result<Vec<BaselineRow>> {
    let mut statement = connection.prepare(
        "SELECT work_baselines.path, work_baselines.oid
         FROM work_baselines
         JOIN work_claims ON work_claims.id = work_baselines.claim_id
         JOIN work_items ON work_items.id = work_claims.work_id
         WHERE work_items.client = ?1 AND work_items.session_id = ?2
           AND work_claims.repo_root = ?3 AND work_items.state = 'active'
         ORDER BY work_baselines.path",
    )?;
    Ok(statement
        .query_map(params![client_name(identity.client), identity.session_id, repo_root], |row| {
            Ok(BaselineRow { path: row.get(0)?, oid: row.get(1)? })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

struct WorkBase {
    id: i64,
    identity: Identity,
    label: String,
    state: WorkState,
    blocked_reason: Option<String>,
    submitted_at: Option<f64>,
    updated_at: f64,
    revision: i64,
}

fn work_select(suffix: &str) -> String {
    format!(
        "SELECT work_items.id, client, session_id, label, state, work_items.blocked_reason,
                submitted_at, updated_at, revision FROM work_items {suffix}"
    )
}

fn work_base_from_row(row: &Row<'_>) -> rusqlite::Result<WorkBase> {
    Ok(WorkBase {
        id: row.get(0)?,
        identity: Identity { client: parse_client(row.get(1)?)?, session_id: row.get(2)? },
        label: row.get(3)?,
        state: parse_work_state(row.get(4)?)?,
        blocked_reason: row.get(5)?,
        submitted_at: row.get(6)?,
        updated_at: row.get(7)?,
        revision: row.get(8)?,
    })
}

fn finish_work(connection: &Connection, base: WorkBase) -> Result<WorkRow> {
    let claim_bases = {
        let mut statement = connection
            .prepare("SELECT id, repo_root, blocked_reason FROM work_claims WHERE work_id = ?1 ORDER BY repo_root")?;
        statement
            .query_map([base.id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    if claim_bases.is_empty() {
        return Err(AppError::operational("work item has no repository claims"));
    }
    let mut claims = Vec::with_capacity(claim_bases.len());
    for (id, repo_root, blocked_reason) in claim_bases {
        let mut statement =
            connection.prepare("SELECT path, kind FROM work_scopes WHERE claim_id = ?1 ORDER BY path")?;
        let scopes = statement
            .query_map([id], |row| Ok(Scope { path: row.get(0)?, kind: parse_scope_kind(row.get(1)?)? }))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if scopes.is_empty() {
            return Err(AppError::operational(format!("repository claim {repo_root} has no scopes")));
        }
        claims.push(WorkClaimRow { id, repo_root, blocked_reason, scopes });
    }
    Ok(WorkRow {
        id: base.id,
        identity: base.identity,
        label: base.label,
        state: base.state,
        blocked_reason: base.blocked_reason,
        claims,
        submitted_at: base.submitted_at,
        updated_at: base.updated_at,
        revision: base.revision,
    })
}

fn insert_baselines(transaction: &Transaction<'_>, claim_id: i64, baselines: &[BaselineRow]) -> Result<()> {
    for baseline in baselines {
        transaction.execute(
            "INSERT INTO work_baselines(claim_id, path, oid) VALUES (?1, ?2, ?3)",
            params![claim_id, baseline.path, baseline.oid],
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

pub(super) const fn scope_kind_name(kind: ScopeKind) -> &'static str {
    match kind {
        ScopeKind::Exact => "exact",
        ScopeKind::Recursive => "recursive",
    }
}

pub(super) fn parse_scope_kind(value: String) -> rusqlite::Result<ScopeKind> {
    match value.as_str() {
        "exact" => Ok(ScopeKind::Exact),
        "recursive" => Ok(ScopeKind::Recursive),
        _ => Err(invalid_value(format!("invalid scope kind {value:?}"))),
    }
}

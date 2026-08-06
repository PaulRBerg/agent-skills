use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use crate::{
    domain::{ClaimState, Identity, Scope},
    error::Result,
};

use super::{
    BaselineRow, ClaimRow, ClaimUpdate, DirtObservationRow, ResidualOwnerRow, Store,
    store::{bump_generation, claim_state_name, client_name, parse_claim_state, parse_client},
    store_communications::add_message,
};

/// State-owned facade for one atomic claim arbitration.
///
/// Callers collect slow provider and Git evidence before entering this facade,
/// then re-read every mutable claim decision through it before writing.
pub(crate) struct ClaimTransaction<'store> {
    transaction: Transaction<'store>,
}

impl Store {
    pub(crate) fn with_claim_transaction<T>(
        &mut self,
        operation: impl FnOnce(&mut ClaimTransaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let transaction = self.connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut claims = ClaimTransaction { transaction };
        let result = operation(&mut claims)?;
        claims.transaction.commit()?;
        Ok(result)
    }
}

impl ClaimTransaction<'_> {
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

    pub(crate) fn claim(&self, identity: &Identity) -> Result<Option<ClaimRow>> {
        claim_from(&self.transaction, identity)
    }

    pub(crate) fn claims(&self, repo_root: &str) -> Result<Vec<ClaimRow>> {
        claims_from(&self.transaction, Some(repo_root))
    }

    pub(crate) fn baselines(&self, identity: &Identity) -> Result<Vec<BaselineRow>> {
        baselines_from(&self.transaction, identity)
    }

    pub(crate) fn residual_owners(&self, repo_root: &str) -> Result<Vec<ResidualOwnerRow>> {
        residual_owners_from(&self.transaction, repo_root)
    }

    pub(crate) fn save_claim(&mut self, update: &ClaimUpdate) -> Result<i64> {
        save_claim(&self.transaction, update)
    }

    pub(crate) fn send_message(
        &mut self,
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
    /// Atomically replaces a claim, all scopes, optional baselines, and residual ownership.
    pub(crate) fn save_claim(&mut self, update: &ClaimUpdate) -> Result<i64> {
        self.immediate(|transaction| save_claim(transaction, update))
    }

    pub(crate) fn claim(&self, identity: &Identity) -> Result<Option<ClaimRow>> {
        claim_from(&self.connection, identity)
    }

    pub(crate) fn claims(&self, repo_root: Option<&str>) -> Result<Vec<ClaimRow>> {
        claims_from(&self.connection, repo_root)
    }

    pub(crate) fn residual_owners(&self, repo_root: &str) -> Result<Vec<ResidualOwnerRow>> {
        residual_owners_from(&self.connection, repo_root)
    }

    pub(crate) fn baselines(&self, identity: &Identity) -> Result<Vec<BaselineRow>> {
        baselines_from(&self.connection, identity)
    }

    pub(crate) fn delete_claim(&mut self, identity: &Identity) -> Result<bool> {
        self.immediate(|transaction| {
            let removed = transaction.execute(
                "DELETE FROM claims WHERE client = ?1 AND session_id = ?2",
                params![client_name(identity.client), identity.session_id],
            )? > 0;
            transaction.execute(
                "UPDATE sessions SET label = NULL WHERE client = ?1 AND session_id = ?2",
                params![client_name(identity.client), identity.session_id],
            )?;
            bump_generation(transaction)?;
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

    pub(crate) fn dirt_observations(&self, repo_root: &str) -> Result<Vec<DirtObservationRow>> {
        dirt_observations_from(&self.connection, repo_root)
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
            let claim_id = transaction
                .query_row(
                    "SELECT id FROM claims
                     WHERE client = ?1 AND session_id = ?2 AND state = 'active'",
                    params![client_name(identity.client), identity.session_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;
            if let Some(claim_id) = claim_id {
                transaction.execute("DELETE FROM claim_baselines WHERE claim_id = ?1", [claim_id])?;
                insert_baselines(transaction, claim_id, baselines)?;
            }
            Ok(())
        })
    }
}

fn save_claim(transaction: &Transaction<'_>, update: &ClaimUpdate) -> Result<i64> {
    transaction.execute(
        "INSERT INTO claims(
                    client, session_id, repo_root, label, state, blocked_reason,
                    created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(client, session_id) DO UPDATE SET
                    repo_root = excluded.repo_root,
                    label = excluded.label,
                    state = excluded.state,
                    blocked_reason = excluded.blocked_reason,
                    created_at = excluded.created_at,
                    updated_at = excluded.updated_at",
        params![
            client_name(update.identity.client),
            update.identity.session_id,
            update.repo_root,
            update.label,
            claim_state_name(update.state),
            update.blocked_reason,
            update.created_at,
            update.updated_at,
        ],
    )?;
    let claim_id = transaction.query_row(
        "SELECT id FROM claims WHERE client = ?1 AND session_id = ?2",
        params![client_name(update.identity.client), update.identity.session_id],
        |row| row.get::<_, i64>(0),
    )?;
    transaction.execute("DELETE FROM claim_paths WHERE claim_id = ?1", [claim_id])?;
    for scope in &update.scopes {
        transaction.execute(
            "INSERT INTO claim_paths(claim_id, path, recursive) VALUES (?1, ?2, ?3)",
            params![claim_id, scope.path, scope.recursive],
        )?;
    }
    if let Some(baselines) = &update.baselines {
        transaction.execute("DELETE FROM claim_baselines WHERE claim_id = ?1", [claim_id])?;
        insert_baselines(transaction, claim_id, baselines)?;
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
    transaction.execute(
        "UPDATE sessions SET label = ?1 WHERE client = ?2 AND session_id = ?3",
        params![update.label, client_name(update.identity.client), update.identity.session_id],
    )?;
    bump_generation(transaction)?;
    Ok(claim_id)
}

fn claim_from(connection: &Connection, identity: &Identity) -> Result<Option<ClaimRow>> {
    let base = connection
        .query_row(
            &claim_select("WHERE client = ?1 AND session_id = ?2"),
            params![client_name(identity.client), identity.session_id],
            claim_base_from_row,
        )
        .optional()?;
    base.map(|base| finish_claim(connection, base)).transpose()
}

fn claims_from(connection: &Connection, repo_root: Option<&str>) -> Result<Vec<ClaimRow>> {
    let (query, arguments) = match repo_root {
        Some(repo_root) => (claim_select("WHERE repo_root = ?1 ORDER BY created_at, id"), vec![repo_root]),
        None => (claim_select("ORDER BY created_at, id"), Vec::new()),
    };
    let mut statement = connection.prepare(&query)?;
    let bases = statement
        .query_map(rusqlite::params_from_iter(arguments), claim_base_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    bases.into_iter().map(|base| finish_claim(connection, base)).collect()
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
        "SELECT claim_baselines.path, claim_baselines.oid
             FROM claim_baselines
             JOIN claims ON claims.id = claim_baselines.claim_id
             WHERE claims.client = ?1 AND claims.session_id = ?2 AND claims.state = 'active'
             ORDER BY claim_baselines.path",
    )?;
    Ok(statement
        .query_map(params![client_name(identity.client), identity.session_id], |row| {
            Ok(BaselineRow { path: row.get(0)?, oid: row.get(1)? })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

struct ClaimBase {
    id: i64,
    identity: Identity,
    repo_root: String,
    label: String,
    state: ClaimState,
    blocked_reason: Option<String>,
    created_at: f64,
    updated_at: f64,
}

fn claim_select(suffix: &str) -> String {
    format!(
        "SELECT id, client, session_id, repo_root, label, state, blocked_reason,
                created_at, updated_at FROM claims {suffix}"
    )
}

fn claim_base_from_row(row: &Row<'_>) -> rusqlite::Result<ClaimBase> {
    Ok(ClaimBase {
        id: row.get(0)?,
        identity: Identity { client: parse_client(row.get(1)?)?, session_id: row.get(2)? },
        repo_root: row.get(3)?,
        label: row.get(4)?,
        state: parse_claim_state(row.get(5)?)?,
        blocked_reason: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn finish_claim(connection: &Connection, base: ClaimBase) -> Result<ClaimRow> {
    let mut statement =
        connection.prepare("SELECT path, recursive FROM claim_paths WHERE claim_id = ?1 ORDER BY path")?;
    let scopes = statement
        .query_map([base.id], |row| Ok(Scope { path: row.get(0)?, recursive: row.get(1)? }))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ClaimRow {
        id: base.id,
        identity: base.identity,
        repo_root: base.repo_root,
        label: base.label,
        state: base.state,
        blocked_reason: base.blocked_reason,
        scopes,
        created_at: base.created_at,
        updated_at: base.updated_at,
    })
}

fn insert_baselines(transaction: &Transaction<'_>, claim_id: i64, baselines: &[BaselineRow]) -> Result<()> {
    for baseline in baselines {
        transaction.execute(
            "INSERT INTO claim_baselines(claim_id, path, oid) VALUES (?1, ?2, ?3)",
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

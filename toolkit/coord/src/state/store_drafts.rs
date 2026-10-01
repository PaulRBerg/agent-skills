use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use crate::{
    domain::{Identity, Scope},
    error::{AppError, Result},
};

use super::{
    DraftClaimRow, DraftClaimUpdate, DraftOwner, DraftRow, Store,
    store::{bump_generation, client_name, parse_client},
    store_work::{parse_scope_kind, scope_kind_name},
};

impl Store {
    /// Create a named or session-owned draft, or atomically replace an existing one.
    ///
    /// Replacing a named draft is only allowed when its stored repository roots equal
    /// the new claim vector's roots; replacing a session-owned draft is unconditional.
    pub(crate) fn save_draft(
        &mut self,
        owner: DraftOwner,
        label: &str,
        claims: &[DraftClaimUpdate],
        current: f64,
    ) -> Result<DraftRow> {
        self.immediate(|transaction| save_draft(transaction, &owner, label, claims, current))
    }

    pub(crate) fn draft_for_session(&self, identity: &Identity) -> Result<Option<DraftRow>> {
        draft_for_session_from(&self.connection, identity)
    }

    pub(crate) fn draft_named(&self, name: &str) -> Result<Option<DraftRow>> {
        draft_named_from(&self.connection, name)
    }

    pub(crate) fn drafts(&self) -> Result<Vec<DraftRow>> {
        drafts_from(&self.connection)
    }

    pub(crate) fn delete_draft(&mut self, id: i64) -> Result<bool> {
        self.immediate(|transaction| delete_draft(transaction, id))
    }
}

fn save_draft(
    transaction: &Transaction<'_>,
    owner: &DraftOwner,
    label: &str,
    claims: &[DraftClaimUpdate],
    current: f64,
) -> Result<DraftRow> {
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
    }
    claims.sort_by(|left, right| left.repo_root.cmp(&right.repo_root));
    if claims.windows(2).any(|pair| pair[0].repo_root == pair[1].repo_root) {
        return Err(AppError::usage("duplicate repository claim root"));
    }

    let existing = match owner {
        DraftOwner::Session(identity) => draft_for_session_from(transaction, identity)?,
        DraftOwner::Name(name) => draft_named_from(transaction, name)?,
    };

    let draft_id = if let Some(existing) = &existing {
        if let DraftOwner::Name(name) = owner {
            let mut existing_roots = existing.claims.iter().map(|claim| claim.repo_root.clone()).collect::<Vec<_>>();
            existing_roots.sort();
            let mut new_roots = claims.iter().map(|claim| claim.repo_root.clone()).collect::<Vec<_>>();
            new_roots.sort();
            if existing_roots != new_roots {
                return Err(AppError::usage(format!(
                    "draft {name} belongs to {}; choose another name",
                    existing_roots.join(",")
                )));
            }
        }
        transaction.execute(
            "UPDATE drafts SET label = ?1, updated_at = ?2 WHERE id = ?3",
            params![label, current, existing.id],
        )?;
        transaction.execute("DELETE FROM draft_claims WHERE draft_id = ?1", [existing.id])?;
        existing.id
    } else {
        let (name, owner_client, owner_session_id): (Option<&str>, Option<&str>, Option<String>) = match owner {
            DraftOwner::Name(name) => (Some(name.as_str()), None, None),
            DraftOwner::Session(identity) => {
                (None, Some(client_name(identity.client)), Some(identity.session_id.clone()))
            }
        };
        transaction.execute(
            "INSERT INTO drafts(name, owner_client, owner_session_id, label, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![name, owner_client, owner_session_id, label, current],
        )?;
        transaction.last_insert_rowid()
    };

    for claim in &claims {
        transaction.execute(
            "INSERT INTO draft_claims(draft_id, repo_root) VALUES (?1, ?2)",
            params![draft_id, claim.repo_root],
        )?;
        let claim_id = transaction.last_insert_rowid();
        for scope in &claim.scopes {
            transaction.execute(
                "INSERT INTO draft_scopes(claim_id, path, kind) VALUES (?1, ?2, ?3)",
                params![claim_id, scope.path, scope_kind_name(scope.kind)],
            )?;
        }
    }
    bump_generation(transaction)?;
    draft_from_id(transaction, draft_id)?.ok_or_else(|| AppError::retry("draft disappeared during replacement"))
}

pub(super) fn delete_draft(transaction: &Transaction<'_>, id: i64) -> Result<bool> {
    let removed = transaction.execute("DELETE FROM drafts WHERE id = ?1", [id])? > 0;
    if removed {
        bump_generation(transaction)?;
    }
    Ok(removed)
}

struct DraftBase {
    id: i64,
    name: Option<String>,
    owner: Option<Identity>,
    label: String,
    created_at: f64,
    updated_at: f64,
}

fn draft_select(suffix: &str) -> String {
    format!("SELECT id, name, owner_client, owner_session_id, label, created_at, updated_at FROM drafts {suffix}")
}

fn draft_base_from_row(row: &Row<'_>) -> rusqlite::Result<DraftBase> {
    let owner_client: Option<String> = row.get(2)?;
    let owner_session_id: Option<String> = row.get(3)?;
    let owner = match (owner_client, owner_session_id) {
        (Some(client), Some(session_id)) => Some(Identity { client: parse_client(client)?, session_id }),
        _ => None,
    };
    Ok(DraftBase {
        id: row.get(0)?,
        name: row.get(1)?,
        owner,
        label: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn finish_draft(connection: &Connection, base: DraftBase) -> Result<DraftRow> {
    let claim_bases = {
        let mut statement =
            connection.prepare("SELECT id, repo_root FROM draft_claims WHERE draft_id = ?1 ORDER BY repo_root")?;
        statement
            .query_map([base.id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut claims = Vec::with_capacity(claim_bases.len());
    for (id, repo_root) in claim_bases {
        let mut statement =
            connection.prepare("SELECT path, kind FROM draft_scopes WHERE claim_id = ?1 ORDER BY path")?;
        let scopes = statement
            .query_map([id], |row| Ok(Scope { path: row.get(0)?, kind: parse_scope_kind(row.get(1)?)? }))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        claims.push(DraftClaimRow { repo_root, scopes });
    }
    Ok(DraftRow {
        id: base.id,
        name: base.name,
        owner: base.owner,
        label: base.label,
        created_at: base.created_at,
        updated_at: base.updated_at,
        claims,
    })
}

fn draft_from_id(connection: &Connection, id: i64) -> Result<Option<DraftRow>> {
    draft_by_id_from(connection, id)
}

pub(super) fn draft_by_id_from(connection: &Connection, id: i64) -> Result<Option<DraftRow>> {
    let base = connection.query_row(&draft_select("WHERE id = ?1"), [id], draft_base_from_row).optional()?;
    base.map(|base| finish_draft(connection, base)).transpose()
}

pub(super) fn draft_for_session_from(connection: &Connection, identity: &Identity) -> Result<Option<DraftRow>> {
    let base = connection
        .query_row(
            &draft_select("WHERE owner_client = ?1 AND owner_session_id = ?2"),
            params![client_name(identity.client), identity.session_id],
            draft_base_from_row,
        )
        .optional()?;
    base.map(|base| finish_draft(connection, base)).transpose()
}

pub(super) fn draft_named_from(connection: &Connection, name: &str) -> Result<Option<DraftRow>> {
    let base = connection.query_row(&draft_select("WHERE name = ?1"), [name], draft_base_from_row).optional()?;
    base.map(|base| finish_draft(connection, base)).transpose()
}

fn drafts_from(connection: &Connection) -> Result<Vec<DraftRow>> {
    let mut statement = connection.prepare(&draft_select("ORDER BY updated_at, id"))?;
    let bases = statement.query_map([], draft_base_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
    bases.into_iter().map(|base| finish_draft(connection, base)).collect()
}

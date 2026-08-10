use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use crate::{
    domain::{FindingKind, FindingState, FindingSummary, Identity},
    error::{AppError, Result},
};

use super::{
    CurrentTurnFinding, FindingAdd, FindingAddResult, FindingCounts, FindingResolution, Store,
    store::{bump_generation, client_name, new_id},
};

impl Store {
    pub(crate) fn begin_turn(
        &mut self,
        identity: &Identity,
        provider_turn_id: Option<&str>,
        current: f64,
    ) -> Result<String> {
        let turn_id = provider_turn_id
            .filter(|value| !value.is_empty())
            .map_or_else(|| format!("local-{}", new_id()), str::to_owned);
        self.immediate(|transaction| {
            transaction.execute(
                "INSERT INTO current_turns(client, session_id, turn_id, started_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(client, session_id) DO UPDATE SET
                    turn_id = excluded.turn_id,
                    started_at = excluded.started_at",
                params![client_name(identity.client), identity.session_id, turn_id, current],
            )?;
            Ok(())
        })?;
        Ok(turn_id)
    }

    pub(crate) fn current_turn_findings(&self, identity: &Identity) -> Result<Vec<CurrentTurnFinding>> {
        let mut statement = self.connection.prepare(
            "SELECT findings.id, findings.summary
             FROM current_turns
             JOIN finding_sightings
               ON finding_sightings.author_client = current_turns.client
              AND finding_sightings.author_session_id = current_turns.session_id
              AND finding_sightings.turn_id = current_turns.turn_id
             JOIN findings ON findings.id = finding_sightings.finding_id
             WHERE current_turns.client = ?1
               AND current_turns.session_id = ?2
               AND finding_sightings.surfaced_at IS NULL
             GROUP BY findings.id, findings.summary
             ORDER BY MIN(finding_sightings.created_at), findings.id",
        )?;
        Ok(statement
            .query_map(params![client_name(identity.client), identity.session_id], |row| {
                Ok(CurrentTurnFinding { id: row.get(0)?, summary: row.get(1)? })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn mark_current_turn_findings_surfaced(&mut self, identity: &Identity, current: f64) -> Result<usize> {
        self.immediate(|transaction| {
            Ok(transaction.execute(
                "UPDATE finding_sightings
                 SET surfaced_at = ?1
                 WHERE author_client = ?2
                   AND author_session_id = ?3
                   AND surfaced_at IS NULL
                   AND turn_id = (
                       SELECT turn_id FROM current_turns
                       WHERE client = ?2 AND session_id = ?3
                   )",
                params![current, client_name(identity.client), identity.session_id],
            )?)
        })
    }

    pub(crate) fn finding_counts(&self, repo_root: &str, current: f64) -> Result<FindingCounts> {
        let (pending, triaging, handed_off) = self.connection.query_row(
            "SELECT
                COALESCE(SUM(state = 'pending'), 0),
                COALESCE(SUM(EXISTS(
                    SELECT 1 FROM finding_claims
                    WHERE finding_claims.finding_id = findings.id
                      AND finding_claims.lease_expires_at > ?2
                )), 0),
                COALESCE(SUM(state = 'handed-off'), 0)
             FROM findings WHERE repo_root = ?1",
            params![repo_root, current],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
        )?;
        Ok(FindingCounts {
            pending: count_value(pending)?,
            triaging: count_value(triaging)?,
            handed_off: count_value(handed_off)?,
        })
    }

    pub(crate) fn add_finding(&mut self, input: &FindingAdd) -> Result<FindingAddResult> {
        self.immediate(|transaction| {
            let existing_ids = finding_ids(
                transaction,
                "repo_root = ?1 AND normalized_summary = ?2 AND state IN ('pending', 'handed-off')",
                params![input.repo_root, input.normalized_summary],
            )?;
            let mut exact_id = None;
            for id in existing_ids {
                if finding_paths(transaction, &id)? == input.paths {
                    exact_id = Some(id);
                    break;
                }
            }

            let (finding_id, deduplicated) = if let Some(id) = exact_id {
                transaction.execute("UPDATE findings SET updated_at = ?1 WHERE id = ?2", params![input.current, id])?;
                (id, true)
            } else {
                let id = new_id();
                transaction.execute(
                    "INSERT INTO findings(
                        id, repo_root, summary, normalized_summary, kind, state,
                        created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, ?6)",
                    params![
                        id,
                        input.repo_root,
                        input.summary,
                        input.normalized_summary,
                        input.kind.map(finding_kind_name),
                        input.current,
                    ],
                )?;
                for path in &input.paths {
                    transaction
                        .execute("INSERT INTO finding_paths(finding_id, path) VALUES (?1, ?2)", params![id, path])?;
                }
                add_event(
                    transaction,
                    &id,
                    "created",
                    None,
                    FindingState::Pending,
                    &input.author,
                    None,
                    None,
                    None,
                    input.current,
                )?;
                (id, false)
            };

            let sighting_id = add_sighting(transaction, &finding_id, input)?;
            for observation in &input.observations {
                transaction.execute(
                    "INSERT INTO finding_observations(sighting_id, path, content_sha256)
                     VALUES (?1, ?2, ?3)",
                    params![sighting_id, observation.path, observation.content_sha256],
                )?;
            }
            bump_generation(transaction)?;

            let finding = finding_summary(transaction, &finding_id, input.current)?
                .ok_or_else(|| AppError::operational("finding disappeared while adding a sighting"))?;
            let candidates = if deduplicated {
                Vec::new()
            } else {
                same_path_candidates(transaction, &input.repo_root, &finding_id, &input.paths, input.current)?
            };
            Ok(FindingAddResult { finding, deduplicated, candidates })
        })
    }

    pub(crate) fn finding(&self, repo_root: &str, id: &str, current: f64) -> Result<Option<FindingSummary>> {
        let finding = finding_summary(&self.connection, id, current)?;
        Ok(finding.filter(|row| row.repo_root == repo_root))
    }

    pub(crate) fn findings(
        &self,
        repo_root: &str,
        state: Option<FindingState>,
        include_terminal: bool,
        current: f64,
    ) -> Result<Vec<FindingSummary>> {
        let ids = if let Some(state) = state {
            finding_ids(
                &self.connection,
                "repo_root = ?1 AND state = ?2 ORDER BY updated_at DESC, id",
                params![repo_root, finding_state_name(state)],
            )?
        } else if include_terminal {
            finding_ids(&self.connection, "repo_root = ?1 ORDER BY updated_at DESC, id", [repo_root])?
        } else {
            finding_ids(
                &self.connection,
                "repo_root = ?1 AND state IN ('pending', 'handed-off') ORDER BY updated_at DESC, id",
                [repo_root],
            )?
        };
        ids.into_iter()
            .map(|id| {
                finding_summary(&self.connection, &id, current)?
                    .ok_or_else(|| AppError::operational("finding disappeared while listing"))
            })
            .collect()
    }

    pub(crate) fn all_findings(&self, current: f64) -> Result<Vec<FindingSummary>> {
        let ids = finding_ids(&self.connection, "1 = 1 ORDER BY updated_at DESC, id", [])?;
        ids.into_iter()
            .map(|id| {
                finding_summary(&self.connection, &id, current)?
                    .ok_or_else(|| AppError::operational("finding disappeared while listing"))
            })
            .collect()
    }

    pub(crate) fn handoff_finding(
        &mut self,
        repo_root: &str,
        id: &str,
        path: &str,
        actor: &Identity,
        current: f64,
    ) -> Result<FindingSummary> {
        self.immediate(|transaction| {
            let finding = required_finding(transaction, repo_root, id, current)?;
            if finding.state != FindingState::Pending {
                return Err(AppError::operational(format!(
                    "finding {id} cannot be handed off from state {}",
                    finding_state_name(finding.state)
                )));
            }
            transaction.execute(
                "UPDATE findings
                 SET state = 'handed-off', handoff_path = ?1, updated_at = ?2
                 WHERE id = ?3",
                params![path, current, id],
            )?;
            add_event(
                transaction,
                id,
                "handed-off",
                Some(FindingState::Pending),
                FindingState::HandedOff,
                actor,
                Some(path),
                None,
                None,
                current,
            )?;
            bump_generation(transaction)?;
            required_finding(transaction, repo_root, id, current)
        })
    }

    pub(crate) fn resolve_finding(
        &mut self,
        repo_root: &str,
        id: &str,
        resolution: &FindingResolution,
    ) -> Result<FindingSummary> {
        if !resolution.state.is_terminal() {
            return Err(AppError::usage("finding resolution must use a terminal state"));
        }
        self.immediate(|transaction| {
            let finding = required_finding(transaction, repo_root, id, resolution.current)?;
            if finding.state.is_terminal() {
                return Err(AppError::operational(format!("finding {id} is already terminal")));
            }
            match (resolution.state, resolution.canonical_id.as_deref()) {
                (FindingState::Duplicate, Some(canonical_id)) => {
                    if canonical_id == id {
                        return Err(AppError::usage("a duplicate finding cannot reference itself"));
                    }
                    let canonical = required_finding(transaction, repo_root, canonical_id, resolution.current)?;
                    if canonical.state == FindingState::Duplicate {
                        return Err(AppError::usage("canonical finding cannot itself be a duplicate"));
                    }
                }
                (FindingState::Duplicate, None) => {
                    return Err(AppError::usage("--canonical is required with --as duplicate"));
                }
                (_, Some(_)) => return Err(AppError::usage("--canonical is available only with --as duplicate")),
                (_, None) => {}
            }
            transaction.execute(
                "UPDATE findings
                 SET state = ?1, updated_at = ?2, terminal_at = ?2,
                     commit_oid = ?3, canonical_id = ?4
                 WHERE id = ?5",
                params![
                    finding_state_name(resolution.state),
                    resolution.current,
                    resolution.commit_oid.as_deref(),
                    resolution.canonical_id.as_deref(),
                    id
                ],
            )?;
            add_event(
                transaction,
                id,
                "resolved",
                Some(finding.state),
                resolution.state,
                &resolution.actor,
                None,
                resolution.commit_oid.as_deref(),
                resolution.canonical_id.as_deref(),
                resolution.current,
            )?;
            bump_generation(transaction)?;
            required_finding(transaction, repo_root, id, resolution.current)
        })
    }

    pub(crate) fn reopen_finding(
        &mut self,
        repo_root: &str,
        id: &str,
        actor: &Identity,
        current: f64,
    ) -> Result<FindingSummary> {
        self.immediate(|transaction| {
            let finding = required_finding(transaction, repo_root, id, current)?;
            if !finding.state.is_terminal() {
                return Err(AppError::operational(format!("finding {id} is not terminal")));
            }
            if let Some(existing_id) = exact_open_peer(transaction, repo_root, id)? {
                return Err(AppError::operational(format!(
                    "finding {id} cannot be reopened because exact open finding {existing_id} exists"
                )));
            }
            transaction.execute(
                "UPDATE findings
                 SET state = 'pending', updated_at = ?1, terminal_at = NULL,
                     handoff_path = NULL, commit_oid = NULL, canonical_id = NULL
                 WHERE id = ?2",
                params![current, id],
            )?;
            add_event(
                transaction,
                id,
                "reopened",
                Some(finding.state),
                FindingState::Pending,
                actor,
                None,
                None,
                None,
                current,
            )?;
            bump_generation(transaction)?;
            required_finding(transaction, repo_root, id, current)
        })
    }
}

fn exact_open_peer(connection: &Connection, repo_root: &str, id: &str) -> Result<Option<String>> {
    let normalized_summary: String =
        connection.query_row("SELECT normalized_summary FROM findings WHERE id = ?1", [id], |row| row.get(0))?;
    let paths = finding_paths(connection, id)?;
    let ids = finding_ids(
        connection,
        "repo_root = ?1 AND id != ?2 AND normalized_summary = ?3
         AND state IN ('pending', 'handed-off')",
        params![repo_root, id, normalized_summary],
    )?;
    for candidate_id in ids {
        if finding_paths(connection, &candidate_id)? == paths {
            return Ok(Some(candidate_id));
        }
    }
    Ok(None)
}

fn add_sighting(transaction: &Transaction<'_>, finding_id: &str, input: &FindingAdd) -> Result<i64> {
    let turn_id = match input.turn_id.as_deref() {
        Some(turn_id) => Some(turn_id.to_owned()),
        None => transaction
            .query_row(
                "SELECT turn_id FROM current_turns WHERE client = ?1 AND session_id = ?2",
                params![client_name(input.author.client), input.author.session_id],
                |row| row.get(0),
            )
            .optional()?,
    };
    transaction.execute(
        "INSERT INTO finding_sightings(
            finding_id, author_client, author_session_id, turn_id, head_oid, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            finding_id,
            client_name(input.author.client),
            input.author.session_id,
            turn_id,
            input.head_oid,
            input.current,
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

#[allow(clippy::too_many_arguments)]
fn add_event(
    transaction: &Transaction<'_>,
    finding_id: &str,
    event: &str,
    from_state: Option<FindingState>,
    to_state: FindingState,
    actor: &Identity,
    handoff_path: Option<&str>,
    commit_oid: Option<&str>,
    canonical_id: Option<&str>,
    current: f64,
) -> Result<()> {
    transaction.execute(
        "INSERT INTO finding_events(
            finding_id, event, from_state, to_state, actor_client,
            actor_session_id, handoff_path, commit_oid, canonical_id, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            finding_id,
            event,
            from_state.map(finding_state_name),
            finding_state_name(to_state),
            client_name(actor.client),
            actor.session_id,
            handoff_path,
            commit_oid,
            canonical_id,
            current,
        ],
    )?;
    Ok(())
}

fn same_path_candidates(
    connection: &Connection,
    repo_root: &str,
    finding_id: &str,
    paths: &[String],
    current: f64,
) -> Result<Vec<FindingSummary>> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let requested = paths.iter().map(String::as_str).collect::<HashSet<_>>();
    let ids = finding_ids(
        connection,
        "repo_root = ?1 AND id != ?2 AND state IN ('pending', 'handed-off')
         ORDER BY updated_at DESC, id",
        params![repo_root, finding_id],
    )?;
    let mut matches = Vec::new();
    for id in ids {
        if finding_paths(connection, &id)?.iter().any(|path| requested.contains(path.as_str())) {
            matches.push(
                finding_summary(connection, &id, current)?
                    .ok_or_else(|| AppError::operational("finding disappeared while matching candidates"))?,
            );
            if matches.len() == 5 {
                break;
            }
        }
    }
    Ok(matches)
}

fn required_finding(connection: &Connection, repo_root: &str, id: &str, current: f64) -> Result<FindingSummary> {
    let finding = finding_summary(connection, id, current)?
        .filter(|finding| finding.repo_root == repo_root)
        .ok_or_else(|| AppError::operational(format!("finding not found: {id}")))?;
    Ok(finding)
}

fn finding_summary(connection: &Connection, id: &str, current: f64) -> Result<Option<FindingSummary>> {
    let raw = connection
        .query_row(
            "SELECT id, repo_root, summary, kind, state, created_at, updated_at,
                    terminal_at, handoff_path, commit_oid, canonical_id
             FROM findings WHERE id = ?1",
            [id],
            raw_finding_from_row,
        )
        .optional()?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    let sighting_count =
        connection.query_row("SELECT COUNT(*) FROM finding_sightings WHERE finding_id = ?1", [id], |row| {
            row.get::<_, i64>(0)
        })?;
    let triaging = connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM finding_claims WHERE finding_id = ?1 AND lease_expires_at > ?2
         )",
        params![id, current],
        |row| row.get::<_, bool>(0),
    )?;
    Ok(Some(FindingSummary {
        id: raw.id,
        repo_root: raw.repo_root,
        summary: raw.summary,
        kind: raw.kind,
        state: raw.state,
        paths: finding_paths(connection, id)?,
        created_at: raw.created_at,
        updated_at: raw.updated_at,
        terminal_at: raw.terminal_at,
        handoff_path: raw.handoff_path,
        commit_oid: raw.commit_oid,
        canonical_id: raw.canonical_id,
        sighting_count: usize::try_from(sighting_count)
            .map_err(|_| AppError::operational("finding sighting count is invalid"))?,
        triaging,
    }))
}

fn finding_paths(connection: &Connection, id: &str) -> Result<Vec<String>> {
    let mut statement = connection.prepare("SELECT path FROM finding_paths WHERE finding_id = ?1 ORDER BY path")?;
    Ok(statement.query_map([id], |row| row.get(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn finding_ids<P>(connection: &Connection, predicate: &str, parameters: P) -> Result<Vec<String>>
where
    P: rusqlite::Params,
{
    let mut statement = connection.prepare(&format!("SELECT id FROM findings WHERE {predicate}"))?;
    Ok(statement.query_map(parameters, |row| row.get(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
}

struct RawFinding {
    id: String,
    repo_root: String,
    summary: String,
    kind: Option<FindingKind>,
    state: FindingState,
    created_at: f64,
    updated_at: f64,
    terminal_at: Option<f64>,
    handoff_path: Option<String>,
    commit_oid: Option<String>,
    canonical_id: Option<String>,
}

fn raw_finding_from_row(row: &Row<'_>) -> rusqlite::Result<RawFinding> {
    Ok(RawFinding {
        id: row.get(0)?,
        repo_root: row.get(1)?,
        summary: row.get(2)?,
        kind: row.get::<_, Option<String>>(3)?.map(parse_finding_kind).transpose()?,
        state: parse_finding_state(row.get(4)?)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        terminal_at: row.get(7)?,
        handoff_path: row.get(8)?,
        commit_oid: row.get(9)?,
        canonical_id: row.get(10)?,
    })
}

pub(crate) const fn finding_kind_name(kind: FindingKind) -> &'static str {
    match kind {
        FindingKind::Bug => "bug",
        FindingKind::Docs => "docs",
        FindingKind::Improvement => "improvement",
    }
}

pub(crate) const fn finding_state_name(state: FindingState) -> &'static str {
    match state {
        FindingState::Pending => "pending",
        FindingState::HandedOff => "handed-off",
        FindingState::Fixed => "fixed",
        FindingState::Stale => "stale",
        FindingState::Rejected => "rejected",
        FindingState::Duplicate => "duplicate",
    }
}

fn parse_finding_kind(value: String) -> rusqlite::Result<FindingKind> {
    match value.as_str() {
        "bug" => Ok(FindingKind::Bug),
        "docs" => Ok(FindingKind::Docs),
        "improvement" => Ok(FindingKind::Improvement),
        _ => Err(invalid_value(format!("invalid finding kind {value:?}"))),
    }
}

fn parse_finding_state(value: String) -> rusqlite::Result<FindingState> {
    match value.as_str() {
        "pending" => Ok(FindingState::Pending),
        "handed-off" => Ok(FindingState::HandedOff),
        "fixed" => Ok(FindingState::Fixed),
        "stale" => Ok(FindingState::Stale),
        "rejected" => Ok(FindingState::Rejected),
        "duplicate" => Ok(FindingState::Duplicate),
        _ => Err(invalid_value(format!("invalid finding state {value:?}"))),
    }
}

fn invalid_value(message: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, message)),
    )
}

fn count_value(value: i64) -> Result<usize> {
    usize::try_from(value).map_err(|_| AppError::operational("finding count is invalid"))
}

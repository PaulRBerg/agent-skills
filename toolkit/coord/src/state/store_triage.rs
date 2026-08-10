use rusqlite::{OptionalExtension, Transaction, params};

use crate::{
    domain::{Client, Identity},
    error::Result,
};

use super::{
    Store,
    store::{client_name, new_id},
};

pub(crate) const TRIAGE_BATCH_LIMIT: usize = 20;
pub(crate) const TRIAGE_COOLDOWN_SECONDS: f64 = 24.0 * 60.0 * 60.0;
pub(crate) const TRIAGE_LEASE_SECONDS: f64 = 31.0 * 60.0;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TriageRun {
    pub(crate) id: String,
    pub(crate) repo_root: String,
    pub(crate) origin: Identity,
    pub(crate) started_at: f64,
    pub(crate) finished_at: Option<f64>,
    pub(crate) outcome: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TriageClaim {
    pub(crate) finding_id: String,
    pub(crate) claimed_at: f64,
    pub(crate) lease_expires_at: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TriageRunStart {
    pub(crate) run: TriageRun,
    pub(crate) claims: Vec<TriageClaim>,
}

impl Store {
    /// Atomically re-check quiescence, cooldown, singleton state, and claim the
    /// oldest pending findings. An empty result means another caller won or an
    /// eligibility guard changed before this transaction acquired the lock.
    pub(crate) fn begin_triage_run(
        &mut self,
        repo_root: &str,
        origin: &Identity,
        current: f64,
    ) -> Result<Option<TriageRunStart>> {
        self.immediate(|transaction| {
            if transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM work_items WHERE repo_root = ?1)",
                [repo_root],
                |row| row.get::<_, bool>(0),
            )? {
                return Ok(None);
            }
            if transaction.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM triage_runs
                    WHERE repo_root = ?1 AND finished_at IS NULL
                 )",
                [repo_root],
                |row| row.get::<_, bool>(0),
            )? {
                return Ok(None);
            }
            if transaction.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM triage_runs
                    WHERE repo_root = ?1 AND started_at > ?2
                 )",
                params![repo_root, current - TRIAGE_COOLDOWN_SECONDS],
                |row| row.get::<_, bool>(0),
            )? {
                return Ok(None);
            }

            let finding_ids = {
                let mut statement = transaction.prepare(
                    "SELECT f.id
                     FROM findings f
                     LEFT JOIN finding_claims c ON c.finding_id = f.id
                     WHERE f.repo_root = ?1 AND f.state = 'pending' AND c.finding_id IS NULL
                     ORDER BY f.created_at, f.id
                     LIMIT ?2",
                )?;
                statement
                    .query_map(params![repo_root, TRIAGE_BATCH_LIMIT as i64], |row| row.get::<_, String>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            };
            if finding_ids.is_empty() {
                return Ok(None);
            }

            let id = new_id();
            let stored_origin = format!("triage:{}/{}", client_name(origin.client), origin.session_id);
            transaction.execute(
                "INSERT INTO triage_runs(
                    id, repo_root, runner_client, runner_session_id, started_at
                 ) VALUES (?1, ?2, 'codex', ?3, ?4)",
                params![id, repo_root, stored_origin, current],
            )?;
            let lease_expires_at = current + TRIAGE_LEASE_SECONDS;
            let mut claims = Vec::with_capacity(finding_ids.len());
            for finding_id in finding_ids {
                transaction.execute(
                    "INSERT INTO finding_claims(finding_id, triage_run_id, claimed_at, lease_expires_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![finding_id, id, current, lease_expires_at],
                )?;
                claims.push(TriageClaim { finding_id, claimed_at: current, lease_expires_at });
            }
            Ok(Some(TriageRunStart {
                run: TriageRun {
                    id,
                    repo_root: repo_root.to_owned(),
                    origin: origin.clone(),
                    started_at: current,
                    finished_at: None,
                    outcome: None,
                },
                claims,
            }))
        })
    }

    pub(crate) fn active_triage_runs(&self, repo_root: &str) -> Result<Vec<TriageRun>> {
        let mut statement = self.connection.prepare(
            "SELECT id, repo_root, runner_session_id, started_at, finished_at, outcome
             FROM triage_runs
             WHERE repo_root = ?1 AND finished_at IS NULL
             ORDER BY started_at, id",
        )?;
        Ok(statement
            .query_map([repo_root], |row| {
                let stored_origin = row.get::<_, String>(2)?;
                Ok(TriageRun {
                    id: row.get(0)?,
                    repo_root: row.get(1)?,
                    origin: parse_stored_origin(&stored_origin)?,
                    started_at: row.get(3)?,
                    finished_at: row.get(4)?,
                    outcome: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn triage_run(&self, id: &str) -> Result<Option<TriageRun>> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, repo_root, runner_session_id, started_at, finished_at, outcome
                 FROM triage_runs WHERE id = ?1",
                [id],
                |row| {
                    let stored_origin = row.get::<_, String>(2)?;
                    Ok(TriageRun {
                        id: row.get(0)?,
                        repo_root: row.get(1)?,
                        origin: parse_stored_origin(&stored_origin)?,
                        started_at: row.get(3)?,
                        finished_at: row.get(4)?,
                        outcome: row.get(5)?,
                    })
                },
            )
            .optional()?)
    }

    #[cfg(test)]
    pub(crate) fn triage_claims(&self, run_id: &str) -> Result<Vec<TriageClaim>> {
        let mut statement = self.connection.prepare(
            "SELECT finding_id, claimed_at, lease_expires_at
             FROM finding_claims WHERE triage_run_id = ?1
             ORDER BY claimed_at, finding_id",
        )?;
        Ok(statement
            .query_map([run_id], |row| {
                Ok(TriageClaim { finding_id: row.get(0)?, claimed_at: row.get(1)?, lease_expires_at: row.get(2)? })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn renew_triage_claims(&mut self, run_id: &str, current: f64) -> Result<bool> {
        self.immediate(|transaction| {
            if !run_is_open(transaction, run_id)? {
                return Ok(false);
            }
            transaction.execute(
                "UPDATE finding_claims SET lease_expires_at = ?1 WHERE triage_run_id = ?2",
                params![current + TRIAGE_LEASE_SECONDS, run_id],
            )?;
            Ok(true)
        })
    }

    pub(crate) fn finish_triage_run(&mut self, run_id: &str, outcome: &str, current: f64) -> Result<bool> {
        self.immediate(|transaction| {
            let changed = transaction.execute(
                "UPDATE triage_runs SET finished_at = ?1, outcome = ?2
                 WHERE id = ?3 AND finished_at IS NULL",
                params![current, outcome, run_id],
            )?;
            transaction.execute("DELETE FROM finding_claims WHERE triage_run_id = ?1", [run_id])?;
            Ok(changed == 1)
        })
    }

    pub(crate) fn release_orphaned_claims(&mut self, current: f64) -> Result<usize> {
        self.immediate(|transaction| {
            Ok(transaction.execute(
                "DELETE FROM finding_claims
                 WHERE lease_expires_at <= ?1
                    OR triage_run_id IN (SELECT id FROM triage_runs WHERE finished_at IS NOT NULL)",
                [current],
            )?)
        })
    }

    pub(crate) fn pending_claimed_finding_ids(&self, run_id: &str) -> Result<Vec<String>> {
        let mut statement = self.connection.prepare(
            "SELECT f.id
             FROM finding_claims c
             JOIN findings f ON f.id = c.finding_id
             WHERE c.triage_run_id = ?1 AND f.state = 'pending'
             ORDER BY c.claimed_at, f.id",
        )?;
        Ok(statement.query_map([run_id], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

fn run_is_open(transaction: &Transaction<'_>, run_id: &str) -> Result<bool> {
    Ok(transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM triage_runs WHERE id = ?1 AND finished_at IS NULL)",
        [run_id],
        |row| row.get(0),
    )?)
}

fn parse_stored_origin(value: &str) -> rusqlite::Result<Identity> {
    let value = value.strip_prefix("triage:").ok_or_else(|| invalid_value("missing triage role"))?;
    let (client, session_id) = value.split_once('/').ok_or_else(|| invalid_value("missing triage origin"))?;
    if session_id.is_empty() {
        return Err(invalid_value("empty triage origin session"));
    }
    let client = match client {
        "codex" => Client::Codex,
        "claude" => Client::Claude,
        _ => return Err(invalid_value("invalid triage origin client")),
    };
    Ok(Identity { client, session_id: session_id.to_owned() })
}

fn invalid_value(message: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, message)),
    )
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use crate::{
        domain::{FindingKind, Identity},
        state::{FindingAdd, FindingPathObservation},
    };

    use super::*;

    fn identity(id: &str) -> Identity {
        Identity { client: Client::Codex, session_id: id.to_owned() }
    }

    fn add_pending(store: &mut Store, root: &Path, id: usize, current: f64) -> String {
        let actor = identity("source");
        store
            .add_finding(&FindingAdd {
                repo_root: root.to_string_lossy().into_owned(),
                summary: format!("finding {id}"),
                normalized_summary: format!("finding {id}"),
                kind: Some(FindingKind::Docs),
                paths: vec![format!("docs/{id}.md")],
                head_oid: None,
                observations: vec![FindingPathObservation { path: format!("docs/{id}.md"), content_sha256: None }],
                author: actor,
                turn_id: None,
                current,
            })
            .unwrap()
            .finding
            .id
    }

    #[test]
    fn singleton_oldest_first_batch_and_cooldown_are_transactional() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("docs")).unwrap();
        let path = temp.path().join("state.db");
        let mut store = Store::open(&path).unwrap();
        let mut expected = Vec::new();
        for index in 0..25 {
            expected.push(add_pending(&mut store, temp.path(), index, index as f64));
        }
        let root = temp.path().to_string_lossy();
        let start = store.begin_triage_run(&root, &identity("origin"), 100.0).unwrap().unwrap();
        assert_eq!(start.claims.len(), TRIAGE_BATCH_LIMIT);
        assert_eq!(start.claims.iter().map(|claim| claim.finding_id.clone()).collect::<Vec<_>>(), expected[..20]);

        let mut competitor = Store::open(&path).unwrap();
        assert!(competitor.begin_triage_run(&root, &identity("other"), 100.0).unwrap().is_none());
        store.finish_triage_run(&start.run.id, "completed", 101.0).unwrap();
        assert!(
            competitor.begin_triage_run(&root, &identity("other"), 99.9 + TRIAGE_COOLDOWN_SECONDS).unwrap().is_none()
        );
        let retry =
            competitor.begin_triage_run(&root, &identity("other"), 100.0 + TRIAGE_COOLDOWN_SECONDS).unwrap().unwrap();
        assert_eq!(retry.claims.len(), TRIAGE_BATCH_LIMIT);
        assert_eq!(retry.claims.iter().map(|claim| claim.finding_id.clone()).collect::<Vec<_>>(), expected[..20]);
    }

    #[test]
    fn expired_claims_are_released() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(temp.path().join("state.db")).unwrap();
        add_pending(&mut store, temp.path(), 1, 1.0);
        let start = store.begin_triage_run(&temp.path().to_string_lossy(), &identity("origin"), 2.0).unwrap().unwrap();
        assert_eq!(store.release_orphaned_claims(2.0 + TRIAGE_LEASE_SECONDS).unwrap(), 1);
        assert!(store.triage_claims(&start.run.id).unwrap().is_empty());
    }

    #[test]
    fn any_normal_work_state_blocks_triage() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(temp.path().join("state.db")).unwrap();
        add_pending(&mut store, temp.path(), 1, 1.0);
        let root = temp.path().to_string_lossy();
        store.connection.execute(
            "INSERT INTO sessions(client, session_id, cwd, repo_root, state, source, started_at, last_seen, revision)
             VALUES ('codex', 'normal', ?1, ?1, 'working', 'test', 1, 1, 1)",
            [&root],
        ).unwrap();
        store.connection.execute(
            "INSERT INTO work_items(client, session_id, repo_root, label, state, draft_created_at, updated_at, revision)
             VALUES ('codex', 'normal', ?1, 'normal work', 'draft', 1, 1, 1)",
            [&root],
        ).unwrap();
        assert!(store.begin_triage_run(&root, &identity("origin"), 2.0).unwrap().is_none());
    }
}

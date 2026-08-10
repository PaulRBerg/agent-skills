use rusqlite::{OptionalExtension, params};

use crate::{
    domain::Identity,
    error::Result,
    state::{Store, TouchedPaths},
};

use super::store::client_name;

pub(crate) const MAX_TOUCHED_PATHS: usize = 1_000;

impl Store {
    pub(crate) fn record_touched(
        &mut self,
        identity: &Identity,
        repo_root: &str,
        paths: &[String],
        current: f64,
    ) -> Result<()> {
        self.immediate(|transaction| {
            transaction.execute(
                "INSERT INTO touched_sets(client, session_id, repo_root)
                 VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
                params![client_name(identity.client), identity.session_id, repo_root],
            )?;
            for path in paths {
                transaction.execute(
                    "INSERT INTO touched_paths(client, session_id, repo_root, path, touched_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(client, session_id, repo_root, path)
                     DO UPDATE SET touched_at = excluded.touched_at",
                    params![client_name(identity.client), identity.session_id, repo_root, path, current],
                )?;
            }
            let count: i64 = transaction.query_row(
                "SELECT COUNT(*) FROM touched_paths
                 WHERE client = ?1 AND session_id = ?2 AND repo_root = ?3",
                params![client_name(identity.client), identity.session_id, repo_root],
                |row| row.get(0),
            )?;
            if count > MAX_TOUCHED_PATHS as i64 {
                transaction.execute(
                    "DELETE FROM touched_paths WHERE rowid IN (
                        SELECT rowid FROM touched_paths
                        WHERE client = ?1 AND session_id = ?2 AND repo_root = ?3
                        ORDER BY touched_at, path LIMIT ?4
                     )",
                    params![
                        client_name(identity.client),
                        identity.session_id,
                        repo_root,
                        count - MAX_TOUCHED_PATHS as i64
                    ],
                )?;
                transaction.execute(
                    "UPDATE touched_sets SET truncated = 1
                     WHERE client = ?1 AND session_id = ?2 AND repo_root = ?3",
                    params![client_name(identity.client), identity.session_id, repo_root],
                )?;
            }
            Ok(())
        })
    }

    pub(crate) fn touched(&self, identity: &Identity, repo_root: &str) -> Result<TouchedPaths> {
        let mut statement = self.connection.prepare(
            "SELECT path FROM touched_paths
             WHERE client = ?1 AND session_id = ?2 AND repo_root = ?3 ORDER BY path",
        )?;
        let paths = statement
            .query_map(params![client_name(identity.client), identity.session_id, repo_root], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let truncated = self
            .connection
            .query_row(
                "SELECT truncated FROM touched_sets
                 WHERE client = ?1 AND session_id = ?2 AND repo_root = ?3",
                params![client_name(identity.client), identity.session_id, repo_root],
                |row| row.get::<_, bool>(0),
            )
            .optional()?
            .unwrap_or(false);
        Ok(TouchedPaths { paths, truncated })
    }

    /// Returns true only once per transition from dirty to clean.
    pub(crate) fn update_scopes_clean(&mut self, identity: &Identity, repo_root: &str, clean: bool) -> Result<bool> {
        self.immediate(|transaction| {
            transaction.execute(
                "INSERT INTO touched_sets(client, session_id, repo_root, scopes_clean)
                 VALUES (?1, ?2, ?3, 0) ON CONFLICT DO NOTHING",
                params![client_name(identity.client), identity.session_id, repo_root],
            )?;
            let previous: bool = transaction.query_row(
                "SELECT scopes_clean FROM touched_sets
                 WHERE client = ?1 AND session_id = ?2 AND repo_root = ?3",
                params![client_name(identity.client), identity.session_id, repo_root],
                |row| row.get(0),
            )?;
            if previous != clean {
                transaction.execute(
                    "UPDATE touched_sets SET scopes_clean = ?4
                     WHERE client = ?1 AND session_id = ?2 AND repo_root = ?3",
                    params![client_name(identity.client), identity.session_id, repo_root, clean],
                )?;
            }
            Ok(clean && !previous)
        })
    }
}

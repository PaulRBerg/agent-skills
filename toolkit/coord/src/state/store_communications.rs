use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use crate::{
    domain::{Client, Identity},
    error::Result,
};

use super::{
    DelegateRow, HookHealthRow, MessageRow, NoteRow, ProviderCacheRow, Store,
    store::{MAX_ERROR_CODE_CHARS, bump_generation, client_name, new_id, parse_client, sanitize},
};

impl Store {
    pub(crate) fn send_message(
        &mut self,
        sender: &Identity,
        recipients: &[Identity],
        text: &str,
        repo_root: Option<&str>,
        current: f64,
    ) -> Result<Vec<String>> {
        self.immediate(|transaction| {
            recipients
                .iter()
                .map(|recipient| add_message(transaction, sender, recipient, text, repo_root, current))
                .collect()
        })
    }

    pub(crate) fn inbox(&self, identity: &Identity, pending_only: bool) -> Result<Vec<MessageRow>> {
        let pending = if pending_only { "AND acknowledged_at IS NULL" } else { "" };
        let mut statement = self.connection.prepare(&format!(
            "{} WHERE recipient_client = ?1 AND recipient_session_id = ?2 {pending}
             ORDER BY created_at, id",
            message_select()
        ))?;
        Ok(statement
            .query_map(params![client_name(identity.client), identity.session_id], message_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn all_messages(&self) -> Result<Vec<MessageRow>> {
        let mut statement = self.connection.prepare(&format!("{} ORDER BY created_at, id", message_select()))?;
        Ok(statement.query_map([], message_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Mark unread messages as surfaced without waking generation-based waiters.
    pub(crate) fn mark_unnotified(&mut self, identity: &Identity, current: f64) -> Result<usize> {
        self.immediate(|transaction| {
            Ok(transaction.execute(
                "UPDATE messages SET notified_at = ?1
                 WHERE recipient_client = ?2 AND recipient_session_id = ?3
                   AND acknowledged_at IS NULL AND notified_at IS NULL",
                params![current, client_name(identity.client), identity.session_id],
            )?)
        })
    }

    pub(crate) fn acknowledge(&mut self, identity: &Identity, message_id: Option<&str>, current: f64) -> Result<usize> {
        self.immediate(|transaction| {
            let changed = match message_id {
                Some(message_id) => transaction.execute(
                    "UPDATE messages SET acknowledged_at = ?1
                     WHERE id = ?2 AND recipient_client = ?3 AND recipient_session_id = ?4
                       AND acknowledged_at IS NULL",
                    params![current, message_id, client_name(identity.client), identity.session_id],
                )?,
                None => transaction.execute(
                    "UPDATE messages SET acknowledged_at = ?1
                     WHERE recipient_client = ?2 AND recipient_session_id = ?3
                       AND acknowledged_at IS NULL",
                    params![current, client_name(identity.client), identity.session_id],
                )?,
            };
            if changed > 0 {
                bump_generation(transaction)?;
            }
            Ok(changed)
        })
    }

    pub(crate) fn add_note(&mut self, author: &Identity, repo_root: &str, text: &str, current: f64) -> Result<String> {
        let id = new_id();
        self.immediate(|transaction| {
            transaction.execute(
                "INSERT INTO notes(
                    id, repo_root, author_client, author_session_id, text, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, repo_root, client_name(author.client), author.session_id, text, current],
            )?;
            bump_generation(transaction)
        })?;
        Ok(id)
    }

    pub(crate) fn notes(&self, repo_root: &str, since: Option<f64>) -> Result<Vec<NoteRow>> {
        let mut rows = match since {
            Some(since) => {
                let mut statement = self.connection.prepare(
                    "SELECT id, repo_root, author_client, author_session_id, text,
                            created_at, resolved_at
                     FROM notes
                     WHERE repo_root = ?1 AND resolved_at IS NULL AND created_at > ?2
                     ORDER BY created_at, id",
                )?;
                statement.query_map(params![repo_root, since], note_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?
            }
            None => {
                let mut statement = self.connection.prepare(
                    "SELECT id, repo_root, author_client, author_session_id, text,
                            created_at, resolved_at
                     FROM notes WHERE repo_root = ?1 AND resolved_at IS NULL
                     ORDER BY created_at, id",
                )?;
                statement.query_map([repo_root], note_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?
            }
        };
        rows.shrink_to_fit();
        Ok(rows)
    }

    pub(crate) fn all_notes(&self) -> Result<Vec<NoteRow>> {
        let mut statement = self.connection.prepare(
            "SELECT id, repo_root, author_client, author_session_id, text,
                    created_at, resolved_at
             FROM notes ORDER BY created_at, id",
        )?;
        Ok(statement.query_map([], note_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn resolve_note(&mut self, repo_root: &str, note_id: &str, current: f64) -> Result<bool> {
        self.immediate(|transaction| {
            let changed = transaction.execute(
                "UPDATE notes SET resolved_at = ?1
                 WHERE repo_root = ?2 AND id = ?3 AND resolved_at IS NULL",
                params![current, repo_root, note_id],
            )? > 0;
            if changed {
                bump_generation(transaction)?;
            }
            Ok(changed)
        })
    }

    pub(crate) fn update_delegate(
        &mut self,
        parent: &Identity,
        agent_id: &str,
        agent_type: Option<&str>,
        state: &str,
        current: f64,
    ) -> Result<()> {
        self.immediate(|transaction| {
            if state == "ended" {
                transaction.execute(
                    "DELETE FROM delegates
                     WHERE parent_client = ?1 AND parent_session_id = ?2 AND agent_id = ?3",
                    params![client_name(parent.client), parent.session_id, agent_id],
                )?;
            } else {
                transaction.execute(
                    "INSERT INTO delegates(
                        parent_client, parent_session_id, agent_id, agent_type, state, last_seen
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(parent_client, parent_session_id, agent_id) DO UPDATE SET
                        agent_type = excluded.agent_type,
                        state = excluded.state,
                        last_seen = excluded.last_seen",
                    params![client_name(parent.client), parent.session_id, agent_id, agent_type, state, current],
                )?;
            }
            Ok(())
        })
    }

    pub(crate) fn delegates(&self) -> Result<Vec<DelegateRow>> {
        let mut statement = self.connection.prepare(
            "SELECT parent_client, parent_session_id, agent_id, agent_type, state, last_seen
             FROM delegates ORDER BY parent_client, parent_session_id, agent_id",
        )?;
        Ok(statement
            .query_map([], |row| {
                Ok(DelegateRow {
                    parent: Identity { client: parse_client(row.get(0)?)?, session_id: row.get(1)? },
                    agent_id: row.get(2)?,
                    agent_type: row.get(3)?,
                    state: row.get(4)?,
                    last_seen: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn hook_success(&mut self, client: Client, event: &str, current: f64) -> Result<()> {
        self.immediate(|transaction| {
            transaction.execute(
                "INSERT INTO hook_health(client, event, last_success_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(client, event) DO UPDATE SET
                    last_error_code = NULL,
                    last_error_at = NULL,
                    last_success_at = excluded.last_success_at",
                params![client_name(client), event, current],
            )?;
            Ok(())
        })
    }

    pub(crate) fn hook_error(&mut self, client: Client, event: &str, code: &str, current: f64) -> Result<()> {
        let code = sanitize(code, MAX_ERROR_CODE_CHARS);
        self.immediate(|transaction| {
            transaction.execute(
                "INSERT INTO hook_health(client, event, last_error_code, last_error_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(client, event) DO UPDATE SET
                    last_error_code = excluded.last_error_code,
                    last_error_at = excluded.last_error_at",
                params![client_name(client), event, code, current],
            )?;
            Ok(())
        })
    }

    pub(crate) fn hook_health(&self) -> Result<Vec<HookHealthRow>> {
        let mut statement = self.connection.prepare(
            "SELECT client, event, last_error_code, last_error_at, last_success_at
             FROM hook_health ORDER BY client, event",
        )?;
        Ok(statement
            .query_map([], |row| {
                Ok(HookHealthRow {
                    client: parse_client(row.get(0)?)?,
                    event: row.get(1)?,
                    last_error_code: row.get(2)?,
                    last_error_at: row.get(3)?,
                    last_success_at: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn provider_cache(&self, context_key: &str) -> Result<Vec<ProviderCacheRow>> {
        let mut statement = self.connection.prepare(
            "SELECT context_key, client, refreshed_at, ok, source, enabled, dropped
             FROM provider_cache WHERE context_key = ?1 ORDER BY client",
        )?;
        Ok(statement.query_map([context_key], provider_cache_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn replace_provider_cache(
        &mut self,
        context_key: &str,
        reports: &[ProviderCacheRow],
        refreshed_at: f64,
    ) -> Result<()> {
        self.immediate(|transaction| {
            transaction.execute("DELETE FROM provider_cache", [])?;
            for report in reports {
                let dropped = i64::try_from(report.dropped)
                    .map_err(|_| crate::error::AppError::usage("provider dropped count is too large"))?;
                transaction.execute(
                    "INSERT INTO provider_cache(
                        context_key, client, refreshed_at, ok, source, enabled, dropped
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        context_key,
                        client_name(report.client),
                        refreshed_at,
                        report.ok,
                        report.source,
                        report.enabled,
                        dropped,
                    ],
                )?;
            }
            Ok(())
        })
    }

    pub(crate) fn clear_provider_cache(&mut self) -> Result<()> {
        self.immediate(|transaction| {
            transaction.execute("DELETE FROM provider_cache", [])?;
            Ok(())
        })
    }
}

pub(super) fn add_message(
    transaction: &Transaction<'_>,
    sender: &Identity,
    recipient: &Identity,
    text: &str,
    repo_root: Option<&str>,
    current: f64,
) -> Result<String> {
    let id = new_id();
    let sender_callsign = callsign(transaction, sender)?;
    let recipient_callsign = callsign(transaction, recipient)?;
    transaction.execute(
        "INSERT INTO messages(
            id, sender_client, sender_session_id, sender_callsign, recipient_client,
            recipient_session_id, recipient_callsign, repo_root, text, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            id,
            client_name(sender.client),
            sender.session_id,
            sender_callsign,
            client_name(recipient.client),
            recipient.session_id,
            recipient_callsign,
            repo_root,
            text,
            current,
        ],
    )?;
    transaction.execute(
        "DELETE FROM messages WHERE id IN (
            SELECT id FROM messages
            WHERE recipient_client = ?1 AND recipient_session_id = ?2
            ORDER BY created_at DESC, id DESC LIMIT -1 OFFSET ?3
         )",
        params![client_name(recipient.client), recipient.session_id, super::MAX_INBOX_MESSAGES as i64],
    )?;
    bump_generation(transaction)?;
    Ok(id)
}

fn callsign(connection: &Connection, identity: &Identity) -> Result<Option<String>> {
    Ok(connection
        .query_row(
            "SELECT callsign FROM sessions WHERE client = ?1 AND session_id = ?2",
            params![client_name(identity.client), identity.session_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten())
}

fn message_select() -> &'static str {
    "SELECT id, sender_client, sender_session_id, sender_callsign, recipient_client,
            recipient_session_id, recipient_callsign, repo_root, text, created_at,
            acknowledged_at, notified_at FROM messages"
}

fn message_from_row(row: &Row<'_>) -> rusqlite::Result<MessageRow> {
    Ok(MessageRow {
        id: row.get(0)?,
        sender: Identity { client: parse_client(row.get(1)?)?, session_id: row.get(2)? },
        sender_callsign: row.get(3)?,
        recipient: Identity { client: parse_client(row.get(4)?)?, session_id: row.get(5)? },
        recipient_callsign: row.get(6)?,
        repo_root: row.get(7)?,
        text: row.get(8)?,
        created_at: row.get(9)?,
        acknowledged_at: row.get(10)?,
        notified_at: row.get(11)?,
    })
}

fn note_from_row(row: &Row<'_>) -> rusqlite::Result<NoteRow> {
    let author_client = row.get::<_, Option<String>>(2)?;
    let author_session_id = row.get::<_, Option<String>>(3)?;
    let author = match (author_client, author_session_id) {
        (Some(client), Some(session_id)) => Some(Identity { client: parse_client(client)?, session_id }),
        _ => None,
    };
    Ok(NoteRow {
        id: row.get(0)?,
        repo_root: row.get(1)?,
        author,
        text: row.get(4)?,
        created_at: row.get(5)?,
        resolved_at: row.get(6)?,
    })
}

fn provider_cache_from_row(row: &Row<'_>) -> rusqlite::Result<ProviderCacheRow> {
    Ok(ProviderCacheRow {
        context_key: row.get(0)?,
        client: parse_client(row.get(1)?)?,
        refreshed_at: row.get(2)?,
        ok: row.get(3)?,
        source: row.get(4)?,
        enabled: row.get(5)?,
        dropped: row.get::<_, i64>(6)? as usize,
    })
}

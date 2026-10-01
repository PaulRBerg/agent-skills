use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    domain::{Identity, ProcessLiveness, Scope},
    error::{AppError, Result},
    host::scopes_cover,
};

use super::{
    RecommendationDecision, RecommendationEndpoint, RecommendationMutation, RecommendationObservation,
    RecommendationOutcome, RecommendationRow, RecommendationSend, RecommendationState, RecommendationWork, Store,
    WorkTransaction,
    store::{bump_generation, client_name, invalid_value, new_id},
    store_communications::add_message_with_callsigns,
};

pub(crate) const RECOMMENDATION_TTL: f64 = 48.0 * 60.0 * 60.0;
pub(crate) const MAX_LIVE_RECOMMENDATIONS: usize = 50;
pub(crate) const MAX_RECOMMENDATION_SCOPES: usize = 50;
pub(crate) const MAX_RECOMMENDATION_TEXT: usize = 2000;

impl Store {
    /// Both endpoints retain access even after their session rows disappear.
    pub(crate) fn recommendation(&self, identity: &Identity, id: &str) -> Result<Option<RecommendationRow>> {
        recommendation_from(&self.connection, identity, id)
    }

    pub(crate) fn recommendations(
        &self,
        identity: &Identity,
        repo_root: &str,
        sent: bool,
        all: bool,
    ) -> Result<Vec<RecommendationRow>> {
        endpoint_records(&self.connection, identity, Some(repo_root), sent, !all)
    }

    /// This bookkeeping is independent of ordinary pointer-message delivery.
    pub(crate) fn pending_recommendations(
        &self,
        identity: &Identity,
        repo_root: Option<&str>,
        unsurfaced_only: bool,
    ) -> Result<Vec<RecommendationRow>> {
        let records = endpoint_records(&self.connection, identity, repo_root, false, true)?;
        let work = self.work(identity)?.as_ref().map(RecommendationWork::from);
        Ok(records
            .into_iter()
            .filter(|record| {
                (!unsurfaced_only || record.surfaced_at.is_none()) && work.as_ref() == Some(&record.recipient.work)
            })
            .collect())
    }

    /// Mark only the IDs represented by a rendered checkpoint, never a later arrival.
    pub(crate) fn mark_recommendations_surfaced(
        &mut self,
        identity: &Identity,
        ids: &[String],
        current: f64,
    ) -> Result<usize> {
        self.with_work_transaction(|work| {
            work.refresh_recommendations(current)?;
            let mut changed = 0;
            for id in ids {
                changed += work.transaction.execute(
                    "UPDATE recommendations SET surfaced_at = ?1
                     WHERE id = ?2 AND recipient_client = ?3 AND recipient_session_id = ?4
                       AND state = 'pending' AND surfaced_at IS NULL",
                    params![current, id, client_name(identity.client), identity.session_id],
                )?;
            }
            Ok(changed)
        })
    }

    pub(crate) fn refresh_recommendations(&mut self, current: f64) -> Result<usize> {
        self.with_work_transaction(|work| work.refresh_recommendations(current))
    }
}

impl WorkTransaction<'_> {
    pub(crate) fn send_recommendation(&self, input: &RecommendationSend) -> Result<RecommendationMutation> {
        if input.sender.identity == input.recipient.identity {
            return Err(AppError::usage("a recommendation requires a different recipient"));
        }
        validate_text(&input.reason)?;
        validate_text(&input.replacement)?;
        let scopes = normalized_scopes(&input.scopes)?;
        let sender = self.validated_endpoint(&input.sender)?;
        let recipient = self.validated_endpoint(&input.recipient)?;
        if !sender.work.claims.iter().any(|claim| claim.repo_root == input.repo_root) {
            return Err(AppError::usage("sender must have submitted work in the recommendation repository"));
        }
        let recipient_claim =
            recipient.work.claims.iter().find(|claim| claim.repo_root == input.repo_root).ok_or_else(|| {
                AppError::usage("recipient must have submitted work in the recommendation repository")
            })?;
        if !scopes_cover(&recipient_claim.scopes, &scopes) {
            return Err(AppError::usage("recommendation scopes must be covered by the recipient's submitted work"));
        }
        self.refresh_recommendations(input.current)?;
        for existing in endpoint_records(&self.transaction, &sender.identity, Some(&input.repo_root), true, false)? {
            if existing.state.is_live() &&
                existing.recipient.identity == recipient.identity &&
                existing.sender.work == sender.work &&
                existing.recipient.work == recipient.work &&
                existing.scopes == scopes &&
                existing.action == input.action &&
                existing.reason == input.reason &&
                existing.replacement == input.replacement
            {
                return Ok(mutation(existing, RecommendationOutcome::Existing));
            }
        }
        for (endpoint, sent) in [(&sender.identity, true), (&recipient.identity, false)] {
            let records = endpoint_records(&self.transaction, endpoint, None, sent, false)?;
            if records.iter().filter(|record| record.state.is_live()).count() >= MAX_LIVE_RECOMMENDATIONS {
                return Err(AppError::usage(format!(
                    "{} endpoint has 50 live recommendations; resolve or withdraw existing recommendations before sending",
                    if sent { "sender" } else { "recipient" }
                )));
            }
        }
        let record = RecommendationRow {
            id: new_id(),
            repo_root: input.repo_root.clone(),
            sender,
            recipient,
            scopes,
            action: input.action,
            reason: input.reason.clone(),
            replacement: input.replacement.clone(),
            state: RecommendationState::Pending,
            created_at: input.current,
            updated_at: input.current,
            decision: None,
            decision_reason: None,
            decision_at: None,
            invalidation_reason: None,
            surfaced_at: None,
            sender_fingerprint: input.sender.fingerprint.clone(),
            recipient_fingerprint: input.recipient.fingerprint.clone(),
        };
        self.transaction.execute(
            "INSERT INTO recommendations (
                id, repo_root, sender_client, sender_session_id, recipient_client, recipient_session_id,
                sender_snapshot, recipient_snapshot, sender_fingerprint, recipient_fingerprint,
                scopes, action, reason, replacement, state, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 'pending', ?15, ?15)",
            params![
                record.id,
                record.repo_root,
                client_name(record.sender.identity.client),
                record.sender.identity.session_id,
                client_name(record.recipient.identity.client),
                record.recipient.identity.session_id,
                serde_json::to_string(&record.sender)?,
                serde_json::to_string(&record.recipient)?,
                serde_json::to_string(&record.sender_fingerprint)?,
                serde_json::to_string(&record.recipient_fingerprint)?,
                serde_json::to_string(&record.scopes)?,
                enum_name(record.action)?,
                record.reason,
                record.replacement,
                input.current
            ],
        )?;
        self.pointer(&record, false, input.current)?;
        Ok(mutation(record, RecommendationOutcome::Recommended))
    }

    pub(crate) fn respond_recommendation(
        &self,
        actor: &Identity,
        id: &str,
        decision: RecommendationDecision,
        reason: &str,
        observations: &[RecommendationObservation],
        current: f64,
    ) -> Result<RecommendationMutation> {
        validate_text(reason)?;
        let mut record = self.required_recommendation(actor, id)?;
        if record.recipient.identity != *actor {
            return Err(AppError::usage("only the recommendation recipient may respond"));
        }
        self.refresh_record(&mut record, current)?;
        if record.state == RecommendationState::Stale {
            return Ok(mutation(record, RecommendationOutcome::Stale));
        }
        let outcome = match decision {
            RecommendationDecision::Accepted => RecommendationOutcome::Accepted,
            RecommendationDecision::Rejected => RecommendationOutcome::Rejected,
        };
        if record.decision.is_some() {
            let still_decided = matches!(record.state, RecommendationState::Accepted | RecommendationState::Rejected);
            let same = record.decision == Some(decision) && record.decision_reason.as_deref() == Some(reason);
            return Ok(mutation(record, if still_decided && same { outcome } else { RecommendationOutcome::Conflict }));
        }
        if record.state != RecommendationState::Pending {
            return Ok(mutation(record, RecommendationOutcome::Conflict));
        }
        if decision == RecommendationDecision::Accepted {
            for endpoint in [&record.sender, &record.recipient] {
                let observation = observations
                    .iter()
                    .find(|observation| observation.identity == endpoint.identity)
                    .ok_or_else(|| AppError::operational("endpoint liveness must be established before accepting"))?;
                self.validated_endpoint(observation)?;
            }
        }
        record.state = match decision {
            RecommendationDecision::Accepted => RecommendationState::Accepted,
            RecommendationDecision::Rejected => RecommendationState::Rejected,
        };
        record.decision = Some(decision);
        record.decision_reason = Some(reason.to_owned());
        record.decision_at = Some(current);
        record.updated_at = current;
        self.save_transition(&record)?;
        self.pointer(&record, true, current)?;
        Ok(mutation(record, outcome))
    }

    pub(crate) fn withdraw_recommendation(
        &self,
        actor: &Identity,
        id: &str,
        reason: &str,
        current: f64,
    ) -> Result<RecommendationMutation> {
        validate_text(reason)?;
        let mut record = self.required_recommendation(actor, id)?;
        if record.sender.identity != *actor {
            return Err(AppError::usage("only the recommendation sender may withdraw"));
        }
        self.refresh_record(&mut record, current)?;
        if record.state == RecommendationState::Stale {
            return Ok(mutation(record, RecommendationOutcome::Stale));
        }
        if !record.state.is_live() {
            let same =
                record.state == RecommendationState::Withdrawn && record.invalidation_reason.as_deref() == Some(reason);
            return Ok(mutation(
                record,
                if same { RecommendationOutcome::Withdrawn } else { RecommendationOutcome::Conflict },
            ));
        }
        record.state = RecommendationState::Withdrawn;
        record.invalidation_reason = Some(reason.to_owned());
        record.updated_at = current;
        self.save_transition(&record)?;
        self.pointer(&record, false, current)?;
        Ok(mutation(record, RecommendationOutcome::Withdrawn))
    }

    /// Context transitions, expiry, retention, and their pointer messages share
    /// the transaction that observes the current session/work rows.
    pub(crate) fn refresh_recommendations(&self, current: f64) -> Result<usize> {
        let records = {
            let mut statement =
                self.transaction.prepare(&format!("{} WHERE state IN ('pending', 'accepted')", select()))?;
            statement.query_map([], from_row)?.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let mut changed = 0;
        for mut record in records {
            changed += usize::from(self.refresh_record(&mut record, current)?);
        }
        let deleted = self.transaction.execute(
            "DELETE FROM recommendations WHERE state IN ('rejected', 'withdrawn', 'stale') AND updated_at <= ?1",
            [current - RECOMMENDATION_TTL],
        )?;
        if deleted > 0 {
            bump_generation(&self.transaction)?;
        }
        Ok(changed + deleted)
    }

    fn validated_endpoint(&self, observation: &RecommendationObservation) -> Result<RecommendationEndpoint> {
        if observation.liveness != ProcessLiveness::Alive {
            return Err(AppError::operational("endpoint process liveness is not established; refresh and retry"));
        }
        let session = self
            .session(&observation.identity)?
            .ok_or_else(|| AppError::operational("endpoint session ended; refresh and retry"))?;
        let work = self
            .work(&observation.identity)?
            .ok_or_else(|| AppError::operational("endpoint submitted work ended; refresh and retry"))?;
        if session.fingerprint != observation.fingerprint || RecommendationWork::from(&work) != observation.work {
            return Err(AppError::operational("endpoint process or submitted work changed; refresh and retry"));
        }
        Ok(RecommendationEndpoint { identity: session.identity, callsign: session.callsign, work: (&work).into() })
    }

    fn required_recommendation(&self, actor: &Identity, id: &str) -> Result<RecommendationRow> {
        recommendation_from(&self.transaction, actor, id)?
            .ok_or_else(|| AppError::operational("recommendation not found for this endpoint"))
    }

    fn refresh_record(&self, record: &mut RecommendationRow, current: f64) -> Result<bool> {
        if !record.state.is_live() {
            return Ok(false);
        }
        let reason = if current - record.created_at >= RECOMMENDATION_TTL {
            Some("Recommendation expired after 48 hours; inspect the replacement and verify retained requirements.")
        } else if !self.endpoint_matches(&record.sender, &record.sender_fingerprint)? {
            Some("Source work or session changed or ended; inspect the replacement and verify retained requirements.")
        } else if record.state == RecommendationState::Pending &&
            !self.endpoint_matches(&record.recipient, &record.recipient_fingerprint)?
        {
            Some(
                "Recipient work or session changed or ended; review this historical recommendation before applying it to other work.",
            )
        } else {
            None
        };
        let Some(reason) = reason else {
            return Ok(false);
        };
        record.state = RecommendationState::Stale;
        record.invalidation_reason = Some(reason.to_owned());
        record.updated_at = current;
        self.save_transition(record)?;
        self.pointer(record, false, current)?;
        self.pointer(record, true, current)?;
        Ok(true)
    }

    fn endpoint_matches(
        &self,
        endpoint: &RecommendationEndpoint,
        fingerprint: &Option<crate::domain::ProcessFingerprint>,
    ) -> Result<bool> {
        let Some(session) = self.session(&endpoint.identity)? else {
            return Ok(false);
        };
        if session.fingerprint != *fingerprint {
            return Ok(false);
        }
        let Some(work) = self.work(&endpoint.identity)? else {
            return Ok(false);
        };
        Ok(RecommendationWork::from(&work) == endpoint.work)
    }

    fn save_transition(&self, record: &RecommendationRow) -> Result<()> {
        self.transaction.execute(
            "UPDATE recommendations SET state = ?1, updated_at = ?2, decision = ?3,
                decision_reason = ?4, decision_at = ?5, invalidation_reason = ?6 WHERE id = ?7",
            params![
                enum_name(record.state)?,
                record.updated_at,
                record.decision.map(enum_name).transpose()?,
                record.decision_reason,
                record.decision_at,
                record.invalidation_reason,
                record.id
            ],
        )?;
        bump_generation(&self.transaction)
    }

    fn pointer(&self, record: &RecommendationRow, to_sender: bool, current: f64) -> Result<()> {
        // Work IDs are never reused, so an accepting recipient's relabeled or
        // narrowed work is still the work the recommendation addressed.
        if !to_sender &&
            record.decision == Some(RecommendationDecision::Accepted) &&
            !self.work(&record.recipient.identity)?.is_some_and(|work| work.id == record.recipient.work.id)
        {
            return Ok(());
        }
        let (sender, recipient) =
            if to_sender { (&record.recipient, &record.sender) } else { (&record.sender, &record.recipient) };
        let text = format!(
            "Recommendation {} {}; run ai-coord recommend show {}.",
            record.id,
            enum_name(record.state)?,
            record.id
        );
        add_message_with_callsigns(
            &self.transaction,
            (&sender.identity, sender.callsign.as_deref()),
            (&recipient.identity, recipient.callsign.as_deref()),
            &text,
            Some(&record.repo_root),
            current,
        )?;
        Ok(())
    }
}

fn mutation(recommendation: RecommendationRow, outcome: RecommendationOutcome) -> RecommendationMutation {
    RecommendationMutation { outcome, recommendation }
}

fn validate_text(text: &str) -> Result<()> {
    if text.trim().is_empty() || text.chars().count() > MAX_RECOMMENDATION_TEXT {
        return Err(AppError::usage("recommendation text must contain 1 to 2000 Unicode characters"));
    }
    if text.chars().any(char::is_control) {
        return Err(AppError::usage("recommendation text must be normalized before storage"));
    }
    Ok(())
}

fn normalized_scopes(scopes: &[Scope]) -> Result<Vec<Scope>> {
    if scopes.is_empty() || scopes.len() > MAX_RECOMMENDATION_SCOPES {
        return Err(AppError::usage("a recommendation requires 1 to 50 scopes"));
    }
    let mut scopes = scopes.to_vec();
    scopes.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.is_recursive().cmp(&b.is_recursive())));
    if scopes.iter().any(|scope| scope.path.is_empty()) || scopes.windows(2).any(|pair| pair[0].path == pair[1].path) {
        return Err(AppError::usage("recommendation scopes must have unique nonempty paths"));
    }
    Ok(scopes)
}

fn endpoint_records(
    connection: &Connection,
    identity: &Identity,
    repo_root: Option<&str>,
    sent: bool,
    pending_only: bool,
) -> Result<Vec<RecommendationRow>> {
    let endpoint = if sent { "sender" } else { "recipient" };
    let pending = if pending_only { "AND state = 'pending'" } else { "" };
    let mut statement = connection.prepare(&format!(
        "{} WHERE {endpoint}_client = ?1 AND {endpoint}_session_id = ?2
            AND (?3 IS NULL OR repo_root = ?3) {pending} ORDER BY created_at, id",
        select()
    ))?;
    Ok(statement
        .query_map(params![client_name(identity.client), identity.session_id, repo_root], from_row)?
        .collect::<rusqlite::Result<_>>()?)
}

fn recommendation_from(connection: &Connection, identity: &Identity, id: &str) -> Result<Option<RecommendationRow>> {
    Ok(connection
        .query_row(
            &format!(
                "{} WHERE id = ?1 AND ((sender_client = ?2 AND sender_session_id = ?3)
            OR (recipient_client = ?2 AND recipient_session_id = ?3))",
                select()
            ),
            params![id, client_name(identity.client), identity.session_id],
            from_row,
        )
        .optional()?)
}

fn select() -> &'static str {
    "SELECT id, repo_root, sender_snapshot, recipient_snapshot, scopes, action, reason, replacement,
        state, created_at, updated_at, decision, decision_reason, decision_at, invalidation_reason,
        surfaced_at, sender_fingerprint, recipient_fingerprint FROM recommendations"
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<RecommendationRow> {
    Ok(RecommendationRow {
        id: row.get(0)?,
        repo_root: row.get(1)?,
        sender: json_column(row, 2)?,
        recipient: json_column(row, 3)?,
        scopes: json_column(row, 4)?,
        action: parse_enum(row.get(5)?)?,
        reason: row.get(6)?,
        replacement: row.get(7)?,
        state: parse_enum(row.get(8)?)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        decision: row.get::<_, Option<String>>(11)?.map(parse_enum).transpose()?,
        decision_reason: row.get(12)?,
        decision_at: row.get(13)?,
        invalidation_reason: row.get(14)?,
        surfaced_at: row.get(15)?,
        sender_fingerprint: json_column(row, 16)?,
        recipient_fingerprint: json_column(row, 17)?,
    })
}

fn json_column<T: DeserializeOwned>(row: &Row<'_>, index: usize) -> rusqlite::Result<T> {
    let text: String = row.get(index)?;
    serde_json::from_str(&text).map_err(|error| invalid_value(error.to_string()))
}

fn parse_enum<T: DeserializeOwned>(value: String) -> rusqlite::Result<T> {
    T::deserialize(serde_json::Value::String(value)).map_err(|error| invalid_value(error.to_string()))
}

fn enum_name(value: impl Serialize) -> Result<String> {
    match serde_json::to_value(value)? {
        serde_json::Value::String(name) => Ok(name),
        _ => Err(AppError::operational("invalid recommendation enum")),
    }
}

#[cfg(test)]
mod tests;

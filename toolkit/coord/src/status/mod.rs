//! Versioned status JSON and its compact terminal rendering.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    domain::{Client, SessionState, SnapshotScopeKindV1, SnapshotSessionV1, SnapshotV1},
    error::Result,
};

/// Serialize the public status schema.  Keep this separate from dashboard-only
/// fields such as messages, generation, and generated_at.
pub(crate) fn snapshot_json(snapshot: &SnapshotV1) -> Result<String> {
    Ok(serde_json::to_string_pretty(snapshot)?)
}

/// Render the same snapshot as the tab-separated status command output.
pub(crate) fn render_status(snapshot: &SnapshotV1) -> String {
    render_status_at(snapshot, unix_now())
}

fn render_status_at(snapshot: &SnapshotV1, now: f64) -> String {
    let mut lines = vec!["CLIENT\tSTATE\tAGE\tCALLSIGN\tNAME/LABEL\tSESSION\tCWD\tDETAIL".to_owned()];
    let mut named = Vec::new();
    let mut anonymous: Vec<AnonymousGroup<'_>> = Vec::new();

    for session in &snapshot.sessions {
        if is_named(session) {
            named.push(session);
            continue;
        }

        let key = (client_name(session.identity.client), state_name(session.state), session.cwd.as_str());
        if let Some(group) = anonymous.iter_mut().find(|group| group.key == key) {
            group.rows.push(session);
        } else {
            anonymous.push(AnonymousGroup { key, rows: vec![session] });
        }
    }

    for session in named {
        lines.push(session_line(session, now, None));
    }
    for group in anonymous {
        let count = group.rows.len();
        lines.push(session_line(group.rows[0], now, (count > 1).then(|| format!("count={count}"))));
    }

    let coverage = snapshot
        .providers
        .iter()
        .map(|provider| format!("{}={}", client_name(provider.client), coverage_label(provider)))
        .collect::<Vec<_>>()
        .join("; ");
    lines.push(format!("Coverage: {coverage}"));

    if snapshot.outside_scope.sessions > 0 {
        lines.push(format!(
            "Other directories: {} reported sessions across {} working directories.",
            snapshot.outside_scope.sessions, snapshot.outside_scope.directories
        ));
    }

    if !snapshot.notes.is_empty() {
        let machine_wide = snapshot.scope.kind == SnapshotScopeKindV1::Machine;
        let note_scope = if machine_wide { "machine-wide" } else { snapshot.scope.repo_root.as_deref().unwrap_or("") };
        lines.push(format!("Notes ({note_scope}):"));
        for note in &snapshot.notes {
            let prefix = if machine_wide { format!("{}  ", note.repo_root) } else { String::new() };
            lines.push(format!("{prefix}{}  {}  {}", note.id, age_label(note.created_at, now), note.text));
        }
        lines.push("(note --done <id> closes a note)".to_owned());
    }

    let states = snapshot.sessions.iter().map(|session| session.state).collect::<Vec<_>>();
    let partial = !snapshot.complete ||
        snapshot.providers.iter().any(|provider| !provider.enabled || !provider.ok || provider.dropped > 0);
    for (line, present) in [
        (
            "Idle: user prompt; process liveness is reconciled on every coordination read.",
            states.contains(&SessionState::Idle),
        ),
        ("Waiting: host/human wait; claim=queued means coordination queue.", states.contains(&SessionState::Waiting)),
        ("Names/labels: hints; only 'ai-coord start' returning READY grants an edit scope.", true),
        ("Partial coverage: sessions may be missing; absence does not mean no conflicts.", partial),
    ] {
        if present {
            lines.push(line.to_owned());
        }
    }

    lines.join("\n")
}

struct AnonymousGroup<'a> {
    key: (&'static str, &'static str, &'a str),
    rows: Vec<&'a SnapshotSessionV1>,
}

fn is_named(session: &SnapshotSessionV1) -> bool {
    session.callsign.is_some() ||
        session.name.is_some() ||
        session.label.is_some() ||
        session.permission_mode.as_deref() == Some("plan") ||
        session.delegate_count.unwrap_or_default() > 0
}

fn session_line(session: &SnapshotSessionV1, now: f64, session_id: Option<String>) -> String {
    let mut detail = Vec::new();
    if session.permission_mode.as_deref() == Some("plan") {
        detail.push("planning".to_owned());
    }
    if let Some(count) = session.delegate_count.filter(|count| *count > 0) {
        detail.push(format!("delegates={count}"));
    }
    if session.claim_state.is_some_and(|state| state == crate::domain::ClaimState::Queued) {
        detail.push("claim=queued".to_owned());
    }
    if let Some(waiting_for) = &session.waiting_for {
        detail.push(format!("waiting={waiting_for}"));
    }
    if !session.paths.is_empty() {
        detail.push(format!("paths={}", session.paths.join(",")));
    }
    [
        client_name(session.identity.client).to_owned(),
        state_name(session.state).to_owned(),
        age_label(session.last_seen, now),
        session.callsign.clone().unwrap_or_default(),
        session.label.clone().or_else(|| session.name.clone()).unwrap_or_default(),
        session_id.unwrap_or_else(|| session.identity.session_id.clone()),
        session.cwd.clone(),
        detail.join(" "),
    ]
    .join("\t")
}

fn coverage_label(provider: &crate::domain::ProviderReport) -> &'static str {
    if !provider.enabled {
        "disabled"
    } else if provider.ok && provider.dropped == 0 {
        "ok"
    } else {
        "partial"
    }
}

fn client_name(client: Client) -> &'static str {
    match client {
        Client::Codex => "codex",
        Client::Claude => "claude",
    }
}

fn state_name(state: SessionState) -> &'static str {
    match state {
        SessionState::Idle => "idle",
        SessionState::InFlight => "in_flight",
        SessionState::Waiting => "waiting",
        SessionState::Working => "working",
        SessionState::Unknown => "unknown",
    }
}

fn age_label(timestamp: f64, now: f64) -> String {
    let seconds = (now - timestamp).max(0.0) as u64;
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3_600 {
        format!("{}m", seconds / 60)
    } else if seconds < 86_400 {
        format!("{}h", seconds / 3_600)
    } else {
        format!("{}d", seconds / 86_400)
    }
}

fn unix_now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Identity, OutsideScopeV1, ProviderReport, SnapshotScopeV1};

    fn snapshot(sessions: Vec<SnapshotSessionV1>) -> SnapshotV1 {
        SnapshotV1 {
            schema_version: 1,
            complete: true,
            scope: SnapshotScopeV1 { kind: SnapshotScopeKindV1::Repo, repo_root: Some("/repo".into()) },
            self_identity: Some(Identity { client: Client::Codex, session_id: "self".into() }),
            providers: vec![ProviderReport {
                client: Client::Codex,
                enabled: true,
                ok: true,
                source: "test".into(),
                dropped: 0,
                error: None,
            }],
            sessions,
            claims: vec![],
            notes: vec![],
            delegates: vec![],
            outside_scope: OutsideScopeV1 { sessions: 0, directories: 0 },
        }
    }

    fn session(id: &str) -> SnapshotSessionV1 {
        SnapshotSessionV1 {
            identity: Identity { client: Client::Codex, session_id: id.into() },
            cwd: "/repo".into(),
            repo_root: Some("/repo".into()),
            state: SessionState::Working,
            callsign: None,
            name: None,
            label: None,
            waiting_for: None,
            permission_mode: None,
            delegate_count: None,
            pid: None,
            source: "test".into(),
            started_at: 1.0,
            last_seen: 1_000.0,
            claim_state: None,
            paths: vec![],
        }
    }

    #[test]
    fn json_keeps_the_v1_schema() {
        let payload: serde_json::Value = serde_json::from_str(&snapshot_json(&snapshot(vec![])).unwrap()).unwrap();
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["self"]["session_id"], "self");
        assert!(payload.get("messages").is_none());
    }

    #[test]
    fn rendering_groups_anonymous_but_keeps_named_rows() {
        let mut named = session("named");
        named.callsign = Some("🦊 Fox".into());
        named.label = Some("exact files".into());
        let rendered = render_status_at(&snapshot(vec![session("one"), session("two"), named]), 2_000.0);
        assert!(rendered.contains("\t🦊 Fox\texact files\tnamed\t/repo\t"));
        assert!(rendered.contains("\tcount=2\t/repo\t"));
        assert!(!rendered.contains("\tone\t"));
        assert!(!rendered.contains("\ttwo\t"));
    }
}

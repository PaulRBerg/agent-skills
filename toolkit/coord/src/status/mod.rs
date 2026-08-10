//! Versioned status JSON and its compact terminal rendering.

use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    domain::{
        Client, Identity, SessionState, SnapshotScopeKindV2, SnapshotSessionV2, SnapshotV2, SnapshotWorkV2, WorkState,
    },
    error::Result,
};

/// Serialize the public status schema. Keep this separate from dashboard-only
/// fields such as messages, generation, and generated_at.
pub(crate) fn snapshot_json(snapshot: &SnapshotV2) -> Result<String> {
    Ok(serde_json::to_string_pretty(snapshot)?)
}

/// Render the same snapshot as the tab-separated status command output.
pub(crate) fn render_status(snapshot: &SnapshotV2) -> String {
    render_status_at(snapshot, unix_now())
}

fn render_status_at(snapshot: &SnapshotV2, now: f64) -> String {
    let mut lines = vec!["CLIENT\tSTATE\tAGE\tCALLSIGN\tNAME/LABEL\tSESSION\tCWD\tDETAIL".to_owned()];
    let work_by_identity =
        snapshot.work.iter().map(|work| (work.identity.clone(), work)).collect::<HashMap<Identity, &SnapshotWorkV2>>();
    let mut named = Vec::new();
    let mut anonymous: Vec<AnonymousGroup<'_>> = Vec::new();

    for session in &snapshot.sessions {
        let work = work_by_identity.get(&session.identity).copied();
        if is_named(session, work) {
            named.push((session, work));
            continue;
        }

        let key = (client_name(session.identity.client), state_name(session.state), session.cwd.as_str());
        if let Some(group) = anonymous.iter_mut().find(|group| group.key == key) {
            group.rows.push(session);
        } else {
            anonymous.push(AnonymousGroup { key, rows: vec![session] });
        }
    }

    for (session, work) in named {
        lines.push(session_line(session, work, now, None));
    }
    for group in anonymous {
        let count = group.rows.len();
        lines.push(session_line(group.rows[0], None, now, (count > 1).then(|| format!("count={count}"))));
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

    let finding_counts = snapshot.findings.iter().fold((0, 0, 0), |counts, finding| {
        (
            counts.0 + usize::from(finding.state == crate::domain::FindingState::Pending),
            counts.1 + usize::from(finding.triaging),
            counts.2 + usize::from(finding.state == crate::domain::FindingState::HandedOff),
        )
    });
    if finding_counts != (0, 0, 0) {
        let machine_wide = snapshot.scope.kind == SnapshotScopeKindV2::Machine;
        let scope = if machine_wide { "machine-wide" } else { snapshot.scope.repo_root.as_deref().unwrap_or("") };
        lines.push(format!(
            "Findings ({scope}): pending={}; triaging={}; handed-off={}. `ai-coord finding list` shows details.",
            finding_counts.0, finding_counts.1, finding_counts.2
        ));
    }
    for handoff in &snapshot.handoffs {
        lines.push(format!("Task handoffs ({}): {}.", handoff.repo_root, handoff.count));
    }

    let states = snapshot.sessions.iter().map(|session| session.state).collect::<Vec<_>>();
    let partial = !snapshot.complete ||
        snapshot.providers.iter().any(|provider| !provider.enabled || !provider.ok || provider.dropped > 0);
    for (line, present) in [
        (
            "Idle: user prompt; fresh status and authorization reads reconcile process liveness.",
            states.contains(&SessionState::Idle),
        ),
        ("Waiting: host/human wait; work=queued means coordination queue.", states.contains(&SessionState::Waiting)),
        (
            "Drafts: non-authoritative temporary memory; submit with 'ai-coord start --draft'.",
            snapshot.work.iter().any(|work| work.state == WorkState::Draft),
        ),
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
    rows: Vec<&'a SnapshotSessionV2>,
}

fn is_named(session: &SnapshotSessionV2, work: Option<&SnapshotWorkV2>) -> bool {
    session.callsign.is_some() ||
        session.name.is_some() ||
        work.is_some() ||
        session.permission_mode.as_deref() == Some("plan") ||
        session.delegate_count.unwrap_or_default() > 0
}

fn session_line(
    session: &SnapshotSessionV2,
    work: Option<&SnapshotWorkV2>,
    now: f64,
    session_id: Option<String>,
) -> String {
    let mut detail = Vec::new();
    if session.permission_mode.as_deref() == Some("plan") {
        detail.push("planning".to_owned());
    }
    if let Some(count) = session.delegate_count.filter(|count| *count > 0) {
        detail.push(format!("delegates={count}"));
    }
    if let Some(work) = work {
        match work.state {
            WorkState::Draft => detail.push(format!("draft · {} scopes", work.scope_count.unwrap_or_default())),
            WorkState::Queued => detail.push("work=queued".to_owned()),
            WorkState::Active => detail.push("work=active".to_owned()),
        }
        if let Some(scopes) = &work.scopes {
            detail.push(format!(
                "paths={}",
                scopes.iter().map(|scope| scope.path.as_str()).collect::<Vec<_>>().join(",")
            ));
        }
    }
    if let Some(waiting_for) = &session.waiting_for {
        detail.push(format!("waiting={waiting_for}"));
    }
    [
        client_name(session.identity.client).to_owned(),
        state_name(session.state).to_owned(),
        age_label(session.last_seen, now),
        session.callsign.clone().unwrap_or_default(),
        work.map(|work| work.label.clone()).or_else(|| session.name.clone()).unwrap_or_default(),
        session_id.unwrap_or_else(|| session.identity.session_id.clone()),
        session.cwd.clone(),
        detail.join(" "),
    ]
    .map(|value| terminal_field(&value))
    .join("\t")
}

fn terminal_field(value: &str) -> String {
    value.chars().map(|character| if character.is_control() { ' ' } else { character }).collect()
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
    use crate::domain::{
        FindingState, FindingSummary, OutsideScopeV2, ProviderReport, Scope, ScopeKind, SnapshotScopeV2, SnapshotWorkV2,
    };

    fn snapshot(sessions: Vec<SnapshotSessionV2>, work: Vec<SnapshotWorkV2>) -> SnapshotV2 {
        SnapshotV2 {
            schema_version: 4,
            complete: true,
            scope: SnapshotScopeV2 { kind: SnapshotScopeKindV2::Repo, repo_root: Some("/repo".into()) },
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
            work,
            findings: vec![],
            handoffs: vec![],
            delegates: vec![],
            outside_scope: OutsideScopeV2 { sessions: 0, directories: 0 },
        }
    }

    fn session(id: &str) -> SnapshotSessionV2 {
        SnapshotSessionV2 {
            identity: Identity { client: Client::Codex, session_id: id.into() },
            cwd: "/repo".into(),
            repo_root: Some("/repo".into()),
            state: SessionState::Working,
            callsign: None,
            name: None,
            waiting_for: None,
            permission_mode: None,
            delegate_count: None,
            pid: None,
            source: "test".into(),
            started_at: 1.0,
            last_seen: 1_000.0,
        }
    }

    fn work(id: &str, state: WorkState) -> SnapshotWorkV2 {
        SnapshotWorkV2 {
            id: 1,
            identity: Identity { client: Client::Codex, session_id: id.into() },
            repo_root: "/repo".into(),
            label: "exact files".into(),
            state,
            blocked_reason: None,
            scope_count: (state == WorkState::Draft).then_some(1),
            scopes: (state != WorkState::Draft)
                .then_some(vec![Scope { path: "src/lib.rs".into(), kind: ScopeKind::Exact }]),
            draft_created_at: (state == WorkState::Draft).then_some(900.0),
            submitted_at: (state != WorkState::Draft).then_some(950.0),
            updated_at: 1_000.0,
        }
    }

    fn finding(id: &str, summary: &str, state: FindingState, triaging: bool) -> FindingSummary {
        FindingSummary {
            id: id.into(),
            repo_root: "/repo".into(),
            summary: summary.into(),
            kind: None,
            state,
            paths: vec!["private/path.rs".into()],
            created_at: 1.0,
            updated_at: 1_000.0,
            terminal_at: state.is_terminal().then_some(1_000.0),
            handoff_path: (state == FindingState::HandedOff).then(|| "private/path.rs".into()),
            commit_oid: None,
            canonical_id: None,
            sighting_count: 1,
            triaging,
        }
    }

    #[test]
    fn json_keeps_the_v4_schema_and_omits_draft_paths() {
        let payload: serde_json::Value = serde_json::from_str(
            &snapshot_json(&snapshot(vec![session("self")], vec![work("self", WorkState::Draft)])).unwrap(),
        )
        .unwrap();
        assert_eq!(payload["schema_version"], 4);
        assert_eq!(payload["self"]["session_id"], "self");
        assert_eq!(payload["work"][0]["scope_count"], 1);
        assert!(payload["work"][0].get("scopes").is_none());
        assert!(payload["work"][0].get("blocked_reason").is_none());
        assert!(payload.get("messages").is_none());
    }

    #[test]
    fn rendering_reports_task_handoff_counts() {
        let mut value = snapshot(vec![], vec![]);
        value.handoffs.push(crate::domain::SnapshotHandoffV4 { repo_root: "/repo".into(), count: 2 });
        assert!(render_status_at(&value, 100.0).contains("Task handoffs (/repo): 2."));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&snapshot_json(&value).unwrap()).unwrap()["handoffs"][0]["count"],
            2
        );
    }

    #[test]
    fn rendering_groups_anonymous_but_keeps_work_rows_and_hides_draft_paths() {
        let mut named = session("named");
        named.callsign = Some("🦊 Fox\u{1b}[2J".into());
        let rendered = render_status_at(
            &snapshot(vec![session("one"), session("two"), named], vec![work("named", WorkState::Draft)]),
            2_000.0,
        );
        assert!(rendered.contains("\t🦊 Fox [2J\texact files\tnamed\t/repo\tdraft · 1 scopes"));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\tcount=2\t/repo\t"));
        assert!(!rendered.contains("src/lib.rs"));
    }

    #[test]
    fn rendering_summarizes_finding_counts_without_dumping_the_backlog() {
        let mut snapshot = snapshot(vec![session("self")], vec![]);
        snapshot.findings = vec![
            finding("pending-id", "private pending summary", FindingState::Pending, true),
            finding("handoff-id", "private handoff summary", FindingState::HandedOff, false),
            finding("fixed-id", "private fixed summary", FindingState::Fixed, false),
        ];
        let rendered = render_status_at(&snapshot, 2_000.0);
        assert!(rendered.contains("Findings (/repo): pending=1; triaging=1; handed-off=1."));
        for private in ["pending-id", "private pending summary", "private/path.rs", "fixed-id"] {
            assert!(!rendered.contains(private));
        }
    }
}

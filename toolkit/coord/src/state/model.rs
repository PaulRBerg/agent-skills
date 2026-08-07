use crate::domain::{Client, FindingKind, Identity, ProcessFingerprint, Scope, SessionState, WorkState};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SessionRow {
    pub(crate) identity: Identity,
    pub(crate) cwd: String,
    pub(crate) repo_root: Option<String>,
    pub(crate) state: SessionState,
    pub(crate) callsign: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) waiting_for: Option<String>,
    pub(crate) permission_mode: Option<String>,
    pub(crate) fingerprint: Option<ProcessFingerprint>,
    pub(crate) source: String,
    pub(crate) started_at: f64,
    pub(crate) last_seen: f64,
    pub(crate) revision: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct SessionUpdate {
    pub(crate) identity: Identity,
    pub(crate) cwd: String,
    pub(crate) repo_root: Option<String>,
    pub(crate) state: SessionState,
    pub(crate) source: String,
    pub(crate) name: Option<String>,
    pub(crate) waiting_for: Option<String>,
    pub(crate) permission_mode: Option<String>,
    pub(crate) update_permission_mode: bool,
    pub(crate) fingerprint: Option<ProcessFingerprint>,
    pub(crate) started_at: Option<f64>,
    pub(crate) current: f64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EndedObservation {
    pub(crate) identity: Identity,
    pub(crate) expected_fingerprint: Option<ProcessFingerprint>,
    pub(crate) expected_revision: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WorkRow {
    pub(crate) id: i64,
    pub(crate) identity: Identity,
    pub(crate) repo_root: String,
    pub(crate) label: String,
    pub(crate) state: WorkState,
    pub(crate) blocked_reason: Option<String>,
    pub(crate) scopes: Vec<Scope>,
    pub(crate) draft_created_at: Option<f64>,
    pub(crate) submitted_at: Option<f64>,
    pub(crate) updated_at: f64,
    pub(crate) revision: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkUpdate {
    pub(crate) identity: Identity,
    pub(crate) repo_root: String,
    pub(crate) label: String,
    pub(crate) state: WorkState,
    pub(crate) blocked_reason: Option<String>,
    pub(crate) scopes: Vec<Scope>,
    /// `None` preserves existing baselines; `Some` replaces all of them.
    pub(crate) baselines: Option<Vec<BaselineRow>>,
    pub(crate) residual_paths: Vec<String>,
    pub(crate) draft_created_at: Option<f64>,
    pub(crate) submitted_at: Option<f64>,
    pub(crate) updated_at: f64,
    /// Compare-and-swap guard for an existing work item.
    pub(crate) expected_revision: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BaselineRow {
    pub(crate) path: String,
    pub(crate) oid: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DirtObservationRow {
    pub(crate) repo_root: String,
    pub(crate) path: String,
    pub(crate) blob_hash: String,
    pub(crate) first_seen: f64,
    pub(crate) last_seen: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ResidualOwnerRow {
    pub(crate) repo_root: String,
    pub(crate) path: String,
    pub(crate) identity: Identity,
    pub(crate) released_at: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MessageRow {
    pub(crate) id: String,
    pub(crate) sender: Identity,
    pub(crate) sender_callsign: Option<String>,
    pub(crate) recipient: Identity,
    pub(crate) recipient_callsign: Option<String>,
    pub(crate) repo_root: Option<String>,
    pub(crate) text: String,
    pub(crate) created_at: f64,
    pub(crate) acknowledged_at: Option<f64>,
    pub(crate) notified_at: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FindingAdd {
    pub(crate) repo_root: String,
    pub(crate) summary: String,
    pub(crate) normalized_summary: String,
    pub(crate) kind: Option<FindingKind>,
    pub(crate) paths: Vec<String>,
    pub(crate) head_oid: Option<String>,
    pub(crate) observations: Vec<FindingPathObservation>,
    pub(crate) author: Identity,
    pub(crate) turn_id: Option<String>,
    pub(crate) current: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FindingPathObservation {
    pub(crate) path: String,
    pub(crate) content_sha256: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FindingAddResult {
    pub(crate) finding: crate::domain::FindingSummary,
    pub(crate) deduplicated: bool,
    pub(crate) candidates: Vec<crate::domain::FindingSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CurrentTurnFinding {
    pub(crate) id: String,
    pub(crate) summary: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct FindingCounts {
    pub(crate) pending: usize,
    pub(crate) triaging: usize,
    pub(crate) handed_off: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FindingResolution {
    pub(crate) state: crate::domain::FindingState,
    pub(crate) commit_oid: Option<String>,
    pub(crate) canonical_id: Option<String>,
    pub(crate) actor: Identity,
    pub(crate) current: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DelegateRow {
    pub(crate) parent: Identity,
    pub(crate) agent_id: String,
    pub(crate) agent_type: Option<String>,
    pub(crate) state: String,
    pub(crate) last_seen: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HookHealthRow {
    pub(crate) client: Client,
    pub(crate) event: String,
    pub(crate) last_error_code: Option<String>,
    pub(crate) last_error_at: Option<f64>,
    pub(crate) last_success_at: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProviderCacheRow {
    pub(crate) context_key: String,
    pub(crate) client: Client,
    pub(crate) refreshed_at: f64,
    pub(crate) ok: bool,
    pub(crate) source: String,
    pub(crate) enabled: bool,
    pub(crate) dropped: usize,
}

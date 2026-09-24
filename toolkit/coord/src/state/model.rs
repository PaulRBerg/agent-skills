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
    pub(crate) coordination_waived: bool,
    pub(crate) fingerprint: Option<ProcessFingerprint>,
    pub(crate) transcript_path: Option<String>,
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
    /// `None` preserves the current prompt-scoped coordination waiver.
    pub(crate) coordination_waived: Option<bool>,
    pub(crate) fingerprint: Option<ProcessFingerprint>,
    /// `None` preserves a previously observed opaque transcript identity.
    pub(crate) transcript_path: Option<String>,
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
    pub(crate) label: String,
    pub(crate) state: WorkState,
    pub(crate) blocked_reason: Option<String>,
    pub(crate) claims: Vec<WorkClaimRow>,
    pub(crate) submitted_at: Option<f64>,
    pub(crate) updated_at: f64,
    pub(crate) revision: i64,
}

impl WorkRow {
    pub(crate) fn claim(&self, repo_root: &str) -> Option<&WorkClaimRow> {
        self.claims.iter().find(|claim| claim.repo_root == repo_root)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WorkClaimRow {
    pub(crate) id: i64,
    pub(crate) repo_root: String,
    pub(crate) blocked_reason: Option<String>,
    pub(crate) scopes: Vec<Scope>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WorkUpdate {
    pub(crate) identity: Identity,
    pub(crate) label: String,
    pub(crate) state: WorkState,
    pub(crate) blocked_reason: Option<String>,
    pub(crate) claims: Vec<WorkClaimUpdate>,
    pub(crate) submitted_at: Option<f64>,
    pub(crate) updated_at: f64,
    /// Compare-and-swap guard for an existing work item.
    pub(crate) expected_revision: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WorkClaimUpdate {
    pub(crate) repo_root: String,
    pub(crate) blocked_reason: Option<String>,
    pub(crate) scopes: Vec<Scope>,
    /// `None` preserves retained claim baselines; `Some` replaces all of them.
    pub(crate) baselines: Option<Vec<BaselineRow>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DraftOwner {
    Session(Identity),
    Name(String),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DraftClaimUpdate {
    pub(crate) repo_root: String,
    pub(crate) scopes: Vec<Scope>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DraftClaimRow {
    pub(crate) repo_root: String,
    pub(crate) scopes: Vec<Scope>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DraftRow {
    pub(crate) id: i64,
    pub(crate) name: Option<String>,
    pub(crate) owner: Option<Identity>,
    pub(crate) label: String,
    pub(crate) created_at: f64,
    pub(crate) updated_at: f64,
    pub(crate) claims: Vec<DraftClaimRow>,
}

impl DraftRow {
    pub(crate) fn claim(&self, repo_root: &str) -> Option<&DraftClaimRow> {
        self.claims.iter().find(|claim| claim.repo_root == repo_root)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BaselineRow {
    pub(crate) path: String,
    pub(crate) oid: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TouchedPaths {
    pub(crate) paths: Vec<String>,
    pub(crate) truncated: bool,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RecommendationAction {
    Defer,
    Omit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RecommendationState {
    Pending,
    Accepted,
    Rejected,
    Withdrawn,
    Stale,
}

impl RecommendationState {
    pub(crate) const fn is_live(self) -> bool {
        matches!(self, Self::Pending | Self::Accepted)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RecommendationDecision {
    Accepted,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub(crate) struct RecommendationClaim {
    pub(crate) repo_root: String,
    pub(crate) scopes: Vec<Scope>,
}

/// Semantic work context, excluding revision, queue state, timestamps and dirt.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub(crate) struct RecommendationWork {
    pub(crate) id: i64,
    pub(crate) label: String,
    pub(crate) claims: Vec<RecommendationClaim>,
}

impl From<&WorkRow> for RecommendationWork {
    fn from(work: &WorkRow) -> Self {
        let mut claims = work
            .claims
            .iter()
            .map(|claim| {
                let mut scopes = claim.scopes.clone();
                scopes.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.is_recursive().cmp(&b.is_recursive())));
                RecommendationClaim { repo_root: claim.repo_root.clone(), scopes }
            })
            .collect::<Vec<_>>();
        claims.sort_by(|a, b| a.repo_root.cmp(&b.repo_root));
        Self { id: work.id, label: work.label.split_whitespace().collect::<Vec<_>>().join(" "), claims }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub(crate) struct RecommendationEndpoint {
    pub(crate) identity: Identity,
    pub(crate) callsign: Option<String>,
    pub(crate) work: RecommendationWork,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(crate) struct RecommendationRow {
    pub(crate) id: String,
    pub(crate) repo_root: String,
    pub(crate) sender: RecommendationEndpoint,
    pub(crate) recipient: RecommendationEndpoint,
    pub(crate) scopes: Vec<Scope>,
    pub(crate) action: RecommendationAction,
    pub(crate) reason: String,
    pub(crate) replacement: String,
    pub(crate) state: RecommendationState,
    pub(crate) created_at: f64,
    pub(crate) updated_at: f64,
    pub(crate) decision: Option<RecommendationDecision>,
    pub(crate) decision_reason: Option<String>,
    pub(crate) decision_at: Option<f64>,
    pub(crate) invalidation_reason: Option<String>,
    #[serde(skip)]
    pub(crate) surfaced_at: Option<f64>,
    #[serde(skip)]
    pub(super) sender_fingerprint: Option<ProcessFingerprint>,
    #[serde(skip)]
    pub(super) recipient_fingerprint: Option<ProcessFingerprint>,
}

/// A slow process probe is performed before acquiring the transaction. The store
/// revalidates its fingerprint and semantic work context under the write lock.
#[derive(Clone, Debug)]
pub(crate) struct RecommendationObservation {
    pub(crate) identity: Identity,
    pub(crate) fingerprint: Option<ProcessFingerprint>,
    pub(crate) work: RecommendationWork,
    pub(crate) liveness: crate::domain::ProcessLiveness,
}

impl RecommendationObservation {
    pub(crate) fn new(session: &SessionRow, work: &WorkRow, liveness: crate::domain::ProcessLiveness) -> Self {
        Self {
            identity: session.identity.clone(),
            fingerprint: session.fingerprint.clone(),
            work: work.into(),
            liveness,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RecommendationSend {
    pub(crate) sender: RecommendationObservation,
    pub(crate) recipient: RecommendationObservation,
    pub(crate) repo_root: String,
    pub(crate) scopes: Vec<Scope>,
    pub(crate) action: RecommendationAction,
    pub(crate) reason: String,
    pub(crate) replacement: String,
    pub(crate) current: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RecommendationOutcome {
    Recommended,
    Existing,
    Accepted,
    Rejected,
    Withdrawn,
    Stale,
    Conflict,
}

#[derive(Clone, Debug)]
pub(crate) struct RecommendationMutation {
    pub(crate) outcome: RecommendationOutcome,
    pub(crate) recommendation: RecommendationRow,
}

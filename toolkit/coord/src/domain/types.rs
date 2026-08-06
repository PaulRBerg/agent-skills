use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Client {
    Codex,
    Claude,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub(crate) struct Identity {
    pub(crate) client: Client,
    pub(crate) session_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ProcessFingerprint {
    pub(crate) pid: u32,
    pub(crate) start_token: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProcessLiveness {
    Alive,
    Dead,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SessionState {
    Idle,
    InFlight,
    Waiting,
    Working,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ClaimState {
    Active,
    Queued,
    Intent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Scope {
    pub(crate) path: String,
    pub(crate) recursive: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OutcomeKind {
    Active,
    Blocked,
    Done,
    Intent,
    Message,
    Note,
    Ready,
    Released,
    Timeout,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Outcome {
    pub(crate) kind: OutcomeKind,
    pub(crate) code: u8,
    pub(crate) detail: String,
    pub(crate) paths: Vec<String>,
    pub(crate) holders: Vec<String>,
    pub(crate) broad_paths: Vec<String>,
}

impl Outcome {
    pub(crate) fn new(kind: OutcomeKind, code: u8, detail: impl Into<String>) -> Self {
        Self { kind, code, detail: detail.into(), paths: Vec::new(), holders: Vec::new(), broad_paths: Vec::new() }
    }

    pub(crate) fn with_paths(mut self, paths: Vec<String>) -> Self {
        self.paths = paths;
        self
    }

    pub(crate) fn line(&self) -> String {
        let mut fields = vec![self.kind.name().to_owned()];
        if !self.detail.is_empty() {
            fields.push(self.detail.clone());
        }
        fields.extend(self.paths.iter().cloned());
        fields.join("\t")
    }
}

impl OutcomeKind {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Blocked => "BLOCKED",
            Self::Done => "DONE",
            Self::Intent => "INTENT",
            Self::Message => "MESSAGE",
            Self::Note => "NOTE",
            Self::Ready => "READY",
            Self::Released => "RELEASED",
            Self::Timeout => "TIMEOUT",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ProviderReport {
    pub(crate) client: Client,
    pub(crate) ok: bool,
    pub(crate) source: String,
    pub(crate) enabled: bool,
    pub(crate) dropped: usize,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InventoryResult {
    pub(crate) complete: bool,
    pub(crate) providers: Vec<ProviderReport>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SnapshotScopeKindV1 {
    Cwd,
    Machine,
    Repo,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct SnapshotScopeV1 {
    pub(crate) kind: SnapshotScopeKindV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repo_root: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct SnapshotSessionV1 {
    #[serde(flatten)]
    pub(crate) identity: Identity,
    pub(crate) cwd: String,
    pub(crate) repo_root: Option<String>,
    pub(crate) state: SessionState,
    pub(crate) callsign: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) waiting_for: Option<String>,
    pub(crate) permission_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) delegate_count: Option<usize>,
    pub(crate) pid: Option<u32>,
    pub(crate) source: String,
    pub(crate) started_at: f64,
    pub(crate) last_seen: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) claim_state: Option<ClaimState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) paths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct SnapshotClaimV1 {
    pub(crate) id: i64,
    #[serde(flatten)]
    pub(crate) identity: Identity,
    pub(crate) repo_root: String,
    pub(crate) label: String,
    pub(crate) state: ClaimState,
    pub(crate) blocked_reason: Option<String>,
    pub(crate) paths: Vec<String>,
    pub(crate) created_at: f64,
    pub(crate) updated_at: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct SnapshotNoteV1 {
    pub(crate) id: String,
    pub(crate) repo_root: String,
    pub(crate) author_client: Option<Client>,
    pub(crate) author_session_id: Option<String>,
    pub(crate) text: String,
    pub(crate) created_at: f64,
    pub(crate) resolved_at: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct SnapshotDelegateV1 {
    pub(crate) parent_client: Client,
    pub(crate) parent_session_id: String,
    pub(crate) agent_id: String,
    pub(crate) agent_type: Option<String>,
    pub(crate) state: String,
    pub(crate) last_seen: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct OutsideScopeV1 {
    pub(crate) sessions: usize,
    pub(crate) directories: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct SnapshotV1 {
    pub(crate) schema_version: u32,
    pub(crate) complete: bool,
    pub(crate) scope: SnapshotScopeV1,
    #[serde(rename = "self")]
    pub(crate) self_identity: Option<Identity>,
    pub(crate) providers: Vec<ProviderReport>,
    pub(crate) sessions: Vec<SnapshotSessionV1>,
    pub(crate) claims: Vec<SnapshotClaimV1>,
    pub(crate) notes: Vec<SnapshotNoteV1>,
    pub(crate) delegates: Vec<SnapshotDelegateV1>,
    pub(crate) outside_scope: OutsideScopeV1,
}

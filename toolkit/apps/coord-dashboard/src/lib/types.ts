export type WorkState = "active" | "queued";
export type WorkScopeKind = "exact" | "recursive";
export type Client = "claude" | "codex";
export type SessionState = "idle" | "in_flight" | "unknown" | "waiting" | "working";
export type SnapshotScope =
  | { kind: "machine"; repo_root?: never }
  | { kind: "cwd" | "repo"; repo_root: string };

export type WorkScope = {
  path: string;
  kind: WorkScopeKind;
};

export type ProviderCoverage = {
  client: Client;
  ok: boolean;
  source: string;
  enabled: boolean;
  dropped: number;
  error: string | null;
};

export type SessionIdentity = {
  client: Client;
  session_id: string;
};

export type Session = SessionIdentity & {
  cwd: string;
  repo_root: string | null;
  state: SessionState;
  callsign?: string | null;
  name: string | null;
  waiting_for: string | null;
  permission_mode?: string | null;
  coordination_waived: boolean;
  delegate_count?: number;
  pid: number | null;
  source: string;
  started_at: number;
  last_seen: number;
};

export type Work = SessionIdentity & {
  id: number;
  label: string;
  state: WorkState;
  blocked_reason?: string | null;
  scope_count: number;
  submitted_at?: number;
  updated_at: number;
  claims: WorkClaim[];
};

export type WorkClaim = {
  repo_root: string;
  blocked_reason?: string | null;
  scope_count: number;
  scopes?: WorkScope[];
};

export type SnapshotDraftClaim = {
  repo_root: string;
  scope_count: number;
};

export type SnapshotDraft = {
  id: string;
  name: string | null;
  owner: SessionIdentity | null;
  label: string;
  created_at: number;
  updated_at: number;
  claims: SnapshotDraftClaim[];
};

export type FindingState = "pending" | "handed-off" | "fixed" | "stale" | "rejected" | "duplicate";

export type FindingKind = "bug" | "docs" | "improvement";

export type Finding = {
  id: string;
  repo_root: string;
  summary: string;
  kind: FindingKind | null;
  state: FindingState;
  paths: string[];
  created_at: number;
  updated_at: number;
  terminal_at: number | null;
  handoff_path: string | null;
  commit_oid: string | null;
  canonical_id: string | null;
  sighting_count: number;
  triaging: boolean;
};

export type Delegate = {
  parent_client: Client;
  parent_session_id: string;
  agent_id: string;
  agent_type: string | null;
  state: string;
  last_seen: number;
};

export type Message = {
  id: string;
  sender_client: Client;
  sender_session_id: string;
  sender_callsign?: string | null;
  recipient_client: Client;
  recipient_session_id: string;
  recipient_callsign?: string | null;
  repo_root: string | null;
  text: string;
  created_at: number;
  acknowledged_at: number | null;
};

export type Snapshot = {
  schema_version: 8;
  complete: boolean;
  scope: SnapshotScope;
  self: SessionIdentity | null;
  providers: ProviderCoverage[];
  sessions: Session[];
  work: Work[];
  drafts: SnapshotDraft[];
  findings: Finding[];
  handoffs: { repo_root: string; count: number }[];
  delegates: Delegate[];
  outside_scope: {
    sessions: number;
    directories: number;
  };
  messages: Message[];
  generated_at: string;
  generation: number;
};

export type WorkWithQueuePosition = Omit<Work, "claims"> & {
  claims: WorkClaimWithQueuePosition[];
};

export type WorkClaimWithQueuePosition = WorkClaim & {
  queuePosition?: number;
};

export type LaneSession = {
  session: Session;
  work?: WorkWithQueuePosition;
  delegates: Delegate[];
};

export type RepoLaneDraft = {
  draft: SnapshotDraft;
  who: string;
  scopeCount: number;
};

export type RepoLaneModel = {
  repoRoot: string;
  sessions: LaneSession[];
  unmatchedWork: WorkWithQueuePosition[];
  drafts: RepoLaneDraft[];
  lastActivity: number | null;
  handoffCount: number;
};

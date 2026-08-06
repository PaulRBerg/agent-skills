export type WorkState = "active" | "draft" | "queued";
export type WorkScopeKind = "exact" | "recursive";

export interface WorkScope {
  path: string;
  kind: WorkScopeKind;
}

export interface ProviderCoverage {
  client: string;
  ok: boolean;
  source: string;
  enabled: boolean;
  dropped: number;
  error: string | null;
}

export interface SessionIdentity {
  client: string;
  session_id: string;
}

export interface Session extends SessionIdentity {
  cwd: string;
  repo_root: string | null;
  state: string;
  callsign?: string | null;
  name: string | null;
  waiting_for: string | null;
  permission_mode?: string | null;
  delegate_count?: number;
  pid: number | null;
  source: string;
  started_at: number;
  last_seen: number;
}

export interface Work extends SessionIdentity {
  id: number;
  repo_root: string;
  label: string;
  state: WorkState;
  blocked_reason?: string | null;
  scope_count?: number;
  scopes?: WorkScope[];
  draft_created_at?: number;
  submitted_at?: number;
  updated_at: number;
}

export interface Note {
  id: string;
  repo_root: string;
  author_client: string | null;
  author_session_id: string | null;
  text: string;
  created_at: number;
  resolved_at: number | null;
}

export interface Delegate {
  parent_client: string;
  parent_session_id: string;
  agent_id: string;
  agent_type: string | null;
  state: string;
  last_seen: number;
}

export interface Message {
  id: string;
  sender_client: string;
  sender_session_id: string;
  sender_callsign?: string | null;
  recipient_client: string;
  recipient_session_id: string;
  recipient_callsign?: string | null;
  repo_root: string | null;
  text: string;
  created_at: number;
  acknowledged_at: number | null;
}

export interface Snapshot {
  schema_version: number;
  complete: boolean;
  scope: {
    kind: string;
    repo_root?: string;
  };
  self: SessionIdentity | null;
  providers: ProviderCoverage[];
  sessions: Session[];
  work: Work[];
  notes: Note[];
  delegates: Delegate[];
  outside_scope: {
    sessions: number;
    directories: number;
  };
  messages: Message[];
  generated_at: string;
  generation: number;
}

export interface WorkWithQueuePosition extends Work {
  queuePosition?: number;
}

export interface LaneSession {
  session: Session;
  work?: WorkWithQueuePosition;
  delegates: Delegate[];
}

export interface RepoLaneModel {
  repoRoot: string;
  sessions: LaneSession[];
  unmatchedWork: WorkWithQueuePosition[];
  lastActivity: number;
}

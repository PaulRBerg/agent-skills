import type {
  Client,
  FindingKind,
  FindingState,
  SessionState,
  Snapshot,
  WorkScopeKind,
  WorkState,
} from "@/lib/types";

export type ConnectionState =
  "connecting" | "live" | "polling" | "disconnected";

interface SnapshotCallbacks {
  onSnapshot: (snapshot: Snapshot) => void;
  onConnectionChange: (state: ConnectionState) => void;
  onError: (error: Error) => void;
}

function record(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${path} must be an object`);
  }
  return value as Record<string, unknown>;
}

function array(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) throw new Error(`${path} must be an array`);
  return value;
}

function string(value: unknown, path: string): string {
  if (typeof value !== "string") throw new Error(`${path} must be a string`);
  return value;
}

function nullableString(value: unknown, path: string): string | null {
  if (value === null) return null;
  return string(value, path);
}

function number(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error(`${path} must be a finite number`);
  }
  return value;
}

function integer(value: unknown, path: string): number {
  const parsed = number(value, path);
  if (!Number.isInteger(parsed)) throw new Error(`${path} must be an integer`);
  return parsed;
}

function unsignedInteger(value: unknown, path: string): number {
  const parsed = integer(value, path);
  if (parsed < 0) throw new Error(`${path} must be non-negative`);
  return parsed;
}

function boolean(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") throw new Error(`${path} must be a boolean`);
  return value;
}

function client(value: unknown, path: string): Client {
  if (value !== "claude" && value !== "codex") {
    throw new Error(`${path} must be claude or codex`);
  }
  return value;
}

function sessionState(value: unknown, path: string): SessionState {
  if (
    value !== "idle" &&
    value !== "in_flight" &&
    value !== "unknown" &&
    value !== "waiting" &&
    value !== "working"
  ) {
    throw new Error(
      `${path} must be idle, in_flight, unknown, waiting, or working`,
    );
  }
  return value;
}

function workState(value: unknown, path: string): WorkState {
  if (value !== "active" && value !== "draft" && value !== "queued") {
    throw new Error(`${path} must be active, draft, or queued`);
  }
  return value;
}

function workScopeKind(value: unknown, path: string): WorkScopeKind {
  if (value !== "exact" && value !== "recursive") {
    throw new Error(`${path} must be exact or recursive`);
  }
  return value;
}

function findingState(value: unknown, path: string): FindingState {
  if (
    value !== "pending" &&
    value !== "handed-off" &&
    value !== "fixed" &&
    value !== "stale" &&
    value !== "rejected" &&
    value !== "duplicate"
  ) {
    throw new Error(
      `${path} must be pending, handed-off, fixed, stale, rejected, or duplicate`,
    );
  }
  return value;
}

function findingKind(value: unknown, path: string): FindingKind | null {
  if (value === null) return null;
  if (value !== "bug" && value !== "docs" && value !== "improvement") {
    throw new Error(`${path} must be bug, docs, improvement, or null`);
  }
  return value;
}

function validateSession(value: unknown, path: string): void {
  const row = record(value, path);
  client(row.client, `${path}.client`);
  string(row.session_id, `${path}.session_id`);
  string(row.cwd, `${path}.cwd`);
  nullableString(row.repo_root, `${path}.repo_root`);
  sessionState(row.state, `${path}.state`);
  if (row.callsign !== undefined)
    nullableString(row.callsign, `${path}.callsign`);
  nullableString(row.name, `${path}.name`);
  nullableString(row.waiting_for, `${path}.waiting_for`);
  if (row.permission_mode !== undefined)
    nullableString(row.permission_mode, `${path}.permission_mode`);
  boolean(row.coordination_waived, `${path}.coordination_waived`);
  if (row.delegate_count !== undefined)
    unsignedInteger(row.delegate_count, `${path}.delegate_count`);
  if (row.pid !== null) unsignedInteger(row.pid, `${path}.pid`);
  string(row.source, `${path}.source`);
  number(row.started_at, `${path}.started_at`);
  number(row.last_seen, `${path}.last_seen`);
}

function validateWork(value: unknown, path: string): void {
  const row = record(value, path);
  integer(row.id, `${path}.id`);
  client(row.client, `${path}.client`);
  string(row.session_id, `${path}.session_id`);
  string(row.label, `${path}.label`);
  const state = workState(row.state, `${path}.state`);
  if (row.blocked_reason !== undefined)
    nullableString(row.blocked_reason, `${path}.blocked_reason`);
  if (state === "queued" && typeof row.blocked_reason !== "string") {
    throw new Error(`${path}.blocked_reason must describe queued work`);
  }
  const scopeCount = unsignedInteger(row.scope_count, `${path}.scope_count`);
  if (scopeCount < 1) throw new Error(`${path}.scope_count must be positive`);
  if (state === "draft") {
    number(row.draft_created_at, `${path}.draft_created_at`);
    if (row.submitted_at !== undefined)
      throw new Error(`${path}.submitted_at must be omitted for draft work`);
  } else {
    if (row.draft_created_at !== undefined)
      number(row.draft_created_at, `${path}.draft_created_at`);
    number(row.submitted_at, `${path}.submitted_at`);
  }
  if (row.scopes !== undefined)
    throw new Error(`${path}.scopes belongs to a work claim`);
  const claims = array(row.claims, `${path}.claims`);
  if (claims.length < 1) throw new Error(`${path}.claims must not be empty`);
  const claimScopeCount = claims.reduce<number>(
    (total, value, index) =>
      total + validateWorkClaim(value, `${path}.claims[${index}]`, state),
    0,
  );
  if (scopeCount !== claimScopeCount) {
    throw new Error(`${path}.scope_count must equal the claim scope total`);
  }
  number(row.updated_at, `${path}.updated_at`);
}

function validateWorkClaim(
  value: unknown,
  path: string,
  state: WorkState,
): number {
  const claim = record(value, path);
  string(claim.repo_root, `${path}.repo_root`);
  if (claim.blocked_reason !== undefined)
    nullableString(claim.blocked_reason, `${path}.blocked_reason`);
  const scopeCount = unsignedInteger(claim.scope_count, `${path}.scope_count`);
  if (scopeCount < 1) throw new Error(`${path}.scope_count must be positive`);
  if (state === "draft") {
    if (claim.scopes !== undefined)
      throw new Error(`${path}.scopes must be omitted for draft work`);
    return scopeCount;
  }
  const scopes = array(claim.scopes, `${path}.scopes`);
  if (scopes.length < 1) throw new Error(`${path}.scopes must not be empty`);
  scopes.forEach((item, index) => {
    const scope = record(item, `${path}.scopes[${index}]`);
    string(scope.path, `${path}.scopes[${index}].path`);
    workScopeKind(scope.kind, `${path}.scopes[${index}].kind`);
  });
  if (scopeCount !== scopes.length) {
    throw new Error(`${path}.scope_count must match scopes length`);
  }
  return scopeCount;
}

function validateProvider(value: unknown, path: string): void {
  const row = record(value, path);
  client(row.client, `${path}.client`);
  boolean(row.ok, `${path}.ok`);
  string(row.source, `${path}.source`);
  boolean(row.enabled, `${path}.enabled`);
  unsignedInteger(row.dropped, `${path}.dropped`);
  nullableString(row.error, `${path}.error`);
}

function validateFinding(value: unknown, path: string): void {
  const row = record(value, path);
  string(row.id, `${path}.id`);
  string(row.repo_root, `${path}.repo_root`);
  string(row.summary, `${path}.summary`);
  findingKind(row.kind, `${path}.kind`);
  findingState(row.state, `${path}.state`);
  array(row.paths, `${path}.paths`).forEach((item, index) =>
    string(item, `${path}.paths[${index}]`),
  );
  number(row.created_at, `${path}.created_at`);
  number(row.updated_at, `${path}.updated_at`);
  if (row.terminal_at !== null) number(row.terminal_at, `${path}.terminal_at`);
  nullableString(row.handoff_path, `${path}.handoff_path`);
  nullableString(row.commit_oid, `${path}.commit_oid`);
  nullableString(row.canonical_id, `${path}.canonical_id`);
  const sightingCount = integer(row.sighting_count, `${path}.sighting_count`);
  if (sightingCount < 1) {
    throw new Error(`${path}.sighting_count must be positive`);
  }
  boolean(row.triaging, `${path}.triaging`);
}

function validateDelegate(value: unknown, path: string): void {
  const row = record(value, path);
  client(row.parent_client, `${path}.parent_client`);
  string(row.parent_session_id, `${path}.parent_session_id`);
  string(row.agent_id, `${path}.agent_id`);
  nullableString(row.agent_type, `${path}.agent_type`);
  string(row.state, `${path}.state`);
  number(row.last_seen, `${path}.last_seen`);
}

function validateMessage(value: unknown, path: string): void {
  const row = record(value, path);
  string(row.id, `${path}.id`);
  client(row.sender_client, `${path}.sender_client`);
  string(row.sender_session_id, `${path}.sender_session_id`);
  if (row.sender_callsign !== undefined)
    nullableString(row.sender_callsign, `${path}.sender_callsign`);
  client(row.recipient_client, `${path}.recipient_client`);
  string(row.recipient_session_id, `${path}.recipient_session_id`);
  if (row.recipient_callsign !== undefined)
    nullableString(row.recipient_callsign, `${path}.recipient_callsign`);
  nullableString(row.repo_root, `${path}.repo_root`);
  string(row.text, `${path}.text`);
  number(row.created_at, `${path}.created_at`);
  if (row.acknowledged_at !== null)
    number(row.acknowledged_at, `${path}.acknowledged_at`);
}

export function parseSnapshot(value: unknown): Snapshot {
  const snapshot = record(value, "snapshot");
  if (integer(snapshot.schema_version, "snapshot.schema_version") !== 7) {
    throw new Error("snapshot.schema_version must be 7");
  }
  boolean(snapshot.complete, "snapshot.complete");

  const scope = record(snapshot.scope, "snapshot.scope");
  if (scope.kind === "machine") {
    if (scope.repo_root !== undefined) {
      throw new Error("snapshot.scope.repo_root must be omitted for machine scope");
    }
  } else if (scope.kind === "cwd" || scope.kind === "repo") {
    string(scope.repo_root, "snapshot.scope.repo_root");
  } else {
    throw new Error("snapshot.scope.kind must be cwd, machine, or repo");
  }

  if (snapshot.self !== null) {
    const self = record(snapshot.self, "snapshot.self");
    client(self.client, "snapshot.self.client");
    string(self.session_id, "snapshot.self.session_id");
  }

  array(snapshot.providers, "snapshot.providers").forEach((row, index) =>
    validateProvider(row, `snapshot.providers[${index}]`),
  );
  array(snapshot.sessions, "snapshot.sessions").forEach((row, index) =>
    validateSession(row, `snapshot.sessions[${index}]`),
  );
  array(snapshot.work, "snapshot.work").forEach((row, index) =>
    validateWork(row, `snapshot.work[${index}]`),
  );
  array(snapshot.findings, "snapshot.findings").forEach((row, index) =>
    validateFinding(row, `snapshot.findings[${index}]`),
  );
  array(snapshot.handoffs, "snapshot.handoffs").forEach((value, index) => {
    const row = record(value, `snapshot.handoffs[${index}]`);
    string(row.repo_root, `snapshot.handoffs[${index}].repo_root`);
    const count = unsignedInteger(
      row.count,
      `snapshot.handoffs[${index}].count`,
    );
    if (count < 1) {
      throw new Error(`snapshot.handoffs[${index}].count must be positive`);
    }
  });
  array(snapshot.delegates, "snapshot.delegates").forEach((row, index) =>
    validateDelegate(row, `snapshot.delegates[${index}]`),
  );
  array(snapshot.messages, "snapshot.messages").forEach((row, index) =>
    validateMessage(row, `snapshot.messages[${index}]`),
  );

  const outside = record(snapshot.outside_scope, "snapshot.outside_scope");
  unsignedInteger(outside.sessions, "snapshot.outside_scope.sessions");
  unsignedInteger(outside.directories, "snapshot.outside_scope.directories");
  const generatedAt = string(snapshot.generated_at, "snapshot.generated_at");
  if (Number.isNaN(Date.parse(generatedAt))) {
    throw new Error("snapshot.generated_at must be an ISO timestamp");
  }
  unsignedInteger(snapshot.generation, "snapshot.generation");

  return value as Snapshot;
}

export async function fetchSnapshot(signal?: AbortSignal): Promise<Snapshot> {
  const response = await fetch("/api/snapshot", { signal });
  if (!response.ok)
    throw new Error(`Snapshot request failed with HTTP ${response.status}`);
  return parseSnapshot(await response.json());
}

export function subscribeToSnapshots(callbacks: SnapshotCallbacks): () => void {
  let stopped = false;
  let pollingTimer: ReturnType<typeof setInterval> | undefined;
  let pollInFlight = false;
  let latestGeneration: number | undefined;
  const abortController = new AbortController();
  const source = new EventSource("/api/events");

  const deliverSnapshot = (snapshot: Snapshot) => {
    if (
      latestGeneration !== undefined &&
      snapshot.generation < latestGeneration
    ) {
      return;
    }
    latestGeneration = snapshot.generation;
    callbacks.onSnapshot(snapshot);
  };

  const stopPolling = () => {
    if (pollingTimer !== undefined) clearInterval(pollingTimer);
    pollingTimer = undefined;
  };

  const pollOnce = async (): Promise<void> => {
    if (pollInFlight || stopped) return;
    pollInFlight = true;
    try {
      const snapshot = await fetchSnapshot(abortController.signal);
      if (stopped) return;
      deliverSnapshot(snapshot);
      if (source.readyState !== EventSource.OPEN)
        callbacks.onConnectionChange("polling");
    } catch (error) {
      if (stopped) return;
      callbacks.onConnectionChange("disconnected");
      callbacks.onError(
        error instanceof Error ? error : new Error("Snapshot request failed"),
      );
    } finally {
      pollInFlight = false;
    }
  };

  const startPolling = () => {
    if (pollingTimer !== undefined || stopped) return;
    callbacks.onConnectionChange("polling");
    void pollOnce();
    pollingTimer = setInterval(() => void pollOnce(), 2_000);
  };

  source.addEventListener("open", () => {
    if (stopped) return;
    stopPolling();
    callbacks.onConnectionChange("live");
  });
  source.addEventListener("snapshot", (event) => {
    try {
      const snapshot = parseSnapshot(
        JSON.parse((event as MessageEvent<string>).data),
      );
      stopPolling();
      deliverSnapshot(snapshot);
      callbacks.onConnectionChange("live");
    } catch (error) {
      callbacks.onError(
        error instanceof Error ? error : new Error("Invalid snapshot event"),
      );
      startPolling();
    }
  });
  source.addEventListener("error", () => startPolling());

  startPolling();

  return () => {
    stopped = true;
    stopPolling();
    abortController.abort();
    source.close();
  };
}

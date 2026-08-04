import type { ClaimState, Snapshot } from "@/lib/types";

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

function boolean(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") throw new Error(`${path} must be a boolean`);
  return value;
}

function claimState(value: unknown, path: string): ClaimState {
  if (value !== "active" && value !== "queued" && value !== "intent") {
    throw new Error(`${path} must be active, queued, or intent`);
  }
  return value;
}

function validateSession(value: unknown, path: string): void {
  const row = record(value, path);
  string(row.client, `${path}.client`);
  string(row.session_id, `${path}.session_id`);
  string(row.cwd, `${path}.cwd`);
  nullableString(row.repo_root, `${path}.repo_root`);
  string(row.state, `${path}.state`);
  if (row.callsign !== undefined)
    nullableString(row.callsign, `${path}.callsign`);
  nullableString(row.name, `${path}.name`);
  nullableString(row.label, `${path}.label`);
  nullableString(row.waiting_for, `${path}.waiting_for`);
  if (row.pid !== null) integer(row.pid, `${path}.pid`);
  string(row.source, `${path}.source`);
  number(row.started_at, `${path}.started_at`);
  number(row.last_seen, `${path}.last_seen`);
  if (row.claim_state !== undefined)
    claimState(row.claim_state, `${path}.claim_state`);
  if (row.paths !== undefined) {
    array(row.paths, `${path}.paths`).forEach((item, index) =>
      string(item, `${path}.paths[${index}]`),
    );
  }
}

function validateClaim(value: unknown, path: string): void {
  const row = record(value, path);
  integer(row.id, `${path}.id`);
  string(row.client, `${path}.client`);
  string(row.session_id, `${path}.session_id`);
  string(row.repo_root, `${path}.repo_root`);
  string(row.label, `${path}.label`);
  claimState(row.state, `${path}.state`);
  nullableString(row.blocked_reason, `${path}.blocked_reason`);
  array(row.paths, `${path}.paths`).forEach((item, index) =>
    string(item, `${path}.paths[${index}]`),
  );
  number(row.created_at, `${path}.created_at`);
  number(row.updated_at, `${path}.updated_at`);
}

function validateProvider(value: unknown, path: string): void {
  const row = record(value, path);
  string(row.client, `${path}.client`);
  boolean(row.ok, `${path}.ok`);
  string(row.source, `${path}.source`);
  boolean(row.enabled, `${path}.enabled`);
  integer(row.dropped, `${path}.dropped`);
  nullableString(row.error, `${path}.error`);
}

function validateNote(value: unknown, path: string): void {
  const row = record(value, path);
  string(row.id, `${path}.id`);
  string(row.repo_root, `${path}.repo_root`);
  nullableString(row.author_client, `${path}.author_client`);
  nullableString(row.author_session_id, `${path}.author_session_id`);
  string(row.text, `${path}.text`);
  number(row.created_at, `${path}.created_at`);
  if (row.resolved_at !== null) number(row.resolved_at, `${path}.resolved_at`);
}

function validateDelegate(value: unknown, path: string): void {
  const row = record(value, path);
  string(row.parent_client, `${path}.parent_client`);
  string(row.parent_session_id, `${path}.parent_session_id`);
  string(row.agent_id, `${path}.agent_id`);
  nullableString(row.agent_type, `${path}.agent_type`);
  string(row.state, `${path}.state`);
  number(row.last_seen, `${path}.last_seen`);
}

function validateMessage(value: unknown, path: string): void {
  const row = record(value, path);
  string(row.id, `${path}.id`);
  string(row.sender_client, `${path}.sender_client`);
  string(row.sender_session_id, `${path}.sender_session_id`);
  if (row.sender_callsign !== undefined)
    nullableString(row.sender_callsign, `${path}.sender_callsign`);
  string(row.recipient_client, `${path}.recipient_client`);
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
  integer(snapshot.schema_version, "snapshot.schema_version");
  boolean(snapshot.complete, "snapshot.complete");

  const scope = record(snapshot.scope, "snapshot.scope");
  string(scope.kind, "snapshot.scope.kind");
  if (scope.repo_root !== undefined)
    string(scope.repo_root, "snapshot.scope.repo_root");

  if (snapshot.self !== null) {
    const self = record(snapshot.self, "snapshot.self");
    string(self.client, "snapshot.self.client");
    string(self.session_id, "snapshot.self.session_id");
  }

  array(snapshot.providers, "snapshot.providers").forEach((row, index) =>
    validateProvider(row, `snapshot.providers[${index}]`),
  );
  array(snapshot.sessions, "snapshot.sessions").forEach((row, index) =>
    validateSession(row, `snapshot.sessions[${index}]`),
  );
  array(snapshot.claims, "snapshot.claims").forEach((row, index) =>
    validateClaim(row, `snapshot.claims[${index}]`),
  );
  array(snapshot.notes, "snapshot.notes").forEach((row, index) =>
    validateNote(row, `snapshot.notes[${index}]`),
  );
  array(snapshot.delegates, "snapshot.delegates").forEach((row, index) =>
    validateDelegate(row, `snapshot.delegates[${index}]`),
  );
  array(snapshot.messages, "snapshot.messages").forEach((row, index) =>
    validateMessage(row, `snapshot.messages[${index}]`),
  );

  const outside = record(snapshot.outside_scope, "snapshot.outside_scope");
  integer(outside.sessions, "snapshot.outside_scope.sessions");
  integer(outside.directories, "snapshot.outside_scope.directories");
  const generatedAt = string(snapshot.generated_at, "snapshot.generated_at");
  if (Number.isNaN(Date.parse(generatedAt))) {
    throw new Error("snapshot.generated_at must be an ISO timestamp");
  }
  integer(snapshot.generation, "snapshot.generation");

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
  const abortController = new AbortController();
  const source = new EventSource("/api/events");

  const stopPolling = () => {
    if (pollingTimer !== undefined) clearInterval(pollingTimer);
    pollingTimer = undefined;
  };

  const pollOnce = async (): Promise<boolean> => {
    try {
      const snapshot = await fetchSnapshot(abortController.signal);
      if (stopped) return false;
      callbacks.onSnapshot(snapshot);
      if (source.readyState !== EventSource.OPEN)
        callbacks.onConnectionChange("polling");
      return true;
    } catch (error) {
      if (stopped) return false;
      callbacks.onConnectionChange("disconnected");
      callbacks.onError(
        error instanceof Error ? error : new Error("Snapshot request failed"),
      );
      return false;
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
      callbacks.onSnapshot(snapshot);
      callbacks.onConnectionChange("live");
    } catch (error) {
      callbacks.onError(
        error instanceof Error ? error : new Error("Invalid snapshot event"),
      );
      startPolling();
    }
  });
  source.addEventListener("error", () => startPolling());

  void pollOnce().then((success) => {
    if (!success) startPolling();
  });

  return () => {
    stopped = true;
    stopPolling();
    abortController.abort();
    source.close();
  };
}

import { Schema, SchemaIssue } from "effect";

import { SnapshotSchema } from "./snapshot-schema.js";
import type { Snapshot } from "./types.js";

export type ConnectionState = "connecting" | "live" | "polling" | "disconnected";

type SnapshotCallbacks = {
  onSnapshot: (snapshot: Snapshot) => void;
  onConnectionChange: (state: ConnectionState) => void;
  onError: (error: Error) => void;
};

const formatIssue = SchemaIssue.makeFormatterStandardSchemaV1();

export function parseSnapshot(value: unknown): Snapshot {
  try {
    Schema.asserts(SnapshotSchema, value);
    return value;
  } catch (error) {
    if (!(error instanceof Error) || !SchemaIssue.isIssue(error.cause)) {
      throw error;
    }
    const message = formatIssue(error.cause)
      .issues.map((issue) => {
        let path = "snapshot";
        for (const segment of issue.path ?? []) {
          const key = typeof segment === "object" ? segment.key : segment;
          path += typeof key === "number" ? `[${key}]` : `.${String(key)}`;
        }
        return `${path} ${issue.message}`;
      })
      .join("\n");
    throw new Error(message, { cause: error });
  }
}

export async function fetchSnapshot(signal?: AbortSignal): Promise<Snapshot> {
  const response = await fetch("/api/snapshot", { signal });
  if (!response.ok) {
    throw new Error(`Snapshot request failed with HTTP ${response.status}`);
  }
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
    if (latestGeneration !== undefined && snapshot.generation < latestGeneration) {
      return;
    }
    latestGeneration = snapshot.generation;
    callbacks.onSnapshot(snapshot);
  };

  const stopPolling = () => {
    if (pollingTimer !== undefined) {
      clearInterval(pollingTimer);
    }
    pollingTimer = undefined;
  };

  const pollOnce = async (): Promise<void> => {
    if (pollInFlight || stopped) {
      return;
    }
    pollInFlight = true;
    try {
      const snapshot = await fetchSnapshot(abortController.signal);
      if (stopped) {
        return;
      }
      deliverSnapshot(snapshot);
      if (source.readyState !== EventSource.OPEN) {
        callbacks.onConnectionChange("polling");
      }
    } catch (error) {
      if (stopped) {
        return;
      }
      callbacks.onConnectionChange("disconnected");
      callbacks.onError(error instanceof Error ? error : new Error("Snapshot request failed"));
    } finally {
      pollInFlight = false;
    }
  };

  const startPolling = () => {
    if (pollingTimer !== undefined || stopped) {
      return;
    }
    callbacks.onConnectionChange("polling");
    pollOnce();
    pollingTimer = setInterval(pollOnce, 2000);
  };

  source.addEventListener("open", () => {
    if (stopped) {
      return;
    }
    stopPolling();
    callbacks.onConnectionChange("live");
  });
  source.addEventListener("snapshot", (event) => {
    try {
      const snapshot = parseSnapshot(JSON.parse((event as MessageEvent<string>).data));
      stopPolling();
      deliverSnapshot(snapshot);
      callbacks.onConnectionChange("live");
    } catch (error) {
      callbacks.onError(error instanceof Error ? error : new Error("Invalid snapshot event"));
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

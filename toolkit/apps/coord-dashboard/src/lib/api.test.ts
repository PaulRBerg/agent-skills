import { afterEach, describe, expect, test, vi } from "vitest";
import { parseSnapshot, subscribeToSnapshots } from "@/lib/api";
import { sampleSnapshot } from "@/lib/sample-snapshot";

type EventListener = (event: Event | MessageEvent<string>) => void;

class FakeEventSource {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSED = 2;
  static instances: FakeEventSource[] = [];

  readonly listeners = new Map<string, EventListener[]>();
  readyState = FakeEventSource.CONNECTING;
  closed = false;

  constructor(readonly url: string) {
    FakeEventSource.instances.push(this);
  }

  addEventListener(type: string, listener: EventListener): void {
    const listeners = this.listeners.get(type) ?? [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }

  emitSnapshot(snapshot: unknown): void {
    this.readyState = FakeEventSource.OPEN;
    const event = { data: JSON.stringify(snapshot) } as MessageEvent<string>;
    for (const listener of this.listeners.get("snapshot") ?? []) listener(event);
  }

  close(): void {
    this.closed = true;
    this.readyState = FakeEventSource.CLOSED;
  }
}

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  FakeEventSource.instances = [];
});

async function flushPromises(): Promise<void> {
  for (let index = 0; index < 4; index += 1) await Promise.resolve();
}

describe("parseSnapshot", () => {
  test("accepts the committed snapshot fixture with matching parent totals", () => {
    expect(parseSnapshot(sampleSnapshot)).toBe(sampleSnapshot);
  });

  test("requires a parent scope total", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const work = malformed.work as Array<Record<string, unknown>>;
    delete work[1]?.scope_count;

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.work[1].scope_count",
    );
  });

  test("rejects parent scope totals that differ from their claims", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const work = malformed.work as Array<Record<string, unknown>>;
    work[1] = { ...work[1], scope_count: 2 };

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.work[1].scope_count must equal the claim scope total",
    );
  });

  test("accepts matching aggregate scope totals", () => {
    const valid = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const work = valid.work as Array<Record<string, unknown>>;
    const claims = work[1]?.claims as Array<Record<string, unknown>>;
    work[1] = {
      ...work[1],
      scope_count: 2,
      claims: [
        {
          ...claims[0],
          scope_count: 2,
          scopes: [
            { path: "apps/coord-dashboard", kind: "recursive" },
            { path: "apps/coord-dashboard/src", kind: "recursive" },
          ],
        },
      ],
    };

    expect(parseSnapshot(valid)).toBe(valid);
  });

  test("rejects malformed nested records with a useful field path", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const sessions = malformed.sessions as Array<Record<string, unknown>>;
    sessions[0] = { ...sessions[0], last_seen: "recently" };

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.sessions[0].last_seen",
    );
  });

  test("rejects unsupported work states", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const work = malformed.work as Array<Record<string, unknown>>;
    work[0] = { ...work[0], state: "blocked" };

    expect(() => parseSnapshot(malformed)).toThrow("snapshot.work[0].state");
  });

  test("allows additive API fields", () => {
    const extended = { ...sampleSnapshot, server_version: "0.3.0" };
    expect(parseSnapshot(extended)).toBe(extended);
  });

  test("rejects the previous status schema", () => {
    const legacy = { ...sampleSnapshot, schema_version: 5 };
    expect(() => parseSnapshot(legacy)).toThrow(
      "snapshot.schema_version must be 7",
    );
  });

  test("requires the coordination waiver on every session", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const sessions = malformed.sessions as Array<Record<string, unknown>>;
    delete sessions[0]?.coordination_waived;

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.sessions[0].coordination_waived",
    );
  });

  test("allows absent additive callsign fields", () => {
    const withoutCallsigns = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    for (const session of withoutCallsigns.sessions as Array<
      Record<string, unknown>
    >) {
      delete session.callsign;
    }
    for (const message of withoutCallsigns.messages as Array<
      Record<string, unknown>
    >) {
      delete message.sender_callsign;
      delete message.recipient_callsign;
    }

    expect(parseSnapshot(withoutCallsigns)).toBe(withoutCallsigns);
  });

  test("requires draft claim counts without exposing literal scopes", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const work = malformed.work as Array<Record<string, unknown>>;
    const claims = work[0]?.claims as Array<Record<string, unknown>>;
    work[0] = {
      ...work[0],
      claims: [
        {
          ...claims[0],
          scopes: [{ path: "private/file", kind: "exact" }],
        },
      ],
    };

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.work[0].claims[0].scopes must be omitted for draft work",
    );
  });

  test("rejects malformed nested work claims", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const work = malformed.work as Array<Record<string, unknown>>;
    const claims = work[1]?.claims as Array<Record<string, unknown>>;
    work[1] = { ...work[1], claims: [{ ...claims[0], repo_root: 42 }] };

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.work[1].claims[0].repo_root",
    );
  });

  test("validates additive callsign fields when present", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const messages = malformed.messages as Array<Record<string, unknown>>;
    messages[0] = { ...messages[0], sender_callsign: 42 };

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.messages[0].sender_callsign",
    );
  });

  test("rejects malformed finding state", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const findings = malformed.findings as Array<Record<string, unknown>>;
    findings[0] = { ...findings[0], state: "open" };

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.findings[0].state",
    );
  });

  test("rejects malformed finding kind", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const findings = malformed.findings as Array<Record<string, unknown>>;
    findings[0] = { ...findings[0], kind: "coverage" };

    expect(() => parseSnapshot(malformed)).toThrow("snapshot.findings[0].kind");
  });

  test("rejects malformed triage overlays", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const findings = malformed.findings as Array<Record<string, unknown>>;
    findings[0] = { ...findings[0], triaging: "leased" };

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.findings[0].triaging",
    );
  });

  test("rejects enum values outside the Rust v7 contract", () => {
    const invalidClient = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const providers = invalidClient.providers as Array<Record<string, unknown>>;
    providers[0] = { ...providers[0], client: "cursor" };
    expect(() => parseSnapshot(invalidClient)).toThrow(
      "snapshot.providers[0].client must be claude or codex",
    );

    const invalidState = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const sessions = invalidState.sessions as Array<Record<string, unknown>>;
    sessions[0] = { ...sessions[0], state: "paused" };
    expect(() => parseSnapshot(invalidState)).toThrow(
      "snapshot.sessions[0].state",
    );

    const invalidScope = {
      ...sampleSnapshot,
      scope: { kind: "organization" },
    };
    expect(() => parseSnapshot(invalidScope)).toThrow(
      "snapshot.scope.kind must be cwd, machine, or repo",
    );
  });

  test("rejects negative Rust unsigned counters", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const outsideScope = malformed.outside_scope as Record<string, unknown>;
    outsideScope.sessions = -1;

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.outside_scope.sessions must be non-negative",
    );
  });
});

describe("subscribeToSnapshots", () => {
  test("does not let an older polling response replace a newer SSE snapshot", async () => {
    let resolveFetch!: (response: Response) => void;
    const response = new Promise<Response>((resolve) => {
      resolveFetch = resolve;
    });
    vi.stubGlobal("EventSource", FakeEventSource);
    vi.stubGlobal("fetch", vi.fn(() => response));
    const received: number[] = [];

    const stop = subscribeToSnapshots({
      onSnapshot: (snapshot) => received.push(snapshot.generation),
      onConnectionChange: () => undefined,
      onError: () => undefined,
    });
    FakeEventSource.instances[0]?.emitSnapshot({
      ...sampleSnapshot,
      generation: sampleSnapshot.generation + 1,
    });
    resolveFetch(new Response(JSON.stringify(sampleSnapshot)));
    await flushPromises();

    expect(received).toEqual([sampleSnapshot.generation + 1]);
    stop();
    expect(FakeEventSource.instances[0]?.closed).toBe(true);
  });

  test("polls continuously without overlapping slow requests", async () => {
    vi.useFakeTimers();
    let resolveFirst!: (response: Response) => void;
    const firstResponse = new Promise<Response>((resolve) => {
      resolveFirst = resolve;
    });
    const fetchMock = vi
      .fn<() => Promise<Response>>()
      .mockReturnValueOnce(firstResponse)
      .mockResolvedValue(new Response(JSON.stringify(sampleSnapshot)));
    vi.stubGlobal("EventSource", FakeEventSource);
    vi.stubGlobal("fetch", fetchMock);

    const stop = subscribeToSnapshots({
      onSnapshot: () => undefined,
      onConnectionChange: () => undefined,
      onError: () => undefined,
    });
    await vi.advanceTimersByTimeAsync(6_000);
    expect(fetchMock).toHaveBeenCalledTimes(1);

    resolveFirst(new Response(JSON.stringify(sampleSnapshot)));
    await flushPromises();
    await vi.advanceTimersByTimeAsync(2_000);
    expect(fetchMock).toHaveBeenCalledTimes(2);
    stop();
  });
});

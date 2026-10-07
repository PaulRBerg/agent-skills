import { afterEach, describe, expect, test, vi } from "vitest";

import { parseSnapshot, subscribeToSnapshots } from "./api.js";
import { sampleSnapshot } from "./sample-snapshot.js";

type EventListener = (event: Event | MessageEvent<string>) => void;

class FakeEventSource {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSED = 2;
  static instances: FakeEventSource[] = [];

  readonly listeners = new Map<string, EventListener[]>();
  readonly url: string;
  readyState = FakeEventSource.CONNECTING;
  closed = false;

  constructor(url: string) {
    this.url = url;
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
    for (const listener of this.listeners.get("snapshot") ?? []) {
      listener(event);
    }
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
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

describe("parseSnapshot", () => {
  test("accepts the committed snapshot fixture with matching parent totals", () => {
    expect(parseSnapshot(sampleSnapshot)).toBe(sampleSnapshot);
  });

  test("requires a parent scope total", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const work = malformed.work as Record<string, unknown>[];
    delete work[1]?.scope_count;

    expect(() => parseSnapshot(malformed)).toThrow("snapshot.work[1].scope_count");
  });

  test("rejects parent scope totals that differ from their claims", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const work = malformed.work as Record<string, unknown>[];
    work[1] = { ...work[1], scope_count: 2 };

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.work[1].scope_count must equal the claim scope total"
    );
  });

  test("accepts matching aggregate scope totals", () => {
    const valid = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const work = valid.work as Record<string, unknown>[];
    const claims = work[1]?.claims as Record<string, unknown>[];
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
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const sessions = malformed.sessions as Record<string, unknown>[];
    sessions[0] = { ...sessions[0], last_seen: "recently" };

    expect(() => parseSnapshot(malformed)).toThrow("snapshot.sessions[0].last_seen");
  });

  test("rejects unsupported work states", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const work = malformed.work as Record<string, unknown>[];
    work[0] = { ...work[0], state: "blocked" };

    expect(() => parseSnapshot(malformed)).toThrow("snapshot.work[0].state");
  });

  test("allows additive API fields", () => {
    const extended = {
      ...sampleSnapshot,
      server_version: "0.3.0",
      sessions: sampleSnapshot.sessions.map((session) => ({
        ...session,
        server_detail: { supported: true },
      })),
    };
    expect(parseSnapshot(extended)).toBe(extended);
    expect(parseSnapshot(extended).sessions[0]).toBe(extended.sessions[0]);
  });

  test("rejects the previous status schema", () => {
    const legacy = { ...sampleSnapshot, schema_version: 7 };
    expect(() => parseSnapshot(legacy)).toThrow("snapshot.schema_version Expected 8");
  });

  test("requires the coordination waiver on every session", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const sessions = malformed.sessions as Record<string, unknown>[];
    delete sessions[0]?.coordination_waived;

    expect(() => parseSnapshot(malformed)).toThrow("snapshot.sessions[0].coordination_waived");
  });

  test("allows absent additive callsign fields", () => {
    const withoutCallsigns = structuredClone(sampleSnapshot) as Record<string, unknown>;
    for (const session of withoutCallsigns.sessions as Record<string, unknown>[]) {
      delete session.callsign;
    }
    for (const message of withoutCallsigns.messages as Record<string, unknown>[]) {
      delete message.sender_callsign;
      delete message.recipient_callsign;
    }

    expect(parseSnapshot(withoutCallsigns)).toBe(withoutCallsigns);
  });

  test("rejects a work claim without literal scopes", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const work = malformed.work as Record<string, unknown>[];
    const claims = work[0]?.claims as Record<string, unknown>[];
    const { scopes: _scopes, ...claimWithoutScopes } = claims[0] ?? {};
    work[0] = { ...work[0], claims: [claimWithoutScopes] };

    expect(() => parseSnapshot(malformed)).toThrow("snapshot.work[0].claims[0].scopes Missing key");
  });

  test("rejects a draft claim that exposes literal scopes", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const drafts = malformed.drafts as Record<string, unknown>[];
    const claims = drafts[0]?.claims as Record<string, unknown>[];
    drafts[0] = {
      ...drafts[0],
      claims: [{ ...claims[0], scopes: [{ path: "private/file", kind: "exact" }] }],
    };

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.drafts[0].claims[0].scopes must be omitted for a draft claim"
    );
  });

  test("rejects a draft with both name and owner", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const drafts = malformed.drafts as Record<string, unknown>[];
    drafts[1] = {
      ...drafts[1],
      name: "duplicate-owner-and-name",
      owner: { client: "codex", session_id: "some-session" },
    };

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.drafts[1] must have exactly one of name or owner"
    );
  });

  test("rejects a draft with neither name nor owner", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const drafts = malformed.drafts as Record<string, unknown>[];
    drafts[0] = { ...drafts[0], name: null, owner: null };

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.drafts[0] must have exactly one of name or owner"
    );
  });

  test("accepts a named draft and a session-owned draft", () => {
    expect(parseSnapshot(sampleSnapshot)).toBe(sampleSnapshot);
    const drafts = sampleSnapshot.drafts;
    expect(drafts[0]?.name).not.toBeNull();
    expect(drafts[0]?.owner).toBeNull();
    expect(drafts[1]?.name).toBeNull();
    expect(drafts[1]?.owner).not.toBeNull();
  });

  test("rejects malformed nested work claims", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const work = malformed.work as Record<string, unknown>[];
    const claims = work[1]?.claims as Record<string, unknown>[];
    work[1] = { ...work[1], claims: [{ ...claims[0], repo_root: 42 }] };

    expect(() => parseSnapshot(malformed)).toThrow("snapshot.work[1].claims[0].repo_root");
  });

  test("validates additive callsign fields when present", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const messages = malformed.messages as Record<string, unknown>[];
    messages[0] = { ...messages[0], sender_callsign: 42 };

    expect(() => parseSnapshot(malformed)).toThrow("snapshot.messages[0].sender_callsign");
  });

  test("rejects malformed finding state", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const findings = malformed.findings as Record<string, unknown>[];
    findings[0] = { ...findings[0], state: "open" };

    expect(() => parseSnapshot(malformed)).toThrow("snapshot.findings[0].state");
  });

  test("rejects malformed finding kind", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const findings = malformed.findings as Record<string, unknown>[];
    findings[0] = { ...findings[0], kind: "coverage" };

    expect(() => parseSnapshot(malformed)).toThrow("snapshot.findings[0].kind");
  });

  test("rejects malformed triage overlays", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const findings = malformed.findings as Record<string, unknown>[];
    findings[0] = { ...findings[0], triaging: "leased" };

    expect(() => parseSnapshot(malformed)).toThrow("snapshot.findings[0].triaging");
  });

  test("rejects enum values outside the Rust v8 contract", () => {
    const invalidClient = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const providers = invalidClient.providers as Record<string, unknown>[];
    providers[0] = { ...providers[0], client: "cursor" };
    expect(() => parseSnapshot(invalidClient)).toThrow(
      'snapshot.providers[0].client Expected "claude" | "codex"'
    );

    const invalidState = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const sessions = invalidState.sessions as Record<string, unknown>[];
    sessions[0] = { ...sessions[0], state: "paused" };
    expect(() => parseSnapshot(invalidState)).toThrow("snapshot.sessions[0].state");

    const invalidScope = {
      ...sampleSnapshot,
      scope: { kind: "organization" },
    };
    expect(() => parseSnapshot(invalidScope)).toThrow("snapshot.scope.kind");
  });

  test("rejects negative Rust unsigned counters", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<string, unknown>;
    const outsideScope = malformed.outside_scope as Record<string, unknown>;
    outsideScope.sessions = -1;

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.outside_scope.sessions Expected a value greater than or equal to 0"
    );
  });

  test.each([
    ["NaN", Number.NaN],
    ["positive infinity", Infinity],
    ["negative infinity", -Infinity],
  ])("rejects %s in numeric fields", (_label, value) => {
    expect(() =>
      parseSnapshot({
        ...sampleSnapshot,
        sessions: [{ ...sampleSnapshot.sessions[0], last_seen: value }],
      })
    ).toThrow("snapshot.sessions[0].last_seen");
  });

  test.each([-1, 0.5, Number.NaN, Infinity])("rejects generation counter %s", (generation) => {
    expect(() => parseSnapshot({ ...sampleSnapshot, generation })).toThrow("snapshot.generation");
  });

  test("preserves finite integer acceptance beyond the safe integer range", () => {
    const snapshot = { ...sampleSnapshot, generation: 2 ** 53 };
    expect(parseSnapshot(snapshot)).toBe(snapshot);
  });

  test("rejects invalid timestamps", () => {
    expect(() => parseSnapshot({ ...sampleSnapshot, generated_at: "not-a-date" })).toThrow(
      "snapshot.generated_at"
    );
  });

  test.each(["cwd", "repo"])("requires a repository root for %s scope", (kind) => {
    expect(() => parseSnapshot({ ...sampleSnapshot, scope: { kind } })).toThrow(
      "snapshot.scope.repo_root"
    );
  });

  test("rejects repository roots on machine scope", () => {
    expect(() =>
      parseSnapshot({
        ...sampleSnapshot,
        scope: { kind: "machine", repo_root: "/repo" },
      })
    ).toThrow("snapshot.scope.repo_root");
  });

  test("requires nullable fields while accepting null", () => {
    const session = { ...sampleSnapshot.sessions[0], pid: null, repo_root: null };
    const snapshot = { ...sampleSnapshot, sessions: [session] };
    expect(parseSnapshot(snapshot)).toBe(snapshot);
    expect(() =>
      parseSnapshot({
        ...snapshot,
        sessions: [{ ...session, pid: undefined }],
      })
    ).toThrow("snapshot.sessions[0].pid");
    expect(() =>
      parseSnapshot({
        ...snapshot,
        sessions: [{ ...session, repo_root: undefined }],
      })
    ).toThrow("snapshot.sessions[0].repo_root");
  });

  test("accepts absent or explicitly undefined optional fields", () => {
    const snapshot = {
      ...sampleSnapshot,
      sessions: [
        {
          ...sampleSnapshot.sessions[0],
          callsign: undefined,
          permission_mode: undefined,
          delegate_count: undefined,
        },
      ],
    };
    expect(parseSnapshot(snapshot)).toBe(snapshot);
  });

  test.each([-1, 1.5])("rejects delegate counter %s", (delegate_count) => {
    expect(() =>
      parseSnapshot({
        ...sampleSnapshot,
        sessions: [{ ...sampleSnapshot.sessions[0], delegate_count }],
      })
    ).toThrow("snapshot.sessions[0].delegate_count");
  });

  test("requires queued work to describe its blocker", () => {
    expect(() =>
      parseSnapshot({
        ...sampleSnapshot,
        work: [{ ...sampleSnapshot.work[0], state: "queued", blocked_reason: null }],
      })
    ).toThrow("snapshot.work[0].blocked_reason must describe queued work");
  });

  test("requires work claims and forbids parent literal scopes", () => {
    expect(() =>
      parseSnapshot({
        ...sampleSnapshot,
        work: [{ ...sampleSnapshot.work[0], claims: [] }],
      })
    ).toThrow("snapshot.work[0].claims");
    expect(() =>
      parseSnapshot({
        ...sampleSnapshot,
        work: [{ ...sampleSnapshot.work[0], scopes: [] }],
      })
    ).toThrow("snapshot.work[0].scopes belongs to a work claim");
  });

  test("requires positive claim counts matching nonempty literal scopes", () => {
    const work = sampleSnapshot.work[0];
    if (!work) {
      throw new Error("Fixture must include work");
    }
    const claim = work.claims[0];
    if (!claim) {
      throw new Error("Fixture work must include a claim");
    }
    for (const malformed of [
      { ...claim, scope_count: 0 },
      { ...claim, scope_count: 1.5 },
      { ...claim, scopes: [] },
      { ...claim, scope_count: claim.scope_count + 1 },
    ]) {
      expect(() =>
        parseSnapshot({
          ...sampleSnapshot,
          work: [{ ...work, claims: [malformed] }],
        })
      ).toThrow("snapshot.work[0].claims[0]");
    }
  });

  test("requires nonempty draft claims with positive counts", () => {
    const draft = sampleSnapshot.drafts[0];
    if (!draft) {
      throw new Error("Fixture must include a draft");
    }
    expect(() =>
      parseSnapshot({
        ...sampleSnapshot,
        drafts: [{ ...draft, claims: [] }],
      })
    ).toThrow("snapshot.drafts[0].claims");
    expect(() =>
      parseSnapshot({
        ...sampleSnapshot,
        drafts: [{ ...draft, claims: [{ ...draft.claims[0], scope_count: 0 }] }],
      })
    ).toThrow("snapshot.drafts[0].claims[0].scope_count");
  });

  test("requires positive finding and handoff counts", () => {
    expect(() =>
      parseSnapshot({
        ...sampleSnapshot,
        findings: [{ ...sampleSnapshot.findings[0], sighting_count: 0 }],
      })
    ).toThrow("snapshot.findings[0].sighting_count");
    expect(() =>
      parseSnapshot({
        ...sampleSnapshot,
        handoffs: [{ repo_root: "/repo", count: 0 }],
      })
    ).toThrow("snapshot.handoffs[0].count");
  });
});

describe("subscribeToSnapshots", () => {
  test("does not let an older polling response replace a newer SSE snapshot", async () => {
    const { promise: response, resolve: resolveFetch } = Promise.withResolvers<Response>();
    vi.stubGlobal("EventSource", FakeEventSource);
    vi.stubGlobal(
      "fetch",
      vi.fn(() => response)
    );
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
    resolveFetch(Response.json(sampleSnapshot));
    await flushPromises();

    expect(received).toEqual([sampleSnapshot.generation + 1]);
    stop();
    expect(FakeEventSource.instances[0]?.closed).toBe(true);
  });

  test("polls continuously without overlapping slow requests", async () => {
    vi.useFakeTimers();
    const { promise: firstResponse, resolve: resolveFirst } = Promise.withResolvers<Response>();
    const fetchMock = vi
      .fn<() => Promise<Response>>()
      .mockReturnValueOnce(firstResponse)
      .mockResolvedValue(Response.json(sampleSnapshot));
    vi.stubGlobal("EventSource", FakeEventSource);
    vi.stubGlobal("fetch", fetchMock);

    const stop = subscribeToSnapshots({
      onSnapshot: () => undefined,
      onConnectionChange: () => undefined,
      onError: () => undefined,
    });
    await vi.advanceTimersByTimeAsync(6000);
    expect(fetchMock).toHaveBeenCalledTimes(1);

    resolveFirst(Response.json(sampleSnapshot));
    await flushPromises();
    await vi.advanceTimersByTimeAsync(2000);
    expect(fetchMock).toHaveBeenCalledTimes(2);
    stop();
  });

  test("aborts pending polling and ignores its response after cleanup", async () => {
    const { promise: response, resolve: resolveFetch } = Promise.withResolvers<Response>();
    const fetchMock = vi.fn<(...args: Parameters<typeof fetch>) => ReturnType<typeof fetch>>(
      () => response
    );
    vi.stubGlobal("EventSource", FakeEventSource);
    vi.stubGlobal("fetch", fetchMock);
    const onSnapshot = vi.fn();
    const onConnectionChange = vi.fn();
    const onError = vi.fn();

    const stop = subscribeToSnapshots({ onSnapshot, onConnectionChange, onError });
    const options: RequestInit | undefined = fetchMock.mock.calls[0]?.[1];
    expect(options?.signal?.aborted).toBe(false);
    stop();
    expect(options?.signal?.aborted).toBe(true);
    onConnectionChange.mockClear();
    resolveFetch(Response.json(sampleSnapshot));
    await flushPromises();

    expect(onSnapshot).not.toHaveBeenCalled();
    expect(onConnectionChange).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
  });
});

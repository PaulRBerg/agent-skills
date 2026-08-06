import { describe, expect, test } from "vitest";
import { parseSnapshot } from "@/lib/api";
import { sampleSnapshot } from "@/lib/sample-snapshot";

describe("parseSnapshot", () => {
  test("accepts the committed snapshot fixture", () => {
    expect(parseSnapshot(sampleSnapshot)).toBe(sampleSnapshot);
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

  test("rejects the pre-break status schema", () => {
    const legacy = { ...sampleSnapshot, schema_version: 1 };
    expect(() => parseSnapshot(legacy)).toThrow(
      "snapshot.schema_version must be 2",
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

  test("requires draft counts without exposing literal scopes", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const work = malformed.work as Array<Record<string, unknown>>;
    work[0] = {
      ...work[0],
      scopes: [{ path: "private/file", kind: "exact" }],
    };

    expect(() => parseSnapshot(malformed)).toThrow(
      "snapshot.work[0].scopes must be omitted for draft work",
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
});

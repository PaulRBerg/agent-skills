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

  test("rejects unsupported claim states", () => {
    const malformed = structuredClone(sampleSnapshot) as Record<
      string,
      unknown
    >;
    const claims = malformed.claims as Array<Record<string, unknown>>;
    claims[0] = { ...claims[0], state: "blocked" };

    expect(() => parseSnapshot(malformed)).toThrow("snapshot.claims[0].state");
  });

  test("allows additive API fields", () => {
    const extended = { ...sampleSnapshot, server_version: "0.3.0" };
    expect(parseSnapshot(extended)).toBe(extended);
  });

  test("accepts older schema-v1 snapshots without callsign fields", () => {
    const legacy = structuredClone(sampleSnapshot) as Record<string, unknown>;
    for (const session of legacy.sessions as Array<Record<string, unknown>>) {
      delete session.callsign;
    }
    for (const message of legacy.messages as Array<Record<string, unknown>>) {
      delete message.sender_callsign;
      delete message.recipient_callsign;
    }

    expect(parseSnapshot(legacy)).toBe(legacy);
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

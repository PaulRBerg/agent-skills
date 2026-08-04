import { describe, expect, test } from "vitest";
import {
  formatRelativeTime,
  formatUpdatedTime,
  getLivenessTier,
  messageEndpointName,
  sessionDisplayName,
} from "@/lib/format";

describe("formatRelativeTime", () => {
  test.each([
    [100, 102, "just now"],
    [100, 117, "17s ago"],
    [100, 220, "2m ago"],
    [100, 7_300, "2h ago"],
    [100, 172_900, "2d ago"],
  ])("formats %s relative to %s as %s", (timestamp, now, expected) => {
    expect(formatRelativeTime(timestamp, now)).toBe(expected);
  });

  test("uses exact seconds for the header's first minute", () => {
    expect(formatUpdatedTime(100, 102)).toBe("2s ago");
    expect(formatUpdatedTime(100, 220)).toBe("2m ago");
  });
});

describe("getLivenessTier", () => {
  test.each([
    [29, "fresh"],
    [30, "aging"],
    [299, "aging"],
    [300, "stale"],
  ] as const)("classifies an age of %s seconds as %s", (age, expected) => {
    expect(getLivenessTier(1_000 - age, 1_000)).toBe(expected);
  });
});

describe("identity display", () => {
  const session = {
    callsign: "👩‍💻 Baroness Byte",
    label: "dashboard work",
    name: "provider-name",
    session_id: "019fcbf9-d75c-7ba3-a481-18068ea954eb",
  };

  test("prefers callsign, task label, provider name, then short ID", () => {
    expect(sessionDisplayName(session)).toBe("👩‍💻 Baroness Byte");
    expect(sessionDisplayName({ ...session, callsign: null })).toBe(
      "dashboard work",
    );
    expect(
      sessionDisplayName({ ...session, callsign: null, label: null }),
    ).toBe("provider-name");
    expect(
      sessionDisplayName({
        ...session,
        callsign: undefined,
        label: null,
        name: null,
      }),
    ).toBe("019fcbf9");
  });

  test("uses immutable message callsigns with a short-ID fallback", () => {
    expect(messageEndpointName("🦊 Historical Fox", "sender-session")).toBe(
      "🦊 Historical Fox",
    );
    expect(messageEndpointName(undefined, "sender-session")).toBe("sender-s");
  });
});

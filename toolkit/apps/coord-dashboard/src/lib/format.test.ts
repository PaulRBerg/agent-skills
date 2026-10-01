import { describe, expect, test } from "vitest";
import {
  displayPath,
  formatRelativeTime,
  formatUpdatedTime,
  getLivenessTier,
  messageEndpointName,
  sessionDisplayName,
  shortenPath,
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

describe("path display", () => {
  const HOME = "/Users/testuser";

  test.each([
    ["/Users/testuser", "~"],
    ["/Users/testuser/projects/agent-toolkit", "~/projects/agent-toolkit"],
    ["/Users/testuser-work/project", "/Users/testuser-work/project"],
    ["/tmp/project", "/tmp/project"],
  ])("replaces the home directory in %s", (value, expected) => {
    expect(displayPath(value, HOME)).toBe(expected);
  });

  test("only abbreviates at a path boundary, never a plain substring", () => {
    expect(displayPath("/Users/testuser-old", HOME)).toBe(
      "/Users/testuser-old",
    );
    expect(displayPath("/Users/testuserx/project", HOME)).toBe(
      "/Users/testuserx/project",
    );
  });

  test("handles a trailing slash on the home directory", () => {
    expect(displayPath("/Users/testuser", `${HOME}/`)).toBe("~");
    expect(displayPath("/Users/testuser/projects", `${HOME}/`)).toBe(
      "~/projects",
    );
  });

  test("leaves paths untouched when no home directory is configured", () => {
    expect(displayPath("/Users/testuser")).toBe("/Users/testuser");
  });

  test("shortens display paths after replacing the home directory", () => {
    expect(shortenPath("/Users/testuser", HOME)).toBe("~");
    expect(shortenPath("/Users/testuser/projects/agent-toolkit", HOME)).toBe(
      "~/projects/agent-toolkit",
    );
  });
});

describe("identity display", () => {
  const session = {
    callsign: "👩‍💻 Baroness Byte",
    name: "provider-name",
    session_id: "019fcbf9-d75c-7ba3-a481-18068ea954eb",
  };

  test("prefers callsign, task label, provider name, then short ID", () => {
    expect(sessionDisplayName(session)).toBe("👩‍💻 Baroness Byte");
    expect(
      sessionDisplayName({ ...session, callsign: null }, "dashboard work"),
    ).toBe("dashboard work");
    expect(sessionDisplayName({ ...session, callsign: null })).toBe(
      "provider-name",
    );
    expect(
      sessionDisplayName({
        ...session,
        callsign: undefined,
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

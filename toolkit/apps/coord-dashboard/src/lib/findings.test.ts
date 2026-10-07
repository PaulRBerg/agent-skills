import { describe, expect, test } from "vitest";

import { countFindings, filterFindings, groupFindings } from "@/lib/findings.js";
import { sampleSnapshot } from "@/lib/sample-snapshot.js";
import type { Finding, FindingState } from "@/lib/types.js";

function finding(overrides: Partial<Finding> = {}): Finding {
  const sample = sampleSnapshot.findings[0];
  if (!sample) {
    throw new Error("The sample snapshot must include a finding");
  }
  return {
    ...sample,
    triaging: false,
    ...overrides,
  };
}

describe("findings", () => {
  test("keeps triaging as an independent live lease overlay", () => {
    expect(countFindings(sampleSnapshot.findings)).toEqual({
      total: 3,
      unresolved: 2,
      pending: 1,
      triaging: 1,
      handedOff: 1,
      terminal: 1,
    });
  });

  test("filters open, handoff, and terminal findings in recency order", () => {
    expect(filterFindings(sampleSnapshot.findings, "open").map(({ id }) => id)).toEqual([
      "5defa09e",
    ]);
    expect(filterFindings(sampleSnapshot.findings, "handoff").map(({ id }) => id)).toEqual([
      "4f7d2b11",
    ]);
    expect(filterFindings(sampleSnapshot.findings, "resolved").map(({ id }) => id)).toEqual([
      "5d8caf48",
    ]);
  });

  test("groups repositories with durable counts and newest finding first", () => {
    const groups = groupFindings(sampleSnapshot.findings);

    expect(groups.map(({ repoRoot }) => repoRoot)).toEqual([
      "/Users/prb/projects/agent-toolkit",
      "/Users/prb/projects/agent-skills",
    ]);
    expect(groups[0]?.counts).toEqual({
      total: 2,
      unresolved: 2,
      pending: 1,
      triaging: 1,
      handedOff: 1,
      terminal: 0,
    });
    expect(groups[0]?.findings.map(({ id }) => id)).toEqual(["5defa09e", "4f7d2b11"]);
  });

  test("includes pending and handed-off findings in unresolved without counting triaging twice", () => {
    const findings = [
      finding({ id: "pending", state: "pending", triaging: true, updated_at: 2 }),
      finding({ id: "handoff", state: "handed-off", triaging: true, updated_at: 3 }),
      ...(["fixed", "stale", "rejected", "duplicate"] as FindingState[]).map((state) =>
        finding({ id: state, state, updated_at: 4 })
      ),
    ];

    expect(countFindings(findings)).toEqual({
      total: 6,
      unresolved: 2,
      pending: 1,
      triaging: 2,
      handedOff: 1,
      terminal: 4,
    });
    expect(filterFindings(findings, "unresolved").map(({ id }) => id)).toEqual([
      "handoff",
      "pending",
    ]);
    expect(filterFindings(findings, "resolved")).toHaveLength(4);
    expect(countFindings([]).unresolved).toBe(0);
  });

  test("keeps all matching history accessible and orders equal timestamps by ID", () => {
    const findings = ["d", "b", "e", "a", "c"].map((id) => finding({ id, updated_at: 1 }));
    const originalOrder = findings.map(({ id }) => id);

    expect(filterFindings(findings, "all").map(({ id }) => id)).toEqual(["a", "b", "c", "d", "e"]);
    expect(groupFindings(findings)[0]?.findings).toHaveLength(5);
    expect(findings.map(({ id }) => id)).toEqual(originalOrder);
  });

  test("omits repositories with no filter matches while preserving full repository counts", () => {
    const groups = groupFindings(sampleSnapshot.findings, "handoff");

    expect(groups).toHaveLength(1);
    expect(groups[0]?.findings.map(({ id }) => id)).toEqual(["4f7d2b11"]);
    expect(groups[0]?.counts).toMatchObject({ total: 2, unresolved: 2, handedOff: 1 });
    expect(groupFindings([finding({ state: "fixed" })], "unresolved")).toEqual([]);
  });

  test("puts handed-off-only repositories ahead of terminal-only repositories", () => {
    const groups = groupFindings([
      finding({ repo_root: "/repos/a", state: "fixed" }),
      finding({ repo_root: "/repos/b", state: "handed-off" }),
    ]);

    expect(groups.map(({ repoRoot }) => repoRoot)).toEqual(["/repos/b", "/repos/a"]);
  });
});

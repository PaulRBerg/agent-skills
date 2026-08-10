import { describe, expect, test } from "vitest";
import { countFindings, filterFindings, groupFindings } from "@/lib/findings";
import { sampleSnapshot } from "@/lib/sample-snapshot";

describe("findings", () => {
  test("keeps triaging as an independent live lease overlay", () => {
    expect(countFindings(sampleSnapshot.findings)).toEqual({
      pending: 1,
      triaging: 1,
      handedOff: 1,
      terminal: 1,
    });
  });

  test("filters open, handoff, and terminal findings in recency order", () => {
    expect(
      filterFindings(sampleSnapshot.findings, "open").map(({ id }) => id),
    ).toEqual(["5defa09e"]);
    expect(
      filterFindings(sampleSnapshot.findings, "handoff").map(({ id }) => id),
    ).toEqual(["4f7d2b11"]);
    expect(
      filterFindings(sampleSnapshot.findings, "resolved").map(({ id }) => id),
    ).toEqual(["5d8caf48"]);
  });

  test("groups repositories with durable counts and newest finding first", () => {
    const groups = groupFindings(sampleSnapshot.findings);

    expect(groups.map(({ repoRoot }) => repoRoot)).toEqual([
      "/Users/prb/projects/ai-coord",
      "/Users/prb/projects/agent-skills",
    ]);
    expect(groups[0]?.counts).toEqual({
      pending: 1,
      triaging: 1,
      handedOff: 1,
      terminal: 0,
    });
    expect(groups[0]?.findings.map(({ id }) => id)).toEqual([
      "5defa09e",
      "4f7d2b11",
    ]);
  });
});

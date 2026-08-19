import { describe, expect, test } from "vitest";
import { groupSnapshotByRepo } from "@/lib/group";
import { sampleSnapshot } from "@/lib/sample-snapshot";

const toolkitRoot = "/Users/prb/projects/agent-toolkit";
const skillsRoot = "/Users/prb/projects/agent-skills";

describe("groupSnapshotByRepo", () => {
  test("creates lanes for session and claim roots and sorts them by recent activity", () => {
    const lanes = groupSnapshotByRepo(sampleSnapshot);

    expect(lanes.map((lane) => lane.repoRoot)).toEqual([
      toolkitRoot,
      skillsRoot,
    ]);
    expect(lanes[0]?.sessions).toHaveLength(4);
    expect(lanes[1]?.sessions).toHaveLength(1);
  });

  test("assigns queue positions separately for every queued claim repository", () => {
    const lanes = groupSnapshotByRepo(sampleSnapshot);
    const docs = lanes[0]?.sessions.find(
      ({ work }) => work?.label === "docs-followup",
    )?.work;
    const serveApi = lanes[0]?.sessions.find(
      ({ work }) => work?.label === "serve-api",
    )?.work;

    expect(serveApi?.claims.map((claim) => claim.queuePosition)).toEqual([1]);
    expect(docs?.claims.map((claim) => [claim.repo_root, claim.queuePosition])).toEqual([
      [toolkitRoot, 2],
      [skillsRoot, 1],
    ]);
  });

  test("homes a multi-claim task once in its live session repository", () => {
    const lanes = groupSnapshotByRepo(sampleSnapshot);
    const cards = lanes.flatMap((lane) => [
      ...lane.sessions.flatMap((row) => (row.work ? [row.work] : [])),
      ...lane.unmatchedWork,
    ]);

    expect(cards.map((work) => work.id).sort((left, right) => left - right)).toEqual([
      640, 644, 645, 646, 647,
    ]);
    expect(
      lanes[1]?.sessions.some(
        ({ work }) => work?.label === "monorepo-dashboard-orchestrator",
      ),
    ).toBe(false);
  });

  test("keeps delegates only beside their live parent session", () => {
    const lane = groupSnapshotByRepo(sampleSnapshot)[0];
    const parent = lane?.sessions.find(
      ({ session }) =>
        session.session_id === "7ca88f40-3aed-4f2d-be71-a80e544dd332",
    );

    expect(parent?.delegates.map((delegate) => delegate.agent_id)).toEqual([
      "a3-dashboard-implementation",
      "a2-serve-api",
    ]);
  });

  test("homes unmatched multi-claim work in its lexicographically first claim root", () => {
    const orphanedSnapshot = {
      ...sampleSnapshot,
      sessions: sampleSnapshot.sessions.filter(
        (session) =>
          session.session_id !== "7ca88f40-3aed-4f2d-be71-a80e544dd332",
      ),
    };
    const lanes = groupSnapshotByRepo(orphanedSnapshot);

    expect(
      lanes.find((lane) => lane.repoRoot === skillsRoot)?.unmatchedWork.map(
        (work) => work.label,
      ),
    ).toContain(
      "monorepo-dashboard-orchestrator",
    );
    expect(
      lanes.find((lane) => lane.repoRoot === toolkitRoot)?.unmatchedWork,
    ).toEqual([]);
  });
});

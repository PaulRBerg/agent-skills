import { describe, expect, test } from "vitest";
import { groupSnapshotByRepo } from "@/lib/group";
import { sampleSnapshot } from "@/lib/sample-snapshot";

describe("groupSnapshotByRepo", () => {
  test("groups sessions by repository and sorts lanes by most recent activity", () => {
    const lanes = groupSnapshotByRepo(sampleSnapshot);

    expect(lanes.map((lane) => lane.repoRoot)).toEqual([
      "/Users/prb/projects/ai-coord",
      "/Users/prb/projects/agent-skills",
    ]);
    expect(lanes[0]?.sessions).toHaveLength(4);
    expect(lanes[1]?.sessions).toHaveLength(1);
  });

  test("assigns FIFO positions to queued work within a repository", () => {
    const lane = groupSnapshotByRepo(sampleSnapshot)[0];
    const queued = lane?.sessions
      .map((row) => row.work)
      .filter((work) => work?.state === "queued")
      .sort(
        (left, right) =>
          (left?.queuePosition ?? 0) - (right?.queuePosition ?? 0),
      );

    expect(queued?.map((work) => [work?.label, work?.queuePosition])).toEqual([
      ["serve-api", 1],
      ["docs-followup", 2],
    ]);
  });

  test("nests delegates under their parent session in latest-first order", () => {
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

  test("retains work whose session is absent from provider inventory", () => {
    const orphanedSnapshot = {
      ...sampleSnapshot,
      sessions: sampleSnapshot.sessions.filter(
        (session) =>
          session.session_id !== "019fcbf1-1a53-7e20-a682-520d66c5b87f",
      ),
    };
    const lane = groupSnapshotByRepo(orphanedSnapshot)[0];

    expect(lane?.unmatchedWork.map((work) => work.label)).toEqual([
      "docs-followup",
    ]);
  });
});

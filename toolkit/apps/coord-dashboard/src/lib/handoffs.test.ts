import { describe, expect, it } from "vitest";

import type { HandoffRecord } from "@/lib/handoff-types.js";
import { formatHandoffDate, groupByProvenance, stripDuplicateTitle } from "@/lib/handoffs.js";

function record(root: string, repository: string, filename: string): HandoffRecord {
  const path = `${root}/${repository}/.ai/task-handoffs/${filename}`;
  return {
    id: path,
    state: "live",
    root,
    repository,
    filename,
    path,
    format: "legacy",
    title: filename,
    category: null,
    created: null,
    modifiedAt: "2026-08-10T08:00:00.000Z",
    frontmatter: null,
    markdown: "",
  };
}

describe("groupByProvenance", () => {
  it("groups by root and repository in first-seen order", () => {
    const groups = groupByProvenance([
      record("/home/projects", "alpha", "A.md"),
      record("/home/work", "alpha", "B.md"),
      record("/home/projects", "alpha", "C.md"),
    ]);

    expect(groups.map((group) => [group.root, group.handoffs.map((h) => h.filename)])).toEqual([
      ["/home/projects", ["A.md", "C.md"]],
      ["/home/work", ["B.md"]],
    ]);
  });
});

describe("formatHandoffDate", () => {
  const now = new Date(2026, 9, 7);

  it("omits the current year unless forced", () => {
    expect(formatHandoffDate(new Date(2026, 8, 21).toISOString(), false, now)).toBe("21 Sep");
    expect(formatHandoffDate(new Date(2026, 8, 21).toISOString(), true, now)).toBe("21 Sep 2026");
    expect(formatHandoffDate(new Date(2025, 0, 2).toISOString(), false, now)).toBe("2 Jan 2025");
  });

  it("returns unparseable input unchanged", () => {
    expect(formatHandoffDate("not a date", false, now)).toBe("not a date");
  });
});

describe("stripDuplicateTitle", () => {
  it("drops a leading heading that repeats the title and its blank line", () => {
    expect(stripDuplicateTitle("\n# Ship it\n\nBody\n", "Ship it")).toBe("Body\n");
  });

  it("keeps a heading that differs from the title", () => {
    expect(stripDuplicateTitle("# Other\n\nBody", "Ship it")).toBe("# Other\n\nBody");
  });
});

import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import { parseHandoff } from "./parser";

const VALID_FRONTMATTER = `---
category: implementation
created: 2026-08-10T08:00:00Z
launch_repo: /Users/example/projects/app
repos:
  - /Users/example/projects/app
origin: /Users/example/projects/app/.ai/task-handoffs/VIEWER.md
task: Build the handoff viewer
---
# Body heading

Body text.
`;

const TASK_HANDOFF_V1 = readFileSync(
  new URL("./fixtures/task-handoff-v1.md", import.meta.url),
  "utf8",
);
const TASK_HANDOFF_V2 = readFileSync(
  new URL("./fixtures/task-handoff-v2.md", import.meta.url),
  "utf8",
);

describe("parseHandoff", () => {
  it("parses the complete task-handoff producer fixture", () => {
    const parsed = parseHandoff(TASK_HANDOFF_V1, "TASK_HANDOFF_COMPATIBILITY.md");

    expect(parsed).toMatchObject({
      format: "frontmatter",
      title: "Validate task handoff compatibility",
      category: "implementation",
      created: "2026-08-10T08:00:00Z",
      frontmatter: {
        category: "implementation",
        created: "2026-08-10T08:00:00Z",
        launch_repo: "/Users/example/projects/app",
        repos: ["/Users/example/projects/app"],
        origin: "/Users/example/projects/app/.ai/task-handoffs/TASK_HANDOFF_COMPATIBILITY.md",
        task: "Validate task handoff compatibility",
      },
    });
    expect(parsed.markdown).toContain("# Validate task handoff compatibility");
    expect(parsed.markdown).toContain("## Handoff category\n\nCategory: `implementation`");
    expect(parsed.markdown).toContain("## Execution status");
    expect(parsed.markdown).toContain("## Handoff cleanup");
    expect(parsed.markdown).toContain(
      "handoff='/Users/example/projects/app/.ai/task-handoffs/TASK_HANDOFF_COMPATIBILITY.md'",
    );
  });

  it("parses the complete task-handoff v2 producer fixture", () => {
    const parsed = parseHandoff(TASK_HANDOFF_V2, "TASK_HANDOFF_COMPATIBILITY_V2.md");

    expect(parsed).toMatchObject({
      format: "frontmatter",
      title: "Validate task handoff compatibility",
      category: "implementation",
      created: "2026-08-10T08:00:00Z",
      frontmatter: {
        category: "implementation",
        created: "2026-08-10T08:00:00Z",
        launch_repo: "/Users/example/projects/app",
        repos: ["/Users/example/projects/app"],
        origin: "/Users/example/projects/app/.ai/task-handoffs/TASK_HANDOFF_COMPATIBILITY_V2.md",
        task: "Validate task handoff compatibility",
      },
    });
    expect(parsed.markdown).toContain("# Validate task handoff compatibility");
    expect(parsed.markdown).toContain("## Handoff category\n\nCategory: `implementation`");
    expect(parsed.markdown).toContain("## Execution status");
    expect(parsed.markdown).toContain("## Handoff cleanup");
    expect(parsed.markdown).toContain(
      "ai-handoff archive '/Users/example/projects/app/.ai/task-handoffs/TASK_HANDOFF_COMPATIBILITY_V2.md'",
    );
  });

  it("validates and removes a complete leading frontmatter block", () => {
    expect(parseHandoff(VALID_FRONTMATTER, "VIEWER.md")).toEqual({
      format: "frontmatter",
      title: "Build the handoff viewer",
      category: "implementation",
      created: "2026-08-10T08:00:00Z",
      frontmatter: {
        category: "implementation",
        created: "2026-08-10T08:00:00Z",
        launch_repo: "/Users/example/projects/app",
        repos: ["/Users/example/projects/app"],
        origin: "/Users/example/projects/app/.ai/task-handoffs/VIEWER.md",
        task: "Build the handoff viewer",
      },
      markdown: "# Body heading\n\nBody text.\n",
    });
  });

  it("degrades a closed invalid block to legacy while rendering only its body", () => {
    const source = VALID_FRONTMATTER.replace("category: implementation", "category: unknown").replace(
      "# Body heading\n\nBody text.",
      "# Legacy title\n\nBody text.\n\n## Handoff category\n\nCategory: `investigation`",
    );

    expect(parseHandoff(source, "VIEWER.md")).toMatchObject({
      format: "legacy",
      title: "Legacy title",
      category: "investigation",
      created: null,
      frontmatter: null,
      markdown: "# Legacy title\n\nBody text.\n\n## Handoff category\n\nCategory: `investigation`\n",
    });
  });

  it("rejects unknown frontmatter fields", () => {
    const source = VALID_FRONTMATTER.replace(
      "origin: /Users/example/projects/app/.ai/task-handoffs/VIEWER.md",
      "origin: /Users/example/projects/app/.ai/task-handoffs/VIEWER.md\nextra: rejected",
    );
    expect(parseHandoff(source, "VIEWER.md").format).toBe("legacy");
  });

  it("rejects relative repository and origin paths", () => {
    const source = VALID_FRONTMATTER.replace(
      "launch_repo: /Users/example/projects/app",
      "launch_repo: projects/app",
    );
    expect(parseHandoff(source, "VIEWER.md").format).toBe("legacy");
  });

  it("retains all markdown when a leading block is unterminated", () => {
    const source = "---\ncategory: implementation\n# Still visible\n";
    const parsed = parseHandoff(source, "FALLBACK.md");

    expect(parsed.format).toBe("legacy");
    expect(parsed.title).toBe("Still visible");
    expect(parsed.markdown).toBe(source);
  });

  it("falls back to the filename when legacy markdown has no H1", () => {
    expect(parseHandoff("Plain text", "NO_HEADING.md").title).toBe("NO_HEADING");
  });
});

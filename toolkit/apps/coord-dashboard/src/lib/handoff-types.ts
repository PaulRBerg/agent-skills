export const HANDOFF_CATEGORIES = [
  "implementation",
  "investigation",
  "research",
  "audit",
  "operations",
] as const;

export type HandoffCategory = (typeof HANDOFF_CATEGORIES)[number];

export type HandoffFrontmatter = {
  category: HandoffCategory;
  created: string;
  launch_repo: string;
  repos: string[];
  origin: string;
  task: string;
};

export type HandoffRecord = {
  id: string;
  state: "live" | "archived";
  root: string;
  repository: string;
  filename: string;
  path: string;
  format: "frontmatter" | "legacy";
  title: string;
  category: HandoffCategory | null;
  created: string | null;
  modifiedAt: string;
  frontmatter: HandoffFrontmatter | null;
  markdown: string;
};

export type HandoffsResponse = {
  handoffs: HandoffRecord[];
};

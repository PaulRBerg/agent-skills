export const HANDOFF_CATEGORIES = [
  "implementation",
  "investigation",
  "research",
  "audit",
  "operations",
] as const;

export type HandoffCategory = (typeof HANDOFF_CATEGORIES)[number];

export interface HandoffFrontmatter {
  category: HandoffCategory;
  created: string;
  launch_repo: string;
  repos: string[];
  origin: string;
  task: string;
}

export interface HandoffRecord {
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
}

export interface HandoffsResponse {
  handoffs: HandoffRecord[];
}


import { parseDocument } from "yaml";
import { isAbsolute } from "node:path";

import {
  HANDOFF_CATEGORIES,
  type HandoffCategory,
  type HandoffFrontmatter,
} from "../shared/handoff";

export interface ParsedHandoff {
  format: "frontmatter" | "legacy";
  title: string;
  category: HandoffCategory | null;
  created: string | null;
  frontmatter: HandoffFrontmatter | null;
  markdown: string;
}

const FRONTMATTER_KEYS = [
  "category",
  "created",
  "launch_repo",
  "repos",
  "origin",
  "task",
] as const;

const CATEGORY_SET = new Set<string>(HANDOFF_CATEGORIES);
const H1_PATTERN = /^#\s+(.+?)\s*$/m;
const CATEGORY_FOOTER_PATTERN =
  /^##[ \t]+Handoff category[ \t]*\r?\n(?:[ \t]*\r?\n)*[ \t]*(?:Category:[ \t]*)?`?(implementation|investigation|research|audit|operations)`?[ \t]*$/m;

function isNonemptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function isIsoDate(value: string): boolean {
  return (
    /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/.test(value) &&
    !Number.isNaN(Date.parse(value))
  );
}

function isAbsolutePath(value: unknown): value is string {
  return isNonemptyString(value) && isAbsolute(value);
}

function validateFrontmatter(value: unknown): HandoffFrontmatter | null {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }

  const record = value as Record<string, unknown>;
  const keys = Object.keys(record).sort();
  const expectedKeys = [...FRONTMATTER_KEYS].sort();
  if (keys.length !== expectedKeys.length || keys.some((key, index) => key !== expectedKeys[index])) {
    return null;
  }

  if (
    !isNonemptyString(record.category) ||
    !CATEGORY_SET.has(record.category) ||
    !isNonemptyString(record.created) ||
    !isIsoDate(record.created) ||
    !isAbsolutePath(record.launch_repo) ||
    !Array.isArray(record.repos) ||
    record.repos.length === 0 ||
    !record.repos.every(isAbsolutePath) ||
    !isAbsolutePath(record.origin) ||
    !isNonemptyString(record.task)
  ) {
    return null;
  }

  return {
    category: record.category as HandoffCategory,
    created: record.created,
    launch_repo: record.launch_repo,
    repos: [...record.repos],
    origin: record.origin,
    task: record.task,
  };
}

function legacyTitle(markdown: string, filename: string): string {
  return H1_PATTERN.exec(markdown)?.[1]?.trim() || filename.replace(/\.md$/i, "");
}

function legacyCategory(markdown: string): HandoffCategory | null {
  return (CATEGORY_FOOTER_PATTERN.exec(markdown)?.[1] as HandoffCategory | undefined) ?? null;
}

function parseLegacy(markdown: string, filename: string): ParsedHandoff {
  return {
    format: "legacy",
    title: legacyTitle(markdown, filename),
    category: legacyCategory(markdown),
    created: null,
    frontmatter: null,
    markdown,
  };
}

function splitLeadingFrontmatter(source: string): { yaml: string; body: string } | null | undefined {
  const opening = /^---[ \t]*\r?\n/.exec(source);
  if (!opening) return undefined;

  const rest = source.slice(opening[0].length);
  const closing = /^---[ \t]*(?:\r?\n|$)/m.exec(rest);
  if (!closing || closing.index === undefined) return null;

  return {
    yaml: rest.slice(0, closing.index),
    body: rest.slice(closing.index + closing[0].length),
  };
}

export function parseHandoff(source: string, filename: string): ParsedHandoff {
  const block = splitLeadingFrontmatter(source);
  if (block === undefined || block === null) {
    return parseLegacy(source, filename);
  }

  try {
    const document = parseDocument(block.yaml, { uniqueKeys: true });
    if (document.errors.length > 0) {
      return parseLegacy(block.body, filename);
    }

    const frontmatter = validateFrontmatter(document.toJS());
    if (!frontmatter) {
      return parseLegacy(block.body, filename);
    }

    return {
      format: "frontmatter",
      title: frontmatter.task,
      category: frontmatter.category,
      created: frontmatter.created,
      frontmatter,
      markdown: block.body,
    };
  } catch {
    return parseLegacy(block.body, filename);
  }
}

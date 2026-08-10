#!/usr/bin/env bun

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

try {
  const [readmeArg, skillsRootArg] = process.argv.slice(2);
  const readmePath = path.resolve(readmeArg ?? path.join(scriptRoot, "README.md"));
  const skillsRoot = path.resolve(skillsRootArg ?? path.join(scriptRoot, "skills"));

  const readmeNames = readReadmeSkillNames(readmePath);
  const skillNames = readSkillDirectoryNames(skillsRoot);

  const missingFromReadme = [...skillNames].filter((name) => !readmeNames.has(name)).sort();
  const extraInReadme = [...readmeNames].filter((name) => !skillNames.has(name)).sort();

  if (missingFromReadme.length === 0 && extraInReadme.length === 0) {
    console.log(`README skills tables match skills/ (${skillNames.size} skills).`);
    process.exitCode = 0;
  } else {
    if (missingFromReadme.length > 0) {
      console.log("missing from README:");
      for (const name of missingFromReadme) console.log(`  ${name}`);
    }
    if (extraInReadme.length > 0) {
      console.log("in README but not in skills/:");
      for (const name of extraInReadme) console.log(`  ${name}`);
    }
    process.exitCode = 1;
  }
} catch (error) {
  console.error(`Error: ${errorMessage(error)}`);
  process.exitCode = 2;
}

function readReadmeSkillNames(readmePath: string): Set<string> {
  let text: string;
  try {
    text = fs.readFileSync(readmePath, "utf8");
  } catch (error) {
    throw new Error(`Cannot read README at ${readmePath}: ${errorMessage(error)}`);
  }

  const names = new Set<string>();
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed.startsWith("|")) continue;
    const cells = trimmed.split("|");
    const first = cells[1]?.trim();
    if (!first) continue;
    if (/^:?-+:?$/.test(first)) continue; // separator row
    const bare = stripMarkup(first);
    if (bare === "Skill") continue; // header row
    if (/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(bare)) names.add(bare);
  }
  return names;
}

function stripMarkup(cell: string): string {
  const inner = cell.match(/^\[([^\]]+)\]\(/)?.[1] ?? cell;
  return inner.replace(/`/g, "").trim();
}

function readSkillDirectoryNames(skillsRoot: string): Set<string> {
  let entries;
  try {
    entries = fs.readdirSync(skillsRoot, { withFileTypes: true });
  } catch (error) {
    throw new Error(`Cannot read skills root at ${skillsRoot}: ${errorMessage(error)}`);
  }

  const names = new Set<string>();
  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    if (fs.existsSync(path.join(skillsRoot, entry.name, "SKILL.md"))) names.add(entry.name);
  }
  return names;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

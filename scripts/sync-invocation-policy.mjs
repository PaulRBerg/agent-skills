#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const roots = ["skills", "shelved"];
const fix = process.argv.includes("--fix");
const errors = [];
let fixed = 0;

for (const root of roots) {
  if (!fs.existsSync(root)) continue;

  for (const name of fs.readdirSync(root).sort()) {
    const dir = path.join(root, name);
    const skillPath = path.join(dir, "SKILL.md");
    const openaiPath = path.join(dir, "agents", "openai.yaml");

    if (!fs.existsSync(skillPath) || !fs.existsSync(openaiPath)) continue;

    checkSkill(skillPath, openaiPath);
  }
}

if (errors.length > 0) {
  console.error("Skill invocation metadata mismatch:");
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

if (fixed > 0) {
  console.log(`Updated ${fixed} openai.yaml file${fixed === 1 ? "" : "s"}.`);
} else {
  console.log("Skill invocation metadata consistent.");
}

function checkSkill(skillPath, openaiPath) {
  const frontmatter = readFrontmatter(skillPath);
  if (!frontmatter) {
    errors.push(`${skillPath}: missing YAML frontmatter`);
    return;
  }

  const disableModelInvocation = readOptionalBoolean(
    frontmatter,
    "disable-model-invocation",
    false,
    skillPath,
  );
  const userInvocable = readOptionalBoolean(frontmatter, "user-invocable", true, skillPath);
  if (disableModelInvocation === null || userInvocable === null) return;

  const expectedAllowImplicitInvocation = !disableModelInvocation;
  const openaiText = fs.readFileSync(openaiPath, "utf8");
  const currentAllowImplicitInvocation = readOpenaiPolicy(openaiText, openaiPath);
  if (currentAllowImplicitInvocation === null) return;

  if (currentAllowImplicitInvocation === expectedAllowImplicitInvocation) return;

  if (fix) {
    fs.writeFileSync(
      openaiPath,
      openaiText.replace(
        /^(\s*allow_implicit_invocation:\s*)(true|false)(\s*)$/m,
        `$1${expectedAllowImplicitInvocation}$3`,
      ),
    );
    fixed += 1;
    return;
  }

  errors.push(
    `${openaiPath}: allow_implicit_invocation=${currentAllowImplicitInvocation}, ` +
      `expected ${expectedAllowImplicitInvocation} from ${skillPath} ` +
      `disable-model-invocation=${disableModelInvocation}`,
  );
}

function readFrontmatter(filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  const match = text.match(/^---\n([\s\S]*?)\n---\n/);
  if (!match) return null;

  const fields = new Map();
  for (const line of match[1].split("\n")) {
    const field = line.match(/^([A-Za-z0-9_-]+):\s*(.*)$/);
    if (field) fields.set(field[1], field[2].trim());
  }
  return fields;
}

function readOptionalBoolean(fields, key, defaultValue, filePath) {
  if (!fields.has(key)) return defaultValue;

  const value = fields.get(key);
  if (value === "true") return true;
  if (value === "false") return false;

  errors.push(`${filePath}: ${key} must be true or false, got ${JSON.stringify(value)}`);
  return null;
}

function readOpenaiPolicy(text, filePath) {
  const match = text.match(/^\s*allow_implicit_invocation:\s*(true|false)\s*$/m);
  if (!match) {
    errors.push(`${filePath}: missing policy.allow_implicit_invocation`);
    return null;
  }

  return match[1] === "true";
}

---
argument-hint: '[<slug>]'
disable-model-invocation: false
name: debrief
user-invocable: true
description: This skill should be used when the user asks to "debrief", "debrief this task", "debrief the session", "save findings", "save analysis", "save this as a report", "create an HTML report from the transcript", or wants to persist the current task's findings as a self-contained interactive HTML playground at `./.ai/reports/<slug>/index.html`.
---

# Debrief

Persist the current task's findings as a self-contained, interactive HTML debrief at `./.ai/reports/<slug>/index.html`. Pick a slug from the user's task, build the page using the [`playground`](https://github.com/anthropics/skills/tree/main/playground) skill's conventions, and pre-populate it with concrete findings drawn from the transcript.

## Arguments

- `<slug>` (optional): kebab-case folder name, e.g. `auth-security-review`. If omitted, derive a topical slug from the task — short (3-5 words), lowercase, dash-separated.

## Prerequisites

Depends on the [`playground`](https://github.com/anthropics/skills/tree/main/playground) skill for HTML conventions and templates. `scripts/prepare.sh` performs the check automatically; if missing it prints the install command and exits non-zero. Manual install:

```sh
npx skills add anthropics/skills --skill playground --global
```

## Workflow

### 1. Resolve slug

- Use `$ARGUMENTS` if provided.
- Otherwise pick a topical kebab-case slug from the task: `auth-security-review`, `bundle-size-analysis`, `tailwind-v4-migration`, `lcp-regression-q1`. Avoid generic names (`report`, `findings`, `debrief`, `output`).
- Reject slugs that don't match `^[a-z0-9]([a-z0-9-]*[a-z0-9])?$` — the helper script enforces this.

### 2. Prepare and verify dependencies

Run from the skill directory:

```bash
bash scripts/prepare.sh <slug>
```

The script:

- Validates the slug.
- Probes `.agents/skills/playground`, `.claude/skills/playground`, `~/.agents/skills/playground`, `~/.claude/skills/playground`.
- Exits `2` with the install command if `playground` is missing — relay the message verbatim and stop.
- Creates `./.ai/reports/<slug>/`.
- Prints four `KEY=VALUE` lines on stdout: `PLAYGROUND_DIR`, `REPORTS_DIR`, `REPORT_PATH`, `EXISTS`.

If `EXISTS=true`, ask the user before continuing: overwrite or pick a new slug.

### 3. Pick a playground template

Read `$PLAYGROUND_DIR/SKILL.md`, then load **one** template under `$PLAYGROUND_DIR/templates/` whose shape best fits the debrief:

| Debrief shape                   | Template               |
| ------------------------------- | ---------------------- |
| Code review / audit findings    | `diff-review.md`       |
| Document or spec critique       | `document-critique.md` |
| Architecture / codebase tour    | `code-map.md`          |
| Learning, scope, knowledge gaps | `concept-map.md`       |
| Data / query exploration        | `data-explorer.md`     |
| Visual / design decisions       | `design-playground.md` |

Read only the chosen template — don't load all six. If nothing fits cleanly, pick the closest and adapt; do not invent a new template.

### 4. Build the debrief

Write a single HTML file to `$REPORT_PATH` that satisfies playground core requirements:

- Single file. Inline all CSS and JS. No external dependencies, fonts, or CDNs.
- Live preview updating on every control change. No "Apply" button.
- Prompt output at the bottom — natural language, mentions only non-default choices, with a "Copy" button and brief "Copied!" feedback.
- Sensible defaults plus 3-5 named presets that snap controls to a cohesive combination.
- Dark theme. System font for UI, monospace for code/values.
- **Pre-populate with the actual findings from the transcript** — concrete file paths, line numbers, severities, metrics, suggestions, code snippets. Not lorem ipsum. Not placeholders. If the transcript has no findings, ask the user for the source material before writing.

For larger payloads, embed findings as a JS array literal inside one inline `<script>` at the top of the file.

### 5. Open

After writing `$REPORT_PATH`, open it in the user's default browser. Run this unconditionally — do not skip, prompt, or wait for confirmation:

```bash
open "$REPORT_PATH"
```

Then print the absolute `$REPORT_PATH` so the user can locate it.

## Output

A self-contained HTML debrief at `./.ai/reports/<slug>/index.html` that:

- Surfaces concrete findings from the user's task.
- Lets the user explore, filter, or annotate them interactively.
- Generates a copyable prompt to act on the selected items.

## Notes

- Write only under `./.ai/reports/<slug>/`. Never write elsewhere.
- The HTML must be self-contained — no external CSS, JS, fonts, or images.
- Re-run with a different slug to keep parallel debriefs.
- Suggest the user add `.ai/reports/` to `.gitignore` if debriefs shouldn't be committed.

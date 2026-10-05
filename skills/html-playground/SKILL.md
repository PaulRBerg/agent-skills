---
argument-hint: "[debrief [<slug>] | <topic>]"
name: html-playground
description:
  Use to build interactive single-file HTML playgrounds, explorers, or tools with visual controls, live preview, and
  prompt copy-out, or to save the current task's findings as an interactive HTML debrief/report.
---

# HTML Playground Builder

Build a self-contained interactive HTML explorer with controls, live preview, and a copyable natural-language prompt.

Use **debrief mode** when the user asks for a debrief or a saved HTML findings/report from the current task, or invokes
`debrief [<slug>]`. Otherwise, build a general playground.

## Workflow

1. Infer the audience, decisions, and required states. Existing design systems and explicit requirements override these
   defaults.
2. Read exactly one closest template and adapt it:
   - `templates/design-playground.md`
   - `templates/data-explorer.md`
   - `templates/concept-map.md`
   - `templates/document-critique.md`
   - `templates/diff-review.md`
   - `templates/code-map.md`
3. Write one HTML file with inline CSS and JavaScript and no external runtime dependencies.
4. Open it in a desktop browser, interact with every control and preset, and verify live preview, prompt output, copy
   feedback, empty/error states, and keyboard usability. Fix rendered defects before completion.

## Debrief Mode

Persist current-task evidence as an interactive HTML report at `./.ai/debriefs/<slug>/index.html`.

- Derive a short topical kebab-case slug when omitted.
- Write only inside the selected debrief directory. If an inferred slug already exists, choose a collision-free semantic
  qualifier, falling back to a timestamp suffix, and continue without overwriting. If the user explicitly supplied the
  slug, stop and ask whether to overwrite or choose a new slug.
- Use only evidence present in the current task: real findings, paths, verified locations, metrics, decisions, and
  unresolved risks. Do not invent filler.

Before step 1, run the bundled preparer from the target repository:

```sh
bash <skill-dir>/scripts/prepare-debrief.sh <slug>
```

Use its `DEBRIEFS_DIR`, `DEBRIEF_PATH`, and `EXISTS` output. When `EXISTS` is true for an inferred slug, derive a unique
slug and rerun the preparer before writing. When it is true for an explicit slug, request the choice defined above.
Relay slug-validation errors and stop.

Then follow the workflow. Write to `DEBRIEF_PATH`. Use a template matched to the evidence and an evidence view alongside
the controls and presets. Before completion, also verify that every claim traces to the task transcript or tool evidence
and that the output contains no placeholders. Then run `open "$DEBRIEF_PATH"`.

## Opinionated Defaults

Use these when product context does not indicate otherwise:

- controls beside a live preview, with prompt output below.
- a desktop-browser layout. Do not implement or inspect responsive/mobile behavior unless requested.
- a polished light theme, system UI font, monospace code/values, minimal chrome.
- sensible non-empty initial state and 3–5 cohesive named presets.
- one state object, one update path, and immediate preview/prompt refresh.
- controls grouped by concern, with advanced controls collapsed.
- prompt text that explains the desired outcome in natural language and mentions only non-default choices.

## Invariants

- No Apply button: relevant changes render immediately.
- The prompt is actionable without seeing the playground and is not a raw state dump.
- Copy has visible transient feedback and a usable fallback when the Clipboard API fails.
- Standardize copy microcopy as `Copy prompt`, then `Copied`. On failure, show `Copy failed — select the prompt below`.
- Presets update controls, preview, and prompt consistently.
- Do not add controls that do not affect either the preview or the generated prompt.

## Completion

Completion requires the self-contained file and rendered, interactive inspection evidence. Keep generated prompts free
of decorative icons unless the requested content needs them.

- Playground: finish with `### ✨ Playground ready`, the linked artifact, and a compact
  artifact/controls/desktop-browser/copy-fallback table.
- Debrief: also require evidence-grounded content at the selected path and collision handling that never silently
  overwrites. Finish with `### 📊 Debrief ready — <title>`, the clickable absolute path, `Opened in desktop browser`,
  and one compact line naming the evidence view and presets. Keep preparer `KEY=VALUE` output, paths, and slug errors
  exact and undecorated.

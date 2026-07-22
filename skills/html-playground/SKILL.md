---
disable-model-invocation: false
name: html-playground
user-invocable: true
description:
  Use to build interactive single-file HTML playgrounds, explorers, or tools with visual controls, live preview, and
  prompt copy-out.
---

# HTML Playground Builder

Build a self-contained interactive HTML explorer with controls, live preview, and a copyable natural-language prompt.

## Workflow

1. Infer the product context, audience, decisions to explore, and required states. Existing design systems and explicit
   user requirements override this skill's visual defaults.
2. Read exactly one closest template and adapt it:
   - `templates/design-playground.md`
   - `templates/data-explorer.md`
   - `templates/concept-map.md`
   - `templates/document-critique.md`
   - `templates/diff-review.md`
   - `templates/code-map.md`
3. Write one HTML file with inline CSS and JavaScript and no external runtime dependencies.
4. Open it in a browser, interact with every control and preset, inspect representative viewport sizes, and verify live
   preview, prompt output, copy feedback, empty/error states, and keyboard usability. Fix rendered defects before
   completion.

## Opinionated Defaults

Use these when product context does not indicate otherwise:

- controls beside a live preview, with prompt output below;
- a polished light theme, system UI font, monospace code/values, minimal chrome;
- sensible non-empty initial state and 3–5 cohesive named presets;
- one state object, one update path, and immediate preview/prompt refresh;
- controls grouped by concern, with advanced controls collapsed;
- prompt text that explains the desired outcome in natural language and mentions only non-default choices.

## Invariants

- No Apply button: relevant changes render immediately.
- The prompt is actionable without seeing the playground and is not a raw state dump.
- Copy has visible transient feedback and a usable fallback when the Clipboard API fails.
- Standardize copy microcopy as `Copy prompt`, then `Copied`; on failure use `Copy failed — select the prompt below` in
  an `aria-live="polite"` region.
- Presets update controls, preview, and prompt consistently.
- Do not add controls that do not affect either the preview or the generated prompt.

Completion requires the self-contained file plus rendered, interactive inspection evidence; opening the file without
exercising it is insufficient. Finish with `### ✨ Playground ready`, the linked artifact, and a compact
artifact/controls/viewports/copy-fallback table. Keep generated prompts free of decorative icons unless the requested
prompt content itself needs them; report decoration does not authorize ornament in copied output.

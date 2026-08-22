---
name: html-playground
description:
  Use to build interactive single-file HTML playgrounds, explorers, or tools with visual controls, live preview, and
  prompt copy-out.
---

# HTML Playground Builder

Build a self-contained interactive HTML explorer with controls, live preview, and a copyable natural-language prompt.

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

## Opinionated Defaults

Use these when product context does not indicate otherwise:

- controls beside a live preview, with prompt output below;
- a desktop-browser layout; do not implement or inspect responsive/mobile behavior unless requested;
- a polished light theme, system UI font, monospace code/values, minimal chrome;
- sensible non-empty initial state and 3–5 cohesive named presets;
- one state object, one update path, and immediate preview/prompt refresh;
- controls grouped by concern, with advanced controls collapsed;
- prompt text that explains the desired outcome in natural language and mentions only non-default choices.

## Invariants

- No Apply button: relevant changes render immediately.
- The prompt is actionable without seeing the playground and is not a raw state dump.
- Copy has visible transient feedback and a usable fallback when the Clipboard API fails.
- Standardize copy microcopy as `Copy prompt`, then `Copied`; on failure show `Copy failed — select the prompt below`.
- Presets update controls, preview, and prompt consistently.
- Do not add controls that do not affect either the preview or the generated prompt.

Completion requires the self-contained file and rendered, interactive inspection evidence. Finish with
`### ✨ Playground ready`, the linked artifact, and a compact artifact/controls/desktop-browser/copy-fallback table.
Keep generated prompts free of decorative icons unless the requested content needs them.

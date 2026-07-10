---
argument-hint: "[<slug>] [--md]"
disable-model-invocation: false
name: debrief
user-invocable: true
description: "Use for debriefs or saved findings/reports from the current task."
---

# Debrief

Persist evidence from the current task as an opinionated interactive HTML report, or as terse Markdown with `--md`.

## Output and Authority

- Default: `./.ai/debriefs/<slug>/index.html` using the installed `playground` skill.
- `--md`: `./.ai/debriefs/<slug>/index.md`, with no playground dependency.
- Derive a short topical kebab-case slug when omitted.
- Write only inside the selected debrief directory. If the output exists, stop and ask whether to overwrite or choose a
  new slug.
- Use only evidence present in the current task: real findings, paths, verified locations, metrics, decisions, and
  unresolved risks. Do not invent filler.

## Workflow

1. Run the bundled preparer from the target repository:

   ```sh
   bash <skill-dir>/scripts/prepare.sh [--md] <slug>
   ```

   Use its `MODE`, `PLAYGROUND_DIR`, `DEBRIEF_PATH`, and `EXISTS` output. Relay dependency or slug errors and stop.

2. For HTML, read the resolved playground `SKILL.md` and exactly one matching template: diff review, document critique,
   code map, concept map, data explorer, or design playground. Adapt its interaction model to the evidence; do not load
   every template.

3. Build one self-contained file:

   - HTML keeps the playground's live controls, useful presets, evidence view, natural-language prompt output, and copy
     feedback. No external dependencies.
   - Markdown leads with the takeaway, then concise findings with severity/evidence/suggestion and actionable next
     steps. Use snippets only when exact text matters.

4. Verify that every claim traces to the task transcript or tool evidence, the output contains no placeholders, and the
   file opens/renders. Then run `open "$DEBRIEF_PATH"` and report the absolute path.

Completion requires a non-placeholder debrief at the selected path, evidence-grounded content, successful rendered/file
inspection, and explicit overwrite handling.

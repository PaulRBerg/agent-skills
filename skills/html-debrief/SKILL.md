---
argument-hint: "[<slug>]"
disable-model-invocation: false
name: html-debrief
user-invocable: true
description: "Use for interactive HTML debriefs or saved HTML findings/reports from the current task."
---

# HTML Debrief

Persist evidence from the current task as an opinionated interactive HTML report.

## Output and Authority

- Output: `./.ai/debriefs/<slug>/index.html` using the installed `html-playground` skill.
- Derive a short topical kebab-case slug when omitted.
- Write only inside the selected debrief directory. If the output exists, stop and ask whether to overwrite or choose a
  new slug.
- Use only evidence present in the current task: real findings, paths, verified locations, metrics, decisions, and
  unresolved risks. Do not invent filler.

## Workflow

1. Run the bundled preparer from the target repository:

   ```sh
   bash <skill-dir>/scripts/prepare.sh <slug>
   ```

   Use its `PLAYGROUND_DIR`, `DEBRIEFS_DIR`, `DEBRIEF_PATH`, and `EXISTS` output. Relay dependency or slug errors and
   stop.

2. Read the resolved HTML playground `SKILL.md` and exactly one matching template: diff review, document critique, code
   map, concept map, data explorer, or design playground. Adapt its interaction model to the evidence; do not load every
   template.

3. Build one self-contained HTML file with the playground's live controls, useful presets, evidence view,
   natural-language prompt output, and copy feedback. Use no external dependencies.

4. Verify that every claim traces to the task transcript or tool evidence, the output contains no placeholders, and the
   file opens/renders. Then run `open "$DEBRIEF_PATH"` and finish with `### 📊 Debrief ready — <title>`, the clickable
   absolute path, `Opened in browser`, and one compact line naming the evidence view and presets.

Completion requires a non-placeholder HTML debrief at the selected path, evidence-grounded content, successful
rendered/file inspection, and explicit overwrite handling. Keep preparer `KEY=VALUE` output, dependency commands, paths,
and slug errors exact and undecorated.

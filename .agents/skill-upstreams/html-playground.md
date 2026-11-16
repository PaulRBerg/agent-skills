# html-playground

- **Relationship:** confirmed fork and adaptation of
  [anthropics/claude-plugins-official](https://github.com/anthropics/claude-plugins-official/tree/main/plugins/playground/skills/playground),
  `plugins/playground/skills/playground`.
- **Evidence:**
  [initial import](https://github.com/PaulRBerg/agent-skills/commit/f8a8a2ab10602199729698c797620dd62500a133),
  [attribution](https://github.com/PaulRBerg/agent-skills/commit/5cbcd9c3261e5628b70676734f8f5375824560d9), and
  [local rename](https://github.com/PaulRBerg/agent-skills/commit/97f127fcbccce683554637b495509847f1364a32).
- **Reconciled upstream commit:**
  [`4e459fbe7425a1880142a1b919cd62d719f657f4`](https://github.com/anthropics/claude-plugins-official/tree/4e459fbe7425a1880142a1b919cd62d719f657f4/plugins/playground/skills/playground).
  No previous base was recorded. The scheduled run on 2026-10-15 requested no ref.

## Base Evidence

Upstream commit `4e459fbe7425a1880142a1b919cd62d719f657f4` ("Rename artifact plugin to playground", 2026-01-29) is the
only commit that changes `plugins/playground/skills/playground`. It precedes the local import on 2026-05-26. The
imported files match that snapshot, except for the light-theme edits and Prettier formatting.

On 2026-10-15, the default-branch tip was `8f814f49002ff824e8a9ba43cf96112dc7f5948d`. Every mapped file at that tip had
the same blob ID as at the base. That run found no incoming changes and did not change the skill files.

## Mapping and Local Behavior

Map upstream `SKILL.md` and `templates/*.md` to the same relative paths under `skills/html-playground/`. The initial
local name was `playground`. Local-only files include `agents/openai.yaml` and `scripts/prepare-debrief.sh`.

Preserve the system-matched single-theme default and desktop verification contract. The theme edits are in `SKILL.md`,
`templates/diff-review.md`, and `templates/code-map.md`. The
[debrief integration](https://github.com/PaulRBerg/agent-skills/commit/5b12aadd3aa765aa6e8981bf168526410ab3249d) added
task-evidence requirements, output preparation, slug collision handling, and debrief completion rules. Preserve these
behaviors and the single-file, dependency-free output contract.

Preserve these local changes as part of the local delta:

- `SKILL.md`: local frontmatter, the workflow, opinionated defaults, invariants, and completion sections. These sections
  replace the upstream usage steps, state and prompt code patterns, and common-mistakes list.
- `templates/*.md`: STE prose rewrites and Prettier formatting.

## Intentional Exclusions

Upstream added an Apache-2.0 `LICENSE` to `plugins/playground`, outside the mapped skill path and after the recorded
base. Do not bundle a copy under `skills/html-playground/` and do not map it. PRB decided this on 2026-11-16. Do not
raise it as `needs-decision` again.

## Safe Update Boundary

Use the reconciled upstream commit as the base for a three-way merge of the entrypoint and templates. Retain local
helper behavior and metadata. A new upstream file does not automatically replace a local-only file.

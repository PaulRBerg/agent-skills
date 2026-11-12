# deslop

- **Relationship:** independent synthesis of local skills, with indirect external inspiration through `codebase-design`.
  This is not a whole-skill fork.
- **Local source repository:** [PaulRBerg/agent-skills](https://github.com/PaulRBerg/agent-skills), snapshot
  `1116d4be8f76ab3385738b741f3494813c81011e`.
- **Indirect upstream:**
  [mattpocock/skills](https://github.com/mattpocock/skills/tree/main/skills/engineering/codebase-design),
  `skills/engineering/codebase-design`. The local source records reconciled upstream pin
  `4ae2317ddb22b046e26455e4f9c9b3aaaf62212d`. That pin is not a `deslop` merge base.
- **Article inspiration:** [OpenAI harness engineering](https://openai.com/index/harness-engineering/), through local
  `skills/codebase-design/references/AGENT-LEGIBILITY.md`. Article revision unknown.
- **Last reconciled base:** none. No mechanical merge base exists for `deslop`.

## Concept Mapping

These mappings record concepts, not files eligible for content merges. The local source paths below are relative to the
source repository at the snapshot above.

| Local source                                            | Destination in `skills/deslop/SKILL.md`                    |
| ------------------------------------------------------- | ---------------------------------------------------------- |
| `skills/agents-system/SKILL.md`                         | System map, task loops, intuitive/ergonomic/accretive lens |
| `skills/naming-refactor/SKILL.md`                       | Canonical terms, reference accounting, external contracts  |
| `skills/large-file-refactor/SKILL.md`                   | Cohesion-based structure decisions instead of size targets |
| `skills/codebase-design/SKILL.md`                       | Deletion test, small interfaces, invariants, locality      |
| `skills/codebase-design/references/AGENT-LEGIBILITY.md` | Feedback, queryable state, proportionate invariant checks  |
| `skills/code-polish/SKILL.md`                           | Behavior preservation and scoped verification              |
| `skills/fresh-eyes-sweep/SKILL.md`                      | Delete before simplifying, consumer evidence, tests        |
| `skills/agents-brain/SKILL.md`                          | Authority, load scope, durable knowledge, self-containment |

## Local Behavior and Safe Update Boundary

Preserve `deslop`'s cleanup authority, selected scope, evidence requirements, and behavior-preservation contract. Keep
its heuristics inline and independent of companion skill installations. Preserve documentation-only and read-only
branches, proportional checks, and existing approval boundaries.

Review future source changes for useful guidance only. Do not merge content automatically from any source or infer a
whole-skill baseline from the recorded pins. Scheduling, per-worktree machinery, and universal architecture
prescriptions were not imported.

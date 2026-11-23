# agents-brain

**Relationship:** two component adaptations. The whole skill is not an established external fork.

## Writing Great Skills Component

- **Source:**
  [mattpocock/skills](https://github.com/mattpocock/skills/tree/697d4ce9742da558fd1ba6697c8e9775e2e302dd/skills/productivity/writing-great-skills),
  `skills/productivity/writing-great-skills/SKILL.md` and `GLOSSARY.md`.
- **Evidence:**
  [introduced under create-skill](https://github.com/PaulRBerg/agent-skills/commit/e8455e33433c4f3e63737aa005ef1848a5eefb5e),
  then
  [merged into agents-brain](https://github.com/PaulRBerg/agent-skills/commit/0e89cedb6692523e33107afc568a97ea334ebd33).
- **Mapping:** both source documents contribute to `skills/agents-brain/references/writing-great-skills.md`.
- **Historical import revision:** unknown. The linked pre-rename revision is retrievable evidence, not a proven local
  import base.
- **Successor:** the
  [2026-07-23 restructure](https://github.com/mattpocock/skills/commit/1fc6573e0e300118ce342fb9365521c9c34eefd4) moved
  the source to
  [writing-for-agents](https://github.com/mattpocock/skills/tree/main/skills/productivity/writing-for-agents). It merged
  `GLOSSARY.md` into `SKILL.md` and split skill-specific mechanics into `SKILL-MECHANICS.md`. The original path no
  longer resolves at the current default-branch tip.
- **Current source mapping:** `skills/productivity/writing-for-agents/SKILL.md` and `SKILL-MECHANICS.md` contribute
  selectively to `skills/agents-brain/references/writing-great-skills.md`.
- **New reconciliation baseline:** `5beb35b040e3de97e54fa9e386d7e42d15c295c2`, explicitly approved by the user on
  2026-10-24 for a selective two-way reconciliation. This baseline does not recover or identify the historical import
  revision. Future reviews compare both mapped successor files against this revision.

### Reconciliation Decisions at the New Baseline

| Incoming concept or change                                                                                        | Local disposition                                                                                                                                                                                            |
| ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Generalize skill writing to all agent-facing documents, rename the skill, merge the glossary, and split mechanics | Retain the local reference structure and skill-focused scope. Track both successor files without importing their packaging or replacing local workflows.                                                     |
| Predictability as the same process on every run                                                                   | Retain observable outcomes and invariants. The local contract permits different valid execution paths.                                                                                                       |
| Context pointers and concise branch triggers                                                                      | Already covered by conditional routing, one trigger per branch, and removing repeated identity. Retain the local wording.                                                                                    |
| Context load and cognitive load                                                                                   | Already covered. Retain the local invocation tradeoff and its host-aware behavior.                                                                                                                           |
| Information hierarchy, progressive disclosure, co-location, and sprawl                                            | Already covered by representation choices, branch-based disclosure, direct links, and size guidance. Retain local self-containment requirements.                                                             |
| Clear, demanding completion criteria and splitting sequences only across real context boundaries                  | Already covered by the observable contract and reliable-execution sections. Retain the local ordering and criteria.                                                                                          |
| Leading words and their proposed behavioral effects                                                               | Exclude the removed leading-word material. Retain familiar domain terms without adopting the upstream theory or examples.                                                                                    |
| Positive instructions and negation                                                                                | Already covered by positive acceptance criteria and explicit exceptions for consequential boundaries. Retain those exceptions.                                                                               |
| The environment as a source of truth and documentation as a maintained copy                                       | Adopt narrow guidance to read cheap file or command lookups directly. Document conventions, reasons, and pitfalls unavailable from those lookups.                                                            |
| Single source of truth, relevance, and sentence-level pruning                                                     | Already covered by authoritative placement and behavior-based context review. Retain local wording and representative-run checks.                                                                            |
| Failure-mode taxonomy, sediment, and stronger leading words                                                       | Exclude the removed taxonomy and framing. Preserve the existing compact reference.                                                                                                                           |
| Invocation mechanics, shared references, splitting by invocation, and router skills                               | Preserve local host-aware invocation rules and independent skill self-containment. Do not import claims that only a human can reach user-invoked instructions or that skills must share external references. |
| Upstream frontmatter and metadata                                                                                 | Outside the component mapping. Preserve the local `ai-skillet doctor` field contract and existing entrypoint metadata.                                                                                       |
| Punctuation and presentation changes                                                                              | Apply routine local formatting to adopted prose. Do not copy upstream punctuation or reorganize unaffected prose.                                                                                            |

## STE Authoring Component

- **Source:**
  [danyuchn/asd-ste100-skill v0.4.0](https://github.com/danyuchn/asd-ste100-skill/tree/32511c6992ecb5f1971e46a2943f2e6adceedafe),
  `SKILL.md` and `references/writing-rules.md`.
- **Evidence:**
  [local introduction](https://github.com/PaulRBerg/agent-skills/commit/aec720494c7bc56dbd7b837033ea13b21bab6526). The
  local reference retains source attribution and the MIT notice.
- **Mapping:** both source files contribute to `skills/agents-brain/references/asd-ste100.md`.
- **Known source pin:** `32511c6992ecb5f1971e46a2943f2e6adceedafe`. This identifies the adapted component source, not a
  whole-skill rebase base.
- **Last reviewed target:** `65c8ab6ef5fcb3619c8e6daca43e68627f3d7b4a` on 2026-11-23. Previous review:
  `cd62d297d5ef7cf9f59718cadc233ab2876ea013`. The mapped `references/writing-rules.md` is unchanged from the known
  source pin. The mapped `SKILL.md` has three changes. All three are outside the local adaptation, so the local
  reference did not change.

| Incoming change in `SKILL.md`                                                | Local disposition                                                                                         |
| ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Move `version` under `metadata` in the frontmatter                           | Exclude. Upstream frontmatter is outside the component mapping.                                           |
| State that `scripts/ste-lint.py` has no STE-flavored mode                    | Exclude. The local profile has no mode selection and does not bundle the linter.                          |
| Describe `examples/before-after.md` as rule illustrations and agent examples | Exclude. The local profile does not bundle that examples file. It keeps its own meaning-preserving table. |

## Local Behavior and Safe Update Boundary

Limit component updates to their mapped references. Preserve protected technical content, obligation strength,
completion review, attribution, and self-contained reference boundaries. Preserve the distinction between an STE-based
profile and official dictionary compliance. Do not replace the `agents-brain` entrypoint or other workflows with either
source skill. The writing component uses the new user-approved baseline above. Its historical import revision remains
unknown. Future updates must preserve the recorded selective decisions and local behavior.

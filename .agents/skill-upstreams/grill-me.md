# grill-me

- **Relationship:** confirmed adaptation from
  [mattpocock/skills](https://github.com/mattpocock/skills/tree/main/skills/productivity/grill-me),
  `skills/productivity/grill-me`.
- **Source confirmation:** the user confirmed `mattpocock/skills` as the source on 2026-10-15. A matching copy in
  another repository does not override the confirmed source.
- **Recovered content base:** `694fa30311e02c2639942308513555e61ee84a6f`, upstream
  `skills/productivity/grill-me/SKILL.md`, blob `bd04394c675ee54173a093c50eb74da01a2940fa`.
- **Last reconciled upstream target:** `4ae2317ddb22b046e26455e4f9c9b3aaaf62212d`, reviewed on 2026-11-02.
  Implementation: `skills/productivity/grilling/SKILL.md`, blob `d2f09a18e6a6c65c55f1271ba08dfeaf06f0bed3`. Alias:
  `skills/productivity/grill-me/SKILL.md`, blob `3947ff9c4ad980d14fc07fccbf659d47c114e81d`.
- **Previous reconciled target:** `5beb35b040e3de97e54fa9e386d7e42d15c295c2`, reviewed on 2026-10-24. Implementation
  blob `df69d9936880cd3314c28858c6c52f48db23b856`.

## Base Recovery Evidence

The [initial local import](https://github.com/PaulRBerg/agent-skills/commit/30be06f8ffb2494a9de8b68aaf789458a722b009)
matches upstream blob `bd04394c675ee54173a093c50eb74da01a2940fa` after whitespace normalization. Only line wrapping
differs. This establishes a content base, not the exact upstream revision originally imported.

Commit `694fa30311e02c2639942308513555e61ee84a6f` contains that blob and is an ancestor of the reviewed target. It is
the immediate parent of `221ffca96736afefdc08ca7cf0b3965e9ea83f41`, which changed `grill-me` into an alias and copied
its implementation to `grilling`. It is therefore the last matching content before that split. The original imported
commit remains unknown.

## Mapping and Local Behavior

The historical mapping is upstream `skills/productivity/grill-me/SKILL.md` to local `skills/grill-me/SKILL.md`. The
successor implementation is upstream
[skills/productivity/grilling/SKILL.md](https://github.com/mattpocock/skills/blob/5beb35b040e3de97e54fa9e386d7e42d15c295c2/skills/productivity/grilling/SKILL.md).
Compare both upstream paths when assessing later changes. Preserve the local standalone `grill-me` entrypoint instead of
copying the alias or requiring a separate `grilling` installation. `skills/grill-me/agents/openai.yaml` is local catalog
metadata. Do not merge the upstream `agents/openai.yaml` files under either path.

Preserve the running decision record, structured question and report formats, explicit accepted unknowns, coordination
exemption, local name and description, and catalog metadata. Preserve user instruction precedence.

## Reconciliation on 2026-10-24

The user explicitly approved upstream question rounds and the final confirmation gate on 2026-10-24. This approval
replaces the earlier advice to retain one-question cadence and omit the gate. The review compared all incoming changes
from the recovered content base to the pinned target, including the successor implementation and alias.

| Incoming change                                   | Decision and local result                                                                                                                                                                                                       |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Design tree and dependent decisions               | Accepted. Map decisions and their prerequisites explicitly. Preserve the local decision record and material design topics.                                                                                                      |
| Whole-frontier question rounds                    | Accepted by explicit user approval. Ask every currently answerable decision in a round. Defer questions that depend on open decisions. Wait for answers, then recompute the frontier. This replaces local one-question cadence. |
| Numbered question cards and horizontal separators | Adapted. Keep local `Decision <N>` headings and `Recommended`, `Why`, `Tradeoff`, and `Question` fields. Add separators between cards. Exclude the exact upstream Q-card and arrow-only presentation.                           |
| Questions where "yes" accepts the recommendation  | Accepted. Word each `Question` so "yes" accepts its `Recommended` choice.                                                                                                                                                       |
| Fact-finding delegation and independent progress  | Adapted. Delegate when useful and available, otherwise investigate directly. Pending facts block only dependent questions. Preserve investigation of discoverable facts instead of asking the user.                             |
| User owns decisions                               | Accepted. Put decisions to the user and wait for answers. Preserve recorded rationale, dependencies, consequences, and accepted unknowns.                                                                                       |
| Final shared-understanding gate                   | Accepted by explicit user approval. Give the local standalone decision report, ask for explicit confirmation, and wait before acting. Corrections reopen affected branches. A report alone does not complete the workflow.      |
| Empty frontier as completion                      | Adapted. Require no material open branches or required investigations, then explicit user confirmation. Preserve explicitly accepted unknowns with an owner and resolution point.                                               |
| Em-dash removal                                   | Excluded from local report templates. Preserve the existing card headings and acknowledgement format.                                                                                                                           |
| Alias, renamed skill, and upstream metadata       | Excluded. Keep the standalone local `grill-me` name, description, coordination exemption, and catalog metadata. Do not copy `disable-model-invocation` or the Skill-tool alias call.                                            |

## Reconciliation on 2026-11-02

The review compared both upstream paths from `5beb35b040e3de97e54fa9e386d7e42d15c295c2` to the pinned target. Only
upstream commit `7739eaed7f8f3915437a8def198a6d0d7206dbc7` changed the successor implementation. The alias and both
upstream `agents/openai.yaml` files did not change.

| Incoming change                                  | Decision and local result                                                                                                                       |
| ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Fact-finding subagents start with fresh contexts | Adapted. Delegated fact-finding goes to a subagent that starts with a fresh context window. Keep the local availability condition and fallback. |

## Safe Update Boundary

Use the last reconciled target above as the comparison base for later updates. Follow the successor implementation and
inspect the alias for mapping changes. Preserve the accepted local adaptations and exclusions. Do not treat a renamed
entrypoint or changed metadata as authority to replace local catalog behavior.

# Skill Provenance

Known external origins researched on 2026-10-15. These records distinguish whole-skill forks from borrowed components,
inferred sources, inspiration, and replacements. They are repository-only evidence, not installed skill content. Keep
only active skill records on `main`. Move each shelved skill's record and index entry to the `shelved` branch.

| Skill                                                   | Relationship                                    | Update boundary          |
| ------------------------------------------------------- | ----------------------------------------------- | ------------------------ |
| [codebase-design](codebase-design.md)                   | Confirmed fork and adaptation                   | Mapped skill files       |
| [deslop](deslop.md)                                     | Independent synthesis with indirect inspiration | Guidance review only     |
| [apple-device-integration](apple-device-integration.md) | Confirmed fork of Apple's Xcode export          | Mapped skill entrypoint  |
| [html-playground](html-playground.md)                   | Confirmed fork and adaptation                   | Mapped skill files       |
| [retro](retro.md)                                       | Confirmed adaptation                            | Mapped skill entrypoint  |
| [grill-me](grill-me.md)                                 | Confirmed adaptation                            | Mapped skill entrypoint  |
| [agents-brain](agents-brain.md)                         | Two component adaptations                       | Mapped references only   |
| [frontend-design](frontend-design.md)                   | Inspiration followed by independent rewrite     | No mechanical merge base |

Circadian's weekly job discovers the union of the README's [Forked Skills table](../../README.md#forked-skills) and
active `.agents/skill-upstreams/<name>.md` records from fresh `origin/main`. It excludes this index and deduplicates
skill names. Every record above is therefore enrolled, including component adaptations and inspiration. Each record's
update boundary still controls the review or merge. Follow [rebase-skill](../internal-skills/rebase-skill.md) only under
its invocation policy.

When importing or forking a whole external skill, add its table row, provenance record, and index entry in the same
change. Track components and inspiration here without adding them as whole-skill forks. Follow
[skill-lifecycle](../internal-skills/skill-lifecycle.md#external-provenance) to record the required evidence and verify
discovery. No per-skill scheduler list needs an update.

Local import commits establish catalog history. They are not upstream merge bases. Recover an unknown historical base or
obtain explicit approval for a two-way reconciliation before a content merge. No rebase occurred during this research.

## Coverage and Limits

Research covered all 44 active catalog skills, bounded Git addition and attribution history, and current-project
unarchived session history. Targeted searches widened beyond the initial 60-day window. Five relevant session bodies
were inspected. Archived sessions and original precursor-project bodies were not inspected. This records known origins,
not proof that every unlisted skill is original. Normal tool/API documentation and generic prompting guides are not
ancestry evidence.

`brainstorm-with-me` descends from local interview skills rather than a separately established external source. See
[the local branch](https://github.com/PaulRBerg/agent-skills/commit/e29d5ef117a2e5c483367204c328350b17b41c52),
[interview work](https://github.com/PaulRBerg/agent-skills/commit/2e11504f2b616ae4413191957521c9f5506180cc), and
[rename](https://github.com/PaulRBerg/agent-skills/commit/d93b9e71f36e7f43319479a69a40accb4d186b36). It is not an
additional external rebase target.

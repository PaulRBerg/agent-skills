# codebase-design

- **Relationship:** confirmed fork and adaptation of
  [mattpocock/skills](https://github.com/mattpocock/skills/tree/main/skills/engineering/codebase-design),
  `skills/engineering/codebase-design`.
- **Evidence:** the
  [catalog addition](https://github.com/PaulRBerg/agent-skills/commit/b5ef809e34a398c7528794b4ec8d9e2f35e26b2f)
  explicitly records a fork and adaptation with catalog metadata and references. The local entrypoint retains
  attribution.
- **Reconciled upstream commit:** `4ae2317ddb22b046e26455e4f9c9b3aaaf62212d` (default-branch tip, no requested ref).
- **Previous base:** `49dd158d1076134a641b33efb035946536778336`.
- **Original base:** `c0d69015e0cc8b66715beb3f93f9e53256e20f30`, recovered from upstream history, not from the import
  commit itself.
- **Base evidence:** the catalog addition is dated 2026-08-10. `c0d69015` (2026-08-06) is the last upstream commit to
  the mapped files before that date. After word-level normalization, the imported files differ from `c0d69015` only by
  local adaptations. They contain its harness-neutral "Spawn 3+ sub-agents in parallel." wording, absent from earlier
  revisions. They lack the em-dash removal from `32165827` (2026-08-19).

## Mapping and Local Behavior

Map upstream paths to `skills/codebase-design/` as follows:

| Upstream             | Local                           |
| -------------------- | ------------------------------- |
| `SKILL.md`           | `SKILL.md`                      |
| `DEEPENING.md`       | `references/DEEPENING.md`       |
| `DESIGN-IT-TWICE.md` | `references/DESIGN-IT-TWICE.md` |
| `agents/openai.yaml` | `agents/openai.yaml`            |

`references/AGENT-LEGIBILITY.md` is a local addition. `agents/openai.yaml` is locally maintained catalog metadata. Do
not merge upstream metadata into it.

Preserve these local customizations:

- `SKILL.md`: catalog frontmatter description, prose rewritten under the STE profile, the Deep vs shallow definitions
  that replace upstream diagrams, the Agent legibility section, the Output Contract, `references/` link paths, the
  AGENT-LEGIBILITY entry in Going deeper, and the attribution line.
- `references/DEEPENING.md`: `../SKILL.md` link, STE rewrites, the hedged in-process deepening rule, and the local
  Finding deepening candidates section.
- `references/DESIGN-IT-TWICE.md`: `../SKILL.md` links, the separate fresh-context sentence, and the generic
  target-project domain vocabulary source (`AGENTS.md`, glossary, or docs) instead of a named glossary file.

## Upstream Change Dispositions

Changes from `c0d69015` to `49dd158d`:

- **Accepted, adapted:** em-dash removal (`32165827`). Applied to the local wording with colons, commas, periods, and
  parentheses, never semicolons. Extended to local-only lines in mapped files for consistency.
- **Intentionally excluded:** `CONTEXT.md` to `GLOSSARY.md` rename in `DESIGN-IT-TWICE.md` (`d80fa0f4`). The local
  generic domain vocabulary source already covers it.

Changes from `49dd158d` to `4ae2317d`:

- **Accepted, adapted:** fresh-context subagents (`7739eaed`) in `DESIGN-IT-TWICE.md`. Applied as a separate sentence,
  "Start each sub-agent with a fresh context window.", to keep one instruction per sentence.

## Safe Update Boundary

Compare future upstream changes from `4ae2317d` over the mapped files. Treat local-only references and locally
maintained metadata as local behavior.

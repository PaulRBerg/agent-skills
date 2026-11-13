# llm-finder

- **Relationship:** first-party extraction and generalization from a private project-local predecessor. This is not a
  whole external skill fork.
- **Source revision:** `15ad49141ba7ba67ce36822f4e1ee8729b7ae465`.
- **Source location:** `.agents/skills/model-finder/` in the private predecessor repository. No public upstream URL is
  available.
- **Last reconciled base:** the source revision records component lineage. No whole-skill mechanical merge base exists.
- **Review boundary:** guidance review only. The local product wrapper is now a consumer of this global skill.

## Component Mapping

| Source path                                               | Catalog path                                  | Treatment                                                                                                          |
| --------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `.agents/skills/model-finder/SKILL.md`                    | `skills/llm-finder/SKILL.md`                  | Rewrite for general tasks, deployment choices, and explicit invocation. Product policy remains in the consumer.    |
| `.agents/skills/model-finder/references/sources.md`       | `skills/llm-finder/references/sources.md`     | Generalize source research, Hugging Face discovery, and conditional Gateway evidence.                              |
| `.agents/skills/model-finder/scripts/gateway-models.ts`   | `skills/llm-finder/scripts/gateway-models.ts` | Extract the helper and replace private runtime imports with builtins. Preserve validated unknown JSON fields.      |
| `.agents/skills/app-naming/scripts/skill-scripts.test.ts` | `tests/llm-finder/gateway-models.test.ts`     | Move Gateway regressions to the helper owner and add validation cases. Other source tests remain with their owner. |

## Local Behavior and Update Boundary

The catalog owns generic research and the portable Gateway helper. The product wrapper owns product selection,
repository context, deployment restrictions, and its stricter reputation gate. Do not copy those policies into the
catalog or mechanically merge a later wrapper into this skill. Compare guidance changes only within this mapping.

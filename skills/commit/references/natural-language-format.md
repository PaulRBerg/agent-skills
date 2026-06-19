# Natural Language Format

Use this format only when `--natural` is present. Write a human-readable commit subject in the spirit of Common Changelog entries: present-tense, imperative, impact-focused, and self-describing. Do not add a Conventional Commits prefix.

## Verb Selection

Choose the leading verb from the dominant user-visible intent, not the largest file diff or the presence of dependency/config churn. Common Changelog's core wording is useful, but do not stop at `Change`, `Add`, `Remove`, and `Fix` when a sharper verb exists.

| Behavior                                            | Leading verb          |
| --------------------------------------------------- | --------------------- |
| New functionality                                   | `Add`                 |
| Bug fix / error handling                            | `Fix`                 |
| Existing behavior or API change                     | `Change`              |
| Removed functionality                               | `Remove`              |
| Deprecated functionality                            | `Deprecate`           |
| Code migration or API adaptation without new UX/API | `Refactor`            |
| Code reorganization, no behavior change             | `Refactor`            |
| Documentation                                       | `Document`            |
| Tests                                               | `Test`                |
| Build system or local tooling                       | `Configure`, `Build`  |
| CI/CD pipelines                                     | `Configure`           |
| Dependency-only maintenance                         | `Bump`                |
| Formatting / whitespace only                        | `Format`              |
| Performance                                         | `Improve`, `Speed up` |
| Security                                            | `Harden`              |
| Reverting previous commit                           | `Revert`              |
| AI config (CLAUDE.md, .claude/, .gemini/, .codex/)  | `Update`              |
| Other maintenance                                   | `Update`              |

Use `Change` only for a meaningful behavior/API change. For maintenance, prefer a more precise verb such as `Configure`, `Bump`, `Document`, or `Refactor`.

If a dependency bump only enables a migration/refactor/fix, choose the migration/refactor/fix verb instead of `Bump`.

Explicit leading verb/category keywords in arguments take precedence over inference. Normalize lowercase keywords and `Changed`, `Added`, `Removed`, and `Fixed` to the table's imperative forms.

## Subject

- Subject line (\<= 72 chars, prefer \<= 50 when it still reads naturally): `Verb object/context`
- Capitalize the leading verb and proper nouns only
- No trailing period
- Describe what the change does, not which files changed
- Keep the subject self-describing without relying on a category label
- Do not add `type:`, `type(scope):`, ticket IDs, or changelog headings

Examples:

- `Add natural-language commit messages`
- `Fix commit hook retry handling`
- `Remove stale release helper`
- `Refactor API client setup`
- `Bump Foundry from 1.x to 2.x`

## Body

- Use hyphenated lines for distinct changes
- Skip the body for trivial changes
- With `--deep`, write 2-3 hyphenated lines max and focus on why the change exists
- For breaking changes, add `BREAKING CHANGE:` plus a one-line migration note

# Conventional Prefix Format

Use this format by default. Prefix the subject with a Conventional Commits-style type, optionally followed by a scope.

## Type Inference

Infer the type from the dominant user-visible intent, not the largest file diff or the presence of dependency/config churn.

Choose the highest-signal behavior that explains why the commit exists:

- If a dependency bump only enables a migration/refactor/fix, use the migration/refactor/fix type instead of `chore(deps)`.
- If tooling/scripts/config changes keep existing behavior working after a code migration, use the same type as the migration.
- Use `chore` only for maintenance that does not fit a more specific behavioral category.
- Use `chore(deps)` only for dependency-only updates or dependency updates whose main purpose is routine maintenance.

| Behavior                                            | Type          |
| --------------------------------------------------- | ------------- |
| New functionality                                   | `feat`        |
| Bug fix / error handling                            | `fix`         |
| Code migration or API adaptation without new UX/API | `refactor`    |
| Code reorganization, no behavior change             | `refactor`    |
| Documentation                                       | `docs`        |
| Tests                                               | `test`        |
| Build system (webpack, vite, esbuild)               | `build`       |
| CI/CD pipelines                                     | `ci`          |
| Dependency-only maintenance                         | `chore(deps)` |
| Formatting / whitespace only                        | `style`       |
| Performance                                         | `perf`        |
| Reverting previous commit                           | `revert`      |
| AI config (CLAUDE.md, .claude/, .gemini/, .codex/)  | `ai`          |
| Other maintenance                                   | `chore`       |

Explicit type keyword in arguments takes precedence over inference.

## Scope

Infer scope only when the path or code structure makes it obvious. Keep scope lowercase.

With `--deep`, infer scope from code structure even when the path alone is unclear.

## Subject

- Subject line (\<= 50 chars): `type(scope): description` or `type: description`
- Description uses imperative mood: `add`, not `added`
- Description is lowercase unless a proper noun or code identifier requires casing
- No trailing period
- Describe what the change does, not which files changed

## Body

- Use hyphenated lines for distinct changes
- Skip the body for trivial changes
- With `--deep`, write 2-3 hyphenated lines max and focus on why the change exists
- For breaking changes, add `BREAKING CHANGE:` plus a one-line migration note

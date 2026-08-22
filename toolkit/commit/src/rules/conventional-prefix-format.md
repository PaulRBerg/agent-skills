# Conventional Prefix Format

Use this format by default. Subject: `type(scope): description` or `type: description`.

## Type

Infer the type from the dominant user-visible intent, not the largest file diff or dependency/config churn. When a
dependency bump or tooling/config change only enables a migration, refactor, or fix, use that type instead of `chore`;
use `chore(deps)` only for routine dependency-only maintenance. An explicit type keyword in arguments overrides
inference.

- `feat` — new functionality
- `fix` — bug fix or error handling
- `refactor` — code migration, API adaptation, or reorganization without new UX/API or behavior change
- `docs` — documentation; `test` — tests; `style` — formatting/whitespace only; `perf` — performance
- `build` — build system; `ci` — CI/CD pipelines
- `chore(deps)` — dependency-only maintenance; `chore` — other maintenance
- `revert` — reverting a previous commit
- `ai` — AI config (CLAUDE.md, .claude/, .gemini/, .codex/)

## Scope

Optional and lowercase; infer it when the path or code structure makes it clear.

## Subject and Body

- Subject line (\<= 50 chars), imperative mood (`add`, not `added`), lowercase except proper nouns and code
  identifiers, no trailing period; describe what the change does, not which files changed
- Body: hyphenated lines focused on why the change exists; skip it for trivial changes
- For breaking changes, add `BREAKING CHANGE:` plus a one-line migration note

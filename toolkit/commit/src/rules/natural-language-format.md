# Natural Language Format

Write a present-tense, imperative, impact-focused subject in the spirit of Common Changelog: `Verb object/context`,
with no Conventional Commits prefix.

## Verb

Choose the leading verb from the dominant user-visible intent, not the largest file diff or dependency/config churn.
If a dependency bump only enables a migration, refactor, or fix, use that verb instead of `Bump`. Explicit leading
verb or category keywords in arguments override inference. Normalize lowercase or past-tense keywords (`Changed`,
`Added`, `Removed`, `Fixed`) to these imperative forms.

- `Add` — new functionality
- `Fix` — bug fix or error handling
- `Change` — meaningful behavior or API change only
- `Remove` — removed functionality
- `Deprecate` — deprecated functionality
- `Refactor` — code migration, API adaptation, or reorganization without behavior change
- `Document` — documentation
- `Test` — tests
- `Format` — formatting/whitespace only
- `Configure`/`Build` — build system, local tooling, CI/CD
- `Bump` — dependency-only maintenance
- `Improve`/`Speed up` — performance
- `Harden` — security
- `Revert` — reverting a previous commit
- `Update` — AI config (CLAUDE.md, .claude/, .gemini/, .codex/) and other maintenance

## Subject and Body

- Subject line (\<= 72 chars, prefer \<= 50 when it still reads naturally). Example: `Fix commit hook retry handling`.
- Capitalize only the leading verb and proper nouns. Use no trailing period.
  Do not use `type:` prefixes, ticket IDs, or changelog headings.
- Describe what the change does, not which files changed. Keep the subject self-describing.
- Body: use hyphenated lines focused on why the change exists. Skip it for trivial changes.
- For breaking changes, add `BREAKING CHANGE:` plus a one-line migration note

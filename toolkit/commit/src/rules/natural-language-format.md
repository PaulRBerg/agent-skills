# Natural Language Format

Write a present-tense, imperative, impact-focused subject in the spirit of Common Changelog: `Verb object/context`,
with no Conventional Commits prefix.

## Verb

Choose the leading verb from the dominant user-visible intent, not the largest file diff or dependency/config churn.
If a dependency bump only enables a migration, refactor, or fix, use that verb instead of `Bump`. Explicit leading
verb or category keywords in arguments override inference; normalize lowercase or past-tense keywords (`Changed`,
`Added`, `Removed`, `Fixed`) to these imperative forms.

- `Add` — new functionality; `Fix` — bug fix or error handling
- `Change` — meaningful behavior or API change only
- `Remove` — removed functionality; `Deprecate` — deprecated functionality
- `Refactor` — code migration, API adaptation, or reorganization without behavior change
- `Document` — documentation; `Test` — tests; `Format` — formatting/whitespace only
- `Configure`/`Build` — build system, local tooling, CI/CD
- `Bump` — dependency-only maintenance
- `Improve`/`Speed up` — performance; `Harden` — security; `Revert` — reverting a previous commit
- `Update` — AI config (CLAUDE.md, .claude/, .gemini/, .codex/) and other maintenance

## Subject and Body

- Subject line (\<= 72 chars, prefer \<= 50 when it still reads naturally), e.g. `Fix commit hook retry handling`
- Capitalize only the leading verb and proper nouns; no trailing period; no `type:` prefixes, ticket IDs, or
  changelog headings
- Describe what the change does, not which files changed; keep the subject self-describing
- Body: hyphenated lines focused on why the change exists; skip it for trivial changes
- For breaking changes, add `BREAKING CHANGE:` plus a one-line migration note

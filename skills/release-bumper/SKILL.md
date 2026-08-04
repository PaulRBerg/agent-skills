---
argument-hint: "[packages...] [version] [--beta] [--dry-run]"
disable-model-invocation: true
effort: high
model: sonnet
name: release-bumper
user-invocable: true
description: "Cut a release: bump versions, write changelogs, commit, tag."
---

# Release Bumper

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

Release one package or several packages with version bumps, changelog entries, commits, and tags. Supports
single-package repositories, workspace monorepos, stable releases, beta releases, and dry runs.

## Arguments

- `packages`: optional package names or directories. Omit in a single-package repository.
- `version`: optional explicit semver. Valid only for one user-selected package.
- `--beta`: create or advance a `-beta.X` prerelease.
- `--dry-run`: preview without modifying files, committing, or tagging.

## Helper Interface

Resolve `<skill-dir>` from this `SKILL.md`. Keep helper stdout as JSON and diagnostics on stderr.

```sh
node "<skill-dir>/scripts/plan-release.mjs" \
  [--cwd <repo>] [--beta] [--dry-run] [--version <semver>] \
  [--package <name-or-dir>]...
```

The read-only discovery output has `schemaVersion: 2`. It reports package identity, complete per-package `changedFiles`,
workspace edges and declared ranges, previous-tag facts, selected targets, and worktree state. `changeHints` are
filename-based, explicitly non-authoritative navigation hints. Never use them to decide release relevance or changelog
inclusion.

After the agent decides every stable patch/minor/major version, write discovery JSON to a temporary file and run:

```sh
uv run "<skill-dir>/scripts/finalize-release-plan.py" \
  --discovery <discovery.json> \
  [--version <package>=<semver>]...
```

The finalizer performs beta and explicit-version transitions, stable prerelease promotion, npm-range satisfaction,
simple dependency-range suggestions, and dependency ordering. It reports complex ranges, peer ranges, dependency cycles,
and stable versions not supplied by the agent as unresolved decisions. When an unsatisfied edge adds a dependent, choose
that package's release version and rerun with another `--version` assignment. The finalizer never chooses a regular
release magnitude or dependency policy.

For every stable changelog written, validate its deterministic structure:

```sh
uv run "<skill-dir>/scripts/validate-changelog.py" \
  --file <CHANGELOG.md> --version <semver> --date <YYYY-MM-DD> [--tag <tag>]
```

This checks the expected release and date, heading/category order, allowed categories, list structure, and release-link
tag. It does not judge importance, wording, or semantic category.

## Workflow

1. Run discovery with the user arguments mapped directly. Exit `2` means the target is not a releasable Git/package
   repository; exit `64` means invalid input. Stop on either.
2. Require `workingTree.clean`. Do not absorb unrelated work.
3. Resolve unknown or ambiguous package selection. An explicit user version remains single-package only.
4. Inspect each target's complete `changedFiles` and the net diff from its previous tag. Decide whether the surviving
   changes warrant a release. Runtime environments, refactors, documentation, tests, and tooling can all be relevant in
   context; filenames never decide this.
5. For every relevant stable target without an explicit version, choose patch, minor, or major from the consumer-facing
   change. For beta releases, let the finalizer compute the mechanical transition.
6. Run the finalizer. Review unsatisfied workspace edges. Accept its suggestion only for a simple dependency range when
   that policy fits; choose peer and complex range policy explicitly. Add dependents and their agent-chosen release
   versions, then rerun until the package set and dependency order are resolved.
7. For a dry run, report the ordered package/version plan, range edits, changelog/tag/commit actions, and agent-decided
   skips. Stop before writes.
8. For a stable release, read `references/common-changelog.md` and write consumer-facing entries from the bounded net
   diff. The agent owns entry selection, wording, importance, and category. Beta releases do not update changelogs.
9. Update manifests and accepted dependency ranges. Validate every stable changelog with the helper.
10. Format once using the repository's narrowest established command.
11. Commit and tag dependencies before dependents. Use one commit and one annotated tag per package:
    - single-package commit: `docs: release <version>`;
    - monorepo commit: `docs: release <package> <version>`;
    - single-package tag: follow observed `v<version>` or bare-semver facts;
    - monorepo tag: follow observed package tag facts, defaulting to `<package-dir>@<version>`.
12. Do not push. After success, recommend an exact `git push origin <tag>...` command containing only the tags created
    by this execution; do not use `--tags`.
13. Before the final report, inspect `.github/workflows/` for an active workflow that creates or publishes GitHub
    releases from pushed tags. A filename such as `release.yml` is a hint, not proof. Use `$cli-gh` read-only to check
    whether the repository has an established history of maintained GitHub releases. If it does, offer to create a
    GitHub release for each new tag, pending the user's approval, according to these rules:
    - One tag and release CI exists: do not offer manual release creation; the tag push should trigger CI.
    - One tag and no release CI exists: offer to create the release with `$cli-gh`.
    - Multiple tags will be pushed together: offer to create one release per tag with `$cli-gh` even when release CI
      exists, because the multi-tag push is not expected to trigger that automation reliably.

    Never create a GitHub release without the user's approval. If release history cannot be verified, report it as
    unknown and do not offer the write.

## Safety and Completion

Helper failures mean malformed input, violated invariants, or failed validation; an agent decision remaining unresolved
is data in the JSON, not a helper failure. Discovery and dry-run are read-only. Do not write changelogs before the final
stable package set is known, and do not infer a tag convention when discovery reports observed facts.

Dry-run completion requires a discovery-backed, agent-reviewed action preview with zero writes. Release completion
requires validated manifests and stable changelogs, formatting, one commit and annotated tag per package in dependency
order, and a report of created commits/tags, agent-decided skips, the exact tag-push command, and any applicable GitHub
release proposal.

Use `### ⛔ Release stopped — working tree is not clean`, `### ⚠️ Confirm release plan`,
`### 🔎 Release preview — no files, commits, or tags written`, or `### 🏁 Release complete` as applicable. Keep helper
JSON, versions, hashes, tags, commands, and changelog text exact and undecorated.

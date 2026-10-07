---
coordination: exempt
name: cli-gh
skill-dependencies:
  - repo-rename
  - yeet
user-invocable: false
description:
  "Use for GitHub CLI automation: repository reads, workflow runs, search, codespaces, releases, configuration, or gh
  command syntax. Use yeet for contribution writes to PRs, issues, comments, or discussions."
---

# GitHub CLI

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

## Routing

Use `yeet` to author or update a pull request, issue, issue comment, or discussion. `yeet` owns semantic analysis,
repository templates, Paul's writing voice, idempotency, and direct posting. Route repository renames through
`repo-rename` so GitHub and local continuity change together.

## Authority

- Read and inspect without confirmation.
- Execute reversible or ordinary GitHub writes only when the user explicitly requested that outcome.
- Never delete repositories, releases/assets, workflow runs/caches, secrets/variables, keys, codespaces, extensions, or
  gists.
- Label deletion is the sole destructive exception: show the target repo, exact labels, commands, and issue/PR impact,
  then require approval in a subsequent message before `gh label delete ... --yes`.

## Workflow

1. Resolve the repository explicitly when cwd is ambiguous. Let the first required read-only command validate
   authentication. Run `gh auth status` only for auth diagnosis.

2. Use installed `gh <command> <subcommand> --help` to resolve flags, JSON fields, and command behavior. CLI JSON field
   names can differ from API fields. Prefer `--json` with `--jq` for structured output.

3. Preview broad writes with the repository, exact targets, commands, and issue/PR impact. An already requested ordinary
   write does not require a second approval.

## Notable Flags (gh 2.98+)

- `gh pr checkout --worktree <path>` and `gh issue develop --checkout --worktree <path>` check out into a linked git
  worktree instead of switching the current branch.
- `gh search issues --search-type semantic|hybrid` ranks by relevance: issues only, one page, no `--sort`/`--order`, not
  on GitHub Enterprise Server.
- `gh config set api_host <gateway> --host <host>` routes that host's API traffic through a gateway. The feature is
  experimental and is not a security boundary.
- `gh auth login` and `gh auth refresh` copy OAuth device codes to the clipboard by default (2.101+). To opt out, run
  `gh config set clipboard disabled`.
- `--attach '<file>#<alt>'` on issue/PR create, edit, and comment uploads images or videos. Route those writes through
  `yeet`.

## Completion

Complete when command output verifies the requested GitHub data or state. After a write, fetch the resulting resource.
On a partial or ambiguous failure, check whether it changed before retrying. Report the verified state and URL or stable
identifier. Include the concrete error and next action when incomplete, and distinguish an unknown result from a
confirmed failure.

---
disable-model-invocation: false
name: cli-gh
user-invocable: false
description:
  "Use for GitHub CLI automation: repository reads, workflow runs, search, codespaces, releases, configuration, or gh
  command syntax. Use yeet for contribution writes to PRs, issues, comments, or discussions."
---

# GitHub CLI

Route GitHub CLI work through current `gh` help and load only the reference for the active task.

## Boundary with `yeet`

Use `yeet` when the requested outcome is to create or update a pull request, issue, issue comment, or discussion. `yeet`
owns semantic analysis, repository templates, Paul's writing voice, idempotency, and direct posting. Use this skill for
read-only GitHub inspection, command syntax, searches, workflow operations, codespaces, releases, configuration, and
other GitHub automation.

## Authority

- Read and inspect without confirmation.
- Execute reversible or ordinary GitHub writes only when the user explicitly requested that outcome.
- Never delete repositories, releases/assets, workflow runs/caches, secrets/variables, keys, codespaces, extensions, or
  gists.
- Label deletion is the sole destructive exception: show the target repo, exact labels, commands, and issue/PR impact,
  then require approval in a subsequent message before `gh label delete ... --yes`.
- Route repository renames through `repo-rename` so GitHub and local continuity change together.

## Workflow

1. Resolve the repository explicitly when cwd is ambiguous. Let the first required read-only command validate
   authentication; run `gh auth status` only for auth diagnosis.

2. Inspect `gh <command> <subcommand> --help` for the installed version before relying on flags or JSON fields. Prefer
   `--json` plus `--jq` for machine-readable results.

3. Load only the relevant reference:

   | Task                                                      | Reference                            |
   | --------------------------------------------------------- | ------------------------------------ |
   | Workflow runs, checks, logs                               | `references/workflows-actions.md`    |
   | Releases                                                  | `references/releases.md`             |
   | Search                                                    | `references/search.md`               |
   | JSON fields and jq                                        | `references/json-output.md`          |
   | Labels                                                    | `references/labels.md`               |
   | Codespaces                                                | `references/codespaces.md`           |
   | Discussions syntax (not authored contribution workflow)   | `references/discussions.md`          |
   | Gists                                                     | `references/gists.md`                |
   | Aliases, API, extensions, org/projects, secrets, rulesets | `references/advanced-features.md`    |
   | Reusable automation patterns                              | `references/automation-workflows.md` |
   | Failures and auth/rate limits                             | `references/troubleshooting.md`      |

4. Preview commands that have broad write scope. After a requested write, fetch the resulting resource and report its
   URL or stable identifier.

Completion requires the requested GitHub state or data plus command/output evidence. On a partial or ambiguous write
failure, check whether the resource changed before retrying.

## User-Facing Output

Use `### ⚠️ GitHub write preview` for broad writes, showing the repository, exact targets, and issue/PR impact in a
compact table, with each exact command in its own fenced block; an already requested ordinary write does not require a
second approval. For label deletion, use `### ⛔ Destructive approval required` and ask for approval in a later message.
After a verified write, use `### ✅ GitHub operation complete` with the action, repository, resource, and URL or stable
ID. On failure, state the attempted operation, verified resulting state, concrete error, and next action without
implying a write succeeded. Keep commands, JSON, identifiers, diagnostics, and output intended for piping undecorated.

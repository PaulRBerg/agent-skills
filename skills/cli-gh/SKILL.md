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

## GitHub Status

GitHub incidents produce symptoms that look like a local defect. Examples: a run stays `queued` for minutes with no
runner, `gh run cancel` reports a run as completed while `gh run view` still shows it queued, the API returns 5xx
errors, or a push succeeds while Actions does not start.

Before you tell the user that an error, a stall, or an inconsistent result comes from their setup, workflow, or
credentials, check GitHub's status page:

```sh
curl -fsS https://www.githubstatus.com/api/v2/summary.json | jq '{status: .status.description,
  incidents: [.incidents[] | {name, status, impact, updated_at}],
  degraded: [.components[] | select(.status != "operational") | {name, status}]}'
```

If an incident is open or an affected component is not `operational`, report the incident name, the affected components,
and the latest update time. Attribute the symptom to the incident. Do not change the user's workflow, configuration, or
credentials to work around it. Wait for mitigation, then retry. If the status page shows no incident, continue with
local diagnosis.

## CI Failure Triage

Use this workflow when the user asks why CI fails or asks to fix a failing run. Keep each command output bounded.

1. Find the failing runs for the branch or the commit:

   ```sh
   gh run list --branch <branch> --status failure --limit 5
   gh run list --commit <sha> --status failure --limit 5
   ```

2. Extract only the failed-step lines. Never print the raw log.

   ```sh
   gh run view <run-id> --log-failed | rg --no-config -n 'error|FAIL|panic|✗' | head -80
   ```

3. Check toolchain drift before you change code. Compare the versions pinned in `.github/workflows/*.yml` with the local
   `--version` output. Common cases are Foundry (`forge`), Bun (`bun`), Node.js (`node`), and Rust (`rustc`).
4. If the user requested a fix, fix the cause and push the commit.
5. Get the new run ID with `gh run list --commit <new-sha> --limit 5`. Then watch that run:

   ```sh
   gh run watch <run-id> --exit-status --compact
   ```

6. Report the run URL from `gh run view <run-id> --json url --jq .url` and the final run status.

## Notable Flags (gh 2.98+)

- `gh pr checkout --worktree <path>` and `gh issue develop --checkout --worktree <path>` check out into a linked git
  worktree instead of switching the current branch.
- `gh search issues --search-type semantic|hybrid` ranks by relevance: issues only, one page, no `--sort`/`--order`, not
  on GitHub Enterprise Server.
- `gh config set api_host <gateway> --host <host>` routes that host's API traffic through a gateway. The feature is
  experimental and is not a security boundary.
- `gh auth login` and `gh auth refresh` copy OAuth device codes to the clipboard by default (2.101+). To opt out, run
  `gh config set clipboard disabled`.
- `gh auth login --expiring` (2.103+) prefers expiring OAuth tokens that `gh` refreshes. To prefer them on every
  sign-in, run `gh config set expiring_credentials enabled`. Run `gh auth token` each time you need a token. Never cache
  its output. `GH_TOKEN` and `GITHUB_TOKEN` values never refresh.
- `gh auth status` refreshes an expired or near-expiry refreshable token unless you pass `--no-refresh`. With that flag,
  2.103.3+ reports the token as expired, not invalid. To renew it, run the command without the flag. Do not run
  `gh auth refresh` for this case.
- `--attach '<file>#<alt>'` on issue/PR create, edit, and comment uploads images or videos. Route those writes through
  `yeet`. Since 2.103.2, GitHub App installation tokens can attach, and the upload endpoint decides permission.

## Completion

Complete when command output verifies the requested GitHub data or state. After a write, fetch the resulting resource.
On a partial or ambiguous failure, check whether it changed before retrying. Report the verified state and URL or stable
identifier. Include the concrete error and next action when incomplete, and distinguish an unknown result from a
confirmed failure.

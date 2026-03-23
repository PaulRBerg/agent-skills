---
argument-hint: '[<pr-number|pr-url>] [--local [--base <branch>] [--type <type>]]'
disable-model-invocation: false
name: coderabbit
user-invocable: true
description: This skill should be used when the user asks to "triage coderabbit", "check coderabbit comments", "analyze coderabbit review", "fetch coderabbit findings", "coderabbit PR", "review coderabbit suggestions", "run coderabbit review", "coderabbit review", "review with coderabbit", or mentions analyzing CodeRabbit PR comments or running CodeRabbit CLI review locally.
---

# CodeRabbit Review Triage

Fetch and analyze CodeRabbit review comments from a GitHub PR, or run a local CodeRabbit CLI review. Default mode fetches from the remote PR. Use `--local` to run the CLI against local diffs.

## Arguments

Parse `$ARGUMENTS` for:

- `<pr-number>` — integer PR number
- `<pr-url>` — GitHub PR URL (e.g., `https://github.com/owner/repo/pull/123`), extract the PR number from the path
- `--local` — switch to local review mode (see below)

### Mode Dispatch

If `$ARGUMENTS` contains `--local`, strip it and forward remaining arguments to `references/local-review.md`. Follow that workflow instead of continuing here. All flags after `--local` (`--base`, `--base-commit`, `--type`, `--config`) are forwarded to the local workflow.

Otherwise, continue with the PR comment analysis workflow below.

## Prerequisites

Run these checks in order. Stop at the first failure.

### 1) GitHub CLI Authenticated

```bash
gh auth status
```

If not authenticated, stop and tell the user to run `gh auth login`.

### 2) Detect Repository

```bash
gh repo view --json owner,name --jq '"\(.owner.login)/\(.name)"'
```

Store the result as `{owner}/{repo}`.

### 3) Resolve PR Number

- If a PR number was provided as an argument, use it directly.
- If a PR URL was provided, extract the number from the URL path.
- If neither was provided, auto-detect from the current branch:

```bash
gh pr view --json number --jq '.number' 2>/dev/null
```

If auto-detection fails, stop: "No open PR found for the current branch. Provide a PR number or URL, or use `--local` for local review."

### 4) Validate PR

```bash
gh pr view {pr_number} --json state,url --jq '"\(.state) \(.url)"'
```

If the command fails, the PR does not exist. Report and stop. Otherwise, note the state (OPEN/MERGED/CLOSED) and URL for the report scope section.

## Workflow

### 1) Fetch CodeRabbit Comments

Fetch from three GitHub API endpoints, filtering for the CodeRabbit bot. Use `--paginate` for large PRs. The bot's login is `coderabbitai` but may appear as `coderabbitai[bot]`, so match with `startswith`.

**Walkthrough/summary comments** (issue comments where CodeRabbit posts its walkthrough):

```bash
gh api "repos/{owner}/{repo}/issues/{pr_number}/comments" \
  --paginate \
  --jq '[.[] | select(.user.login | startswith("coderabbitai"))]'
```

**Review objects** (top-level review body and state):

```bash
gh api "repos/{owner}/{repo}/pulls/{pr_number}/reviews" \
  --paginate \
  --jq '[.[] | select(.user.login | startswith("coderabbitai"))]'
```

**Inline review comments** (file-level comments with path and line info):

```bash
gh api "repos/{owner}/{repo}/pulls/{pr_number}/comments" \
  --paginate \
  --jq '[.[] | select(.user.login | startswith("coderabbitai"))]'
```

If no CodeRabbit comments are found across all three endpoints, report "No CodeRabbit review found on PR #{pr_number}" and stop.

### 2) Parse Comments into Findings

Process the raw API responses into a normalized list of findings:

- **From inline comments** (`pulls/{pr}/comments`): Each comment becomes a finding with `path` (from `.path`), `line` (from `.line` or `.original_line`), `body` (the comment text), `diff_hunk` (surrounding context), and `source: "inline"`. These are the primary source of actionable findings.
- **From review bodies** (`pulls/{pr}/reviews`): Parse each review's `body` for actionable items. Extract individual suggestions. Mark `source: "review"`.
- **From issue comments** (`issues/{pr}/comments`): Parse for file-level summaries and actionable items within the walkthrough. Mark `source: "walkthrough"`.

Inline comments are the primary source of actionable findings. Review summaries and walkthrough comments provide context and may surface additional actionable items not tied to specific lines.

### 3) Triage Findings

Load `references/triage.md` and follow the shared triage process — categorize, classify, confirm ambiguous, generate fix plan, and report.

## Stop Conditions

Stop and ask for direction when:

- `gh` CLI is not authenticated.
- The PR does not exist or is not accessible.
- No CodeRabbit comments found on the PR.
- Auto-detection fails and no PR number was provided.
- A finding requires architectural changes beyond the PR's scope.
- Classification confidence is low for code on critical paths (auth, payments, data integrity).

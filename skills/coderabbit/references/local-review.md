# Local CodeRabbit CLI Review

Run a CodeRabbit CLI review on local changes and triage the output.

## Arguments

Parse forwarded arguments for optional flags:

- `--base <branch>` — base branch for diff comparison
- `--base-commit <commit>` — base commit on current branch for comparison (mutually exclusive with `--base`)
- `--type <type>` — review scope: `committed`, `uncommitted`, or `all` (default: `all`)
- `--config <files...>` — additional instruction files to pass to CodeRabbit

All flags are optional. If neither `--base` nor `--base-commit` is provided, auto-detect the base branch:

1. Try `git rev-parse --abbrev-ref @{upstream}` (strip remote prefix)
2. Fall back to `git symbolic-ref refs/remotes/origin/HEAD` (strip remote prefix)
3. Fall back to `main`, then `master`

Verify the resolved ref exists with `git rev-parse --verify`.

## Prerequisites

Run these checks in order. Stop at the first failure.

### 1) CodeRabbit CLI Installed

```bash
command -v coderabbit
```

If missing, stop and tell the user to install:

```
curl -fsSL https://cli.coderabbit.ai/install.sh | sh
```

### 2) Authentication

```bash
coderabbit auth status
```

If the exit code indicates unauthenticated or the output does not show a logged-in state, stop and tell the user to authenticate:

```
coderabbit auth login
```

## Workflow

### 1) Run CodeRabbit Review

Construct the command with mandatory flags `--plain --no-color` plus any user-provided flags:

```bash
coderabbit review --plain --no-color [--base <branch>] [--base-commit <commit>] [--type <type>] [--config <files...>]
```

Capture the full output. If the CLI exits non-zero, report the error and stop.

### 2) Parse CLI Output into Findings

Parse the plain-text CLI output into individual findings. Group by file path. Each finding gets: `path`, `line` (if parseable from output), `body` (the suggestion text), and `source: "cli"`.

### 3) Triage Findings

Load `references/triage.md` and follow the shared triage process — categorize, classify, confirm ambiguous, generate fix plan, and report.

## Stop Conditions

Stop and ask for direction when:

- `coderabbit` CLI is not installed or not authenticated.
- The CLI exits with an error.
- The resolved base ref does not exist.
- No findings in the CLI output (report clean and exit).
- A suggestion requires architectural changes beyond the current diff's scope.
- Classification confidence is low for code on critical paths (auth, payments, data integrity).

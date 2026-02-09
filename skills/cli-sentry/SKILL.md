---
name: cli-sentry
user-invocable: true
description: This skill should be used when the user asks to "fetch Sentry issues", "check Sentry errors", "triage Sentry", "categorize Sentry issues", "resolve Sentry issue", "mute Sentry issue", "unresolve Sentry issue", "sentry-cli", or mentions Sentry API, Sentry project issues, error monitoring, issue triage, Sentry stack traces, or browser extension errors in Sentry.
---

# Sentry CLI Issue Management

## Overview

Expert guidance for managing Sentry issues via the CLI and API. Use this skill for fetching, triaging, categorizing, and resolving Sentry issues.

**Key capabilities:**

- Preflight validation of sentry-cli installation and auth
- List and filter issues by status, time period, and query
- Categorize issues (Valid, False Positive, Already Resolved, Third Party)
- Resolve, mute, and unresolve issues (individually or in bulk)

## Safety Rules

**Prohibited operations:**

- `DELETE /issues/{id}/` - Issue deletion (irreversible)
- Project or release deletion
- Bulk status changes without explicit user confirmation
- Modifying alert rules, notification settings, or project configuration

**Allowed operations:**

- Listing and viewing issues (read-only)
- Resolving issues (reversible via unresolve)
- Muting/ignoring issues (reversible via unmute)
- Fetching event details and stack traces

## Prerequisites

### Required Environment Variables

The following env vars **must** be set before running any Sentry operations. Add them to your project's `.envrc`:

```bash
export SENTRY_AUTH_TOKEN=sntrys_...
export SENTRY_PROJECT=<your-project-slug>
```

Then run `direnv allow` to load them.

- **`SENTRY_AUTH_TOKEN`** — Generate at https://sentry.io/settings/account/api/auth-tokens/
- **`SENTRY_PROJECT`** — Your project slug in Sentry

> These are the native sentry-cli env var names. The CLI reads them automatically, so setting them in `.envrc` means you don't need to pass `--project` or `--auth-token` flags.

### Preflight Check

Run the preflight check before any Sentry operations:

```bash
bash scripts/check-sentry.sh -v
```

This validates env vars, sentry-cli installation, and authentication.

### Configuration Resolution Order

Settings resolve in this order (first wins):

1. CLI flags (`--org`, `--project`, `--auth-token`)
2. Environment variables (`SENTRY_ORG`, `SENTRY_PROJECT`, `SENTRY_AUTH_TOKEN`)
3. `.sentryclirc` file (project root or `~/.sentryclirc`)


## Issue Management

### List Issues (CLI)

```bash
# List unresolved issues
sentry-cli issues list --project <project>

# List with status filter
sentry-cli issues list --project <project> --status unresolved
```

### List Issues (API - Richer Data)

Use `scripts/fetch-issues.sh` for triage-quality JSON with metadata, culprit, event counts, and timestamps:

```bash
bash scripts/fetch-issues.sh --org=<org> --project=<project>
bash scripts/fetch-issues.sh --org=<org> --project=<project> --stats-period=7d --limit=50
```

### Get Issue Details (API)

```bash
curl -s -H "Authorization: Bearer $SENTRY_AUTH_TOKEN" \
  "https://sentry.io/api/0/issues/{issue_id}/" | jq
```

### Get Latest Event / Stack Trace (API)

```bash
curl -s -H "Authorization: Bearer $SENTRY_AUTH_TOKEN" \
  "https://sentry.io/api/0/issues/{issue_id}/events/latest/" | jq '.exception'
```

### Resolve / Mute / Unresolve (CLI)

```bash
# Resolve
sentry-cli issues resolve <issue_id> --project <project>

# Mute (ignore)
sentry-cli issues mute <issue_id> --project <project>

# Unresolve
sentry-cli issues unresolve <issue_id> --project <project>
```

## Issue Categorization

Categorize each issue into one of four categories:

### 1. Valid

Genuine application errors requiring investigation and fixes.

**Indicators:**

- Stack trace points to application code (`src/`, `app/`, `webpack://`)
- Error originates from application logic, not external code
- Reproducible user-facing issues
- API errors from application endpoints

### 2. False Positive

Errors that appear as issues but are not actual problems.

**Indicators:**

- Network errors from user connectivity issues (timeouts, DNS failures)
- Browser-specific quirks not affecting functionality
- Expected errors (e.g., 401 for unauthenticated users, 404 for deleted resources)
- Errors from automated bots/crawlers

### 3. Already Resolved

Issues that have been fixed in subsequent deployments.

**Indicators:**

- Last seen date predates a known fix
- No recent occurrences (check `lastSeen` field)
- Related code has been refactored or removed
- Issue matches a closed GitHub issue or merged PR

### 4. Third Party

Errors originating from browser extensions or external scripts. See `references/extension-patterns.md` for comprehensive detection patterns.

**Indicators:**

- Stack trace contains extension URLs (`chrome-extension://`, `moz-extension://`, etc.)
- Error from injected scripts (`inpage.js`, `content.js`, `inject.js`)
- Known extension error messages (`ResizeObserver loop`, `Extension context invalidated`)
- Stack trace mentions known extensions (MetaMask, Grammarly, LastPass, etc.)

## Triage Workflow

1. **Preflight** - Run `bash scripts/check-sentry.sh -v` to verify sentry-cli and auth
2. **Fetch** - Run `bash scripts/fetch-issues.sh --org=<org> --project=<project>` to get all unresolved issues
3. **Inspect** - For each issue, fetch its latest event to examine the stack trace:
   ```bash
   curl -s -H "Authorization: Bearer $SENTRY_AUTH_TOKEN" \
     "https://sentry.io/api/0/issues/{issue_id}/events/latest/" | jq '.exception.values[0].stacktrace.frames'
   ```
   Check `culprit`, `title`, `metadata`, `lastSeen`, and event count
4. **Categorize** - Assign each issue to Valid, False Positive, Already Resolved, or Third Party
5. **Present** - Summarize in a triage report table:

```markdown
## Sentry Issue Triage Report

### Valid (N issues)

| Issue | Title | Events | Last Seen |
| --- | --- | --- | --- |
| PROJ-123 | TypeError in Component | 45 | 2h ago |

### Third Party (N issues)

| Issue | Title | Source | Events |
| --- | --- | --- | --- |
| PROJ-126 | ResizeObserver loop | Browser Extension | 234 |

### False Positives (N issues)

| Issue | Title | Reason |
| --- | --- | --- |
| PROJ-128 | Network Error | User connectivity |

### Already Resolved (N issues)

| Issue | Title | Last Seen | Notes |
| --- | --- | --- | --- |
| PROJ-130 | Hydration mismatch | 14d ago | Fixed in v2.3.1 |
```

## Bulk Actions

After triage, resolve or mute issues in bulk. Always confirm with the user before executing.

```bash
# Bulk resolve via API
curl -X PUT -H "Authorization: Bearer $SENTRY_AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"status": "resolved"}' \
  "https://sentry.io/api/0/projects/{org}/{project}/issues/?id=123&id=456&id=789"

# Bulk ignore/mute via API
curl -X PUT -H "Authorization: Bearer $SENTRY_AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"status": "ignored"}' \
  "https://sentry.io/api/0/projects/{org}/{project}/issues/?id=123&id=456&id=789"
```

## Quick Reference

| Operation          | Method | Command / Endpoint                                |
| ------------------ | ------ | ------------------------------------------------- |
| List issues        | CLI    | `sentry-cli issues list --project <p>`            |
| List issues (rich) | Script | `scripts/fetch-issues.sh --org=<o> --project=<p>` |
| Issue details      | API    | `GET /issues/{id}/`                               |
| Latest event       | API    | `GET /issues/{id}/events/latest/`                 |
| Event list         | API    | `GET /issues/{id}/events/`                        |
| Resolve            | CLI    | `sentry-cli issues resolve <id>`                  |
| Mute               | CLI    | `sentry-cli issues mute <id>`                     |
| Unresolve          | CLI    | `sentry-cli issues unresolve <id>`                |
| Bulk update        | API    | `PUT /projects/{org}/{project}/issues/?id=...`    |

## Additional Resources

- **`scripts/check-sentry.sh`** - Preflight validation (installation, responsiveness, auth)
- **`scripts/fetch-issues.sh`** - Fetch unresolved issues with rich JSON
- **`references/api-fallbacks.md`** - API endpoints for operations sentry-cli can't handle
- **`references/extension-patterns.md`** - Browser extension error detection patterns

## Tips

1. Run `sentry-cli info` to verify configuration and auth status from any source
2. Pipe API responses through `jq` for readable output: `... | jq '.[] | {shortId, title, count, lastSeen}'`
3. Triage Third Party issues first - they are the easiest to identify and often the most numerous
4. Check `references/extension-patterns.md` before categorizing ambiguous stack traces

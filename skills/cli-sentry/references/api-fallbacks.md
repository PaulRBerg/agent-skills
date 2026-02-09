# API Fallbacks

Endpoints that `sentry-cli` cannot handle, requiring direct API calls. Use `~/.agents/skills/cli-sentry/scripts/sentry-api.sh` for all API requests — it handles authentication automatically. Do NOT construct `Authorization` headers manually.

Base URL: `https://sentry.io/api/0`

## Endpoints

### Get Issue Details

```
GET /issues/{issue_id}/
```

Returns full issue metadata, statistics, tags, and activity. Use when `sentry-cli issues list` output lacks fields like `metadata`, `culprit`, or `userCount`.

```bash
bash ~/.agents/skills/cli-sentry/scripts/sentry-api.sh GET /issues/{issue_id}/ | jq
```

### Get Latest Event

```
GET /issues/{issue_id}/events/latest/
```

Returns the most recent event including full stack traces. Essential for triage categorization - the `exception.values[].stacktrace.frames` array reveals whether the error originates from application code or third-party extensions.

```bash
bash ~/.agents/skills/cli-sentry/scripts/sentry-api.sh GET /issues/{issue_id}/events/latest/ | jq '.exception // {note: "no exception data"}'
```

### List Events

```
GET /issues/{issue_id}/events/
```

Query parameters: `full=true` for complete event details. Use when the latest event is insufficient (e.g., intermittent issues with varying stack traces).

```bash
bash ~/.agents/skills/cli-sentry/scripts/sentry-api.sh GET "/issues/{issue_id}/events/?full=true" | jq
```

### Bulk Status Update

```
PUT /projects/{org}/{project}/issues/?id=123&id=456&id=789
```

Body:

```json
{
  "status": "resolved" | "ignored",
  "statusDetails": {
    "ignoreCount": 100,
    "ignoreDuration": 60,
    "ignoreUntil": "2025-03-01"
  }
}
```

Repeat `id` query parameter for each issue. The script auto-substitutes `{org}` and `{project}` from `~/.sentryclirc`.

```bash
bash ~/.agents/skills/cli-sentry/scripts/sentry-api.sh PUT \
  "/projects/{org}/{project}/issues/?id=123&id=456" \
  '{"status": "resolved"}'
```

## Search Query Syntax

| Query            | Description              |
| ---------------- | ------------------------ |
| `is:unresolved`  | Active issues            |
| `is:resolved`    | Fixed issues             |
| `is:ignored`     | Muted/archived issues    |
| `firstSeen:-24h` | New in last 24 hours     |
| `lastSeen:-7d`   | Seen in last 7 days      |
| `assigned:me`    | Assigned to current user |
| `assigned:none`  | Unassigned               |
| `level:error`    | Error severity only      |
| `has:user`       | Has user context         |

Combine queries: `is:unresolved level:error lastSeen:-24h`

## Event Data Structure

Key fields in event responses for categorization:

```
exception.values[].type          — Error type (TypeError, ReferenceError)
exception.values[].value         — Error message
exception.values[].stacktrace.frames[].filename   — Source file
exception.values[].stacktrace.frames[].function   — Function name
exception.values[].stacktrace.frames[].absPath    — Full path (reveals extension:// URLs)
exception.values[].stacktrace.frames[].inApp      — Whether frame is application code
exception.values[].stacktrace.frames[].module     — Module path
```

## Rate Limits & Pagination

**Rate limits** - Check response headers:

```
X-Sentry-Rate-Limit-Remaining: 95
X-Sentry-Rate-Limit-Reset: 1640000000
```

**Pagination** - Cursor-based via `Link` header:

```
Link: <https://sentry.io/api/0/.../?cursor=abc>; rel="next"; results="true"
```

Parse the `cursor` parameter for subsequent requests. Stop when `results="false"`.

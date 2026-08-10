# Wayback Machine

Use the Wayback APIs to select one capture, then use shared Chromium only when inspecting the rendered replay adds
evidence. The traffic limits below are local safety invariants derived from observed throttling, not Internet Archive
service guarantees.

## Traffic Safety

- Run API discovery outside Chromium with one foreground request at a time. Wait for it to finish before starting the
  next request.
- Never parallelize, poll, or launch background Wayback probes. Do not configure retries.
- Bound every prefix, host, or domain query with selective filters, collapse rules, and a positive `limit`.
- On the first HTTP `429` or connection timeout from `archive.org` or `web.archive.org`, stop all Wayback traffic for
  the rest of the task and session. Do not retry through CDX, Availability, replay navigation, Chromium, a proxy, or
  another route. Report that the archive evidence source is blocked and leave unsupported conclusions unknown.

## CDX Discovery

Query `https://web.archive.org/cdx/search/cdx` with these parameters. The
[official CDX documentation](https://github.com/internetarchive/wayback/blob/master/wayback-cdx-server/README.md) is
authoritative.

| Parameter   | Use                                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `url`       | Required capture target. URL-encode its value, especially when the target contains its own query string.                           |
| `matchType` | `exact` (default), `prefix` for a path tree, `host` for one host, or `domain` for the host and all subdomains.                     |
| `from`      | Inclusive lower timestamp bound with 1–14 digits in `yyyyMMddhhmmss` order.                                                        |
| `to`        | Inclusive upper timestamp bound with 1–14 digits in `yyyyMMddhhmmss` order.                                                        |
| `filter`    | Repeatable `[!]field:regex` predicate, such as `statuscode:200`; repeated filters compose constraints.                             |
| `collapse`  | Keep the first of adjacent rows equal on `field`, or on the first `N` characters with `field:N`; repeat for additional reductions. |
| `limit`     | Maximum returned rows. Use a positive value; negative limits may require scanning the search space.                                |
| `output`    | Use `json` for a JSON array whose first row contains the field names and whose remaining rows are captures.                        |

Collapse is adjacency-based, not global deduplication: a duplicate outside the neighboring run remains. For example,
`collapse=timestamp:10` keeps at most the first adjacent capture per hour, while `collapse=digest` removes only adjacent
captures with the same digest.

`url=example.com/*` is an implicit path-prefix query equivalent to `url=example.com/&matchType=prefix`; it does not
enumerate the whole domain. Use explicit `matchType=domain` for the host plus its subdomains.

Run each example separately and synchronously. None retries.

### Exact URL

```sh
curl --get --fail-with-body --show-error --silent \
  --connect-timeout 10 --max-time 30 \
  'https://web.archive.org/cdx/search/cdx' \
  --data-urlencode 'url=https://example.com/path?item=1' \
  --data-urlencode 'matchType=exact' \
  --data-urlencode 'output=json' \
  --data-urlencode 'limit=50'
```

### Path Prefix

```sh
curl --get --fail-with-body --show-error --silent \
  --connect-timeout 10 --max-time 30 \
  'https://web.archive.org/cdx/search/cdx' \
  --data-urlencode 'url=https://example.com/docs/' \
  --data-urlencode 'matchType=prefix' \
  --data-urlencode 'filter=statuscode:200' \
  --data-urlencode 'collapse=urlkey' \
  --data-urlencode 'output=json' \
  --data-urlencode 'limit=100'
```

### Full Domain

```sh
curl --get --fail-with-body --show-error --silent \
  --connect-timeout 10 --max-time 30 \
  'https://web.archive.org/cdx/search/cdx' \
  --data-urlencode 'url=example.com' \
  --data-urlencode 'matchType=domain' \
  --data-urlencode 'filter=statuscode:200' \
  --data-urlencode 'collapse=urlkey' \
  --data-urlencode 'output=json' \
  --data-urlencode 'limit=100'
```

### Bounded Successful Captures

```sh
curl --get --fail-with-body --show-error --silent \
  --connect-timeout 10 --max-time 30 \
  'https://web.archive.org/cdx/search/cdx' \
  --data-urlencode 'url=https://example.com/docs/' \
  --data-urlencode 'matchType=prefix' \
  --data-urlencode 'from=20200101' \
  --data-urlencode 'to=20201231235959' \
  --data-urlencode 'filter=statuscode:200' \
  --data-urlencode 'collapse=digest' \
  --data-urlencode 'output=json' \
  --data-urlencode 'limit=100'
```

## Closest Snapshot

The [official Availability API documentation](https://archive.org/help/wayback_api.php) defines required `url` and
optional `timestamp` parameters. The timestamp accepts 1–14 digits in `YYYYMMDDhhmmss` order. When omitted, the API
returns the most recent accessible capture; when present, `archived_snapshots.closest` describes the closest accessible
capture, not necessarily an exact timestamp match.

```sh
curl --get --fail-with-body --show-error --silent \
  --connect-timeout 10 --max-time 30 \
  'https://archive.org/wayback/available' \
  --data-urlencode 'url=https://example.com/path' \
  --data-urlencode 'timestamp=20200101'
```

A successful match includes its replay URL, timestamp, status, and availability:

```json
{
  "archived_snapshots": {
    "closest": {
      "available": true,
      "url": "https://web.archive.org/web/20200102030405/https://example.com/path",
      "timestamp": "20200102030405",
      "status": "200"
    }
  }
}
```

A valid negative has this empty snapshot shape; it is not a transport failure:

```json
{ "archived_snapshots": {} }
```

## Replay Inspection

Choose one replay from a CDX row or `archived_snapshots.closest`. For a CDX row, the replay URL has the form
`https://web.archive.org/web/<timestamp>/<original>`. Open only that selected URL in the shared Chromium browser, and
only when rendered DOM, layout, script behavior, or visual state materially strengthens the evidence. Do not use
Chromium for capture discovery, calendar browsing, Availability checks, or repeated replay probes.

Follow the parent skill's page-ownership rules: record the page returned by `new_page`, pass its `pageId` explicitly,
and close only that task-created page. A replay timeout triggers the same session-long traffic stop as an API timeout.

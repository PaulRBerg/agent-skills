#!/usr/bin/env bash
# fetch-issues.sh - Fetch unresolved Sentry issues with rich JSON
#
# Uses the Sentry REST API (not sentry-cli issues list) because the API
# returns richer JSON needed for triage (metadata, culprit, shortId,
# lastSeen, firstSeen, count).
#
# Exit codes:
#   0 - Success
#   1 - Missing auth token
#   2 - Missing org or project
#   3 - API request failed
#
# Options:
#   --org=          Sentry organization slug
#   --project=      Sentry project slug
#   --stats-period= Stats time window (default: 14d)
#   --limit=        Max issues to return (default: 100)

set -euo pipefail

# Defaults
ORG=""
PROJECT=""
STATS_PERIOD="14d"
LIMIT="100"

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Fetch unresolved Sentry issues with full metadata.

Options:
  --org=ORG             Sentry organization slug
  --project=PROJECT     Sentry project slug
  --stats-period=PERIOD Stats time window (default: 14d)
  --limit=N             Max issues to return (default: 100)
  -h, --help            Show this help message

Config resolution (first wins):
  1. CLI flags (--org, --project)
  2. Environment variables (SENTRY_ORG, SENTRY_PROJECT)
  3. .sentryclirc file

Auth token resolution:
  1. SENTRY_AUTH_TOKEN environment variable
  2. .sentryclirc [auth] token
EOF
}

# Parse arguments
for arg in "$@"; do
    case $arg in
        --org=*) ORG="${arg#*=}" ;;
        --project=*) PROJECT="${arg#*=}" ;;
        --stats-period=*) STATS_PERIOD="${arg#*=}" ;;
        --limit=*) LIMIT="${arg#*=}" ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown argument: $arg" >&2; usage >&2; exit 1 ;;
    esac
done

# Resolve org: flag > env > .sentryclirc
if [[ -z "$ORG" ]]; then
    ORG="${SENTRY_ORG:-}"
fi
if [[ -z "$ORG" ]]; then
    for rc in .sentryclirc ~/.sentryclirc; do
        if [[ -f "$rc" ]]; then
            ORG=$(grep -E '^\s*org\s*=' "$rc" 2>/dev/null | head -1 | sed 's/.*=\s*//' | tr -d '[:space:]') || true
            [[ -n "$ORG" ]] && break
        fi
    done
fi

# Resolve project: flag > env > .sentryclirc
if [[ -z "$PROJECT" ]]; then
    PROJECT="${SENTRY_PROJECT:-}"
fi
if [[ -z "$PROJECT" ]]; then
    for rc in .sentryclirc ~/.sentryclirc; do
        if [[ -f "$rc" ]]; then
            PROJECT=$(grep -E '^\s*project\s*=' "$rc" 2>/dev/null | head -1 | sed 's/.*=\s*//' | tr -d '[:space:]') || true
            [[ -n "$PROJECT" ]] && break
        fi
    done
fi

# Resolve auth token: env > .sentryclirc
TOKEN="${SENTRY_AUTH_TOKEN:-}"
if [[ -z "$TOKEN" ]]; then
    for rc in .sentryclirc ~/.sentryclirc; do
        if [[ -f "$rc" ]]; then
            TOKEN=$(grep -E '^\s*token\s*=' "$rc" 2>/dev/null | head -1 | sed 's/.*=\s*//' | tr -d '[:space:]') || true
            [[ -n "$TOKEN" ]] && break
        fi
    done
fi

# Validate
if [[ -z "$TOKEN" ]]; then
    echo "Error: No auth token found." >&2
    echo "Set SENTRY_AUTH_TOKEN or add token to .sentryclirc" >&2
    exit 1
fi

if [[ -z "$ORG" || -z "$PROJECT" ]]; then
    echo "Error: Organization and project are required." >&2
    echo "Use --org=<org> --project=<project>, set SENTRY_ORG/SENTRY_PROJECT, or configure .sentryclirc" >&2
    exit 2
fi

# Fetch issues
URL="https://sentry.io/api/0/projects/${ORG}/${PROJECT}/issues/?query=is:unresolved&statsPeriod=${STATS_PERIOD}&limit=${LIMIT}"

TMPFILE=$(mktemp /tmp/sentry-issues.XXXXXX.json)
trap 'rm -f "$TMPFILE"' EXIT

HTTP_CODE=$(curl -s -o "$TMPFILE" -w "%{http_code}" \
    -H "Authorization: Bearer $TOKEN" \
    "$URL")

if [[ "$HTTP_CODE" -ne 200 ]]; then
    echo "Error: API request failed with HTTP $HTTP_CODE" >&2
    cat "$TMPFILE" >&2
    exit 3
fi

cat "$TMPFILE"

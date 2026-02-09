#!/usr/bin/env bash
# sentry-api.sh - Authenticated Sentry API wrapper
#
# Sources lib.sh to resolve credentials from ~/.sentryclirc.
# Auto-replaces {org} and {project} placeholders in the API path.
#
# Usage:
#   sentry-api.sh METHOD PATH [BODY]
#
# Examples:
#   sentry-api.sh GET /issues/12345/
#   sentry-api.sh GET /issues/12345/events/latest/
#   sentry-api.sh PUT "/projects/{org}/{project}/issues/?id=123&id=456" '{"status":"resolved"}'
#
# Exit codes:
#   0 - Success (JSON on stdout)
#   1 - Missing auth token
#   2 - Missing org or project (path has placeholders but values not found)
#   3 - API request failed (HTTP 4xx/5xx)
#   4 - Prohibited method (DELETE)

set -euo pipefail

source "$(dirname "$0")/lib.sh"

BASE_URL="https://sentry.io/api/0"

# --- Args ---
METHOD="${1:-}"
PATH_ARG="${2:-}"
BODY="${3:-}"

if [[ -z "$METHOD" || -z "$PATH_ARG" ]]; then
    echo "Usage: $(basename "$0") METHOD PATH [BODY]" >&2
    echo "  METHOD: GET, PUT, POST" >&2
    echo "  PATH:   Sentry API path (e.g. /issues/12345/)" >&2
    echo "  BODY:   Optional JSON body for PUT/POST" >&2
    exit 1
fi

# Bash 3.2 compatibility: `${var^^}` is Bash 4+, so use `tr` instead.
METHOD="$(printf '%s' "$METHOD" | tr '[:lower:]' '[:upper:]')"

# --- Safety: reject DELETE ---
if [[ "$METHOD" == "DELETE" ]]; then
    echo "Error: DELETE is prohibited by safety rules." >&2
    exit 4
fi

# --- Resolve credentials: env > ~/.sentryclirc ---
TOKEN="${SENTRY_AUTH_TOKEN:-$(rc_value auth token)}"
ORG="${SENTRY_ORG:-$(rc_value defaults org)}"
PROJECT="${SENTRY_PROJECT:-$(rc_value defaults project)}"

if [[ -z "$TOKEN" ]]; then
    echo "Error: No auth token found." >&2
    echo "Set SENTRY_AUTH_TOKEN or add token to ~/.sentryclirc [auth] section." >&2
    exit 1
fi

# --- Substitute {org} and {project} placeholders ---
if [[ "$PATH_ARG" == *"{org}"* || "$PATH_ARG" == *"{project}"* ]]; then
    if [[ -z "$ORG" || -z "$PROJECT" ]]; then
        echo "Error: Path contains {org}/{project} placeholders but values not found." >&2
        echo "Set SENTRY_ORG/SENTRY_PROJECT or configure ~/.sentryclirc [defaults]." >&2
        exit 2
    fi
    PATH_ARG="${PATH_ARG//\{org\}/$ORG}"
    PATH_ARG="${PATH_ARG//\{project\}/$PROJECT}"
fi

# --- Build curl command ---
URL="${BASE_URL}${PATH_ARG}"

TMPFILE=$(mktemp "${TMPDIR:-/tmp}/sentry-api.XXXXXX")
trap 'rm -f "$TMPFILE"' EXIT

CURL_ARGS=(
    -s
    -o "$TMPFILE"
    -w "%{http_code}"
    -H "Authorization: Bearer $TOKEN"
)

if [[ "$METHOD" != "GET" ]]; then
    CURL_ARGS+=(-X "$METHOD")
fi

if [[ -n "$BODY" ]]; then
    CURL_ARGS+=(-H "Content-Type: application/json" -d "$BODY")
fi

CURL_ARGS+=("$URL")

# --- Execute ---
HTTP_CODE=$(curl "${CURL_ARGS[@]}")

if [[ "$HTTP_CODE" -ge 400 ]]; then
    echo "Error: HTTP $HTTP_CODE from $METHOD $PATH_ARG" >&2
    cat "$TMPFILE" >&2
    exit 3
fi

cat "$TMPFILE"

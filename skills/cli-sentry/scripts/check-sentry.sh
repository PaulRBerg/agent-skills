#!/usr/bin/env bash
# check-sentry.sh - Validate sentry-cli availability and authentication
#
# Exit codes:
#   0 - sentry-cli is ready (installed and authenticated)
#   1 - sentry-cli is not installed
#   2 - sentry-cli is not responding (timeout)
#   3 - sentry-cli is not authenticated
#   4 - missing required env vars (SENTRY_AUTH_TOKEN, SENTRY_ORG, SENTRY_PROJECT)
#
# Options:
#   -v, --verbose   Show detailed status
#   -q, --quiet     Suppress output on success
#   --skip-auth     Skip authentication check

set -euo pipefail

if ((BASH_VERSINFO[0] < 4)); then
    echo "ERROR: bash 4+ required (found ${BASH_VERSION})" >&2
    exit 1
fi

VERBOSE=0
QUIET=0
SKIP_AUTH=0
TIMEOUT_SECS=5

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Validate sentry-cli availability and authentication.

Options:
  -v, --verbose   Show detailed status information
  -q, --quiet     Suppress output on success (exit code only)
  --skip-auth     Skip authentication check
  -h, --help      Show this help message

Exit codes:
  0  sentry-cli is ready
  1  sentry-cli is not installed
  2  sentry-cli is not responding
  3  sentry-cli is not authenticated
  4  missing required env vars
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        -v|--verbose) VERBOSE=1; shift ;;
        -q|--quiet) QUIET=1; shift ;;
        --skip-auth) SKIP_AUTH=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown option: $1" >&2; usage >&2; exit 1 ;;
    esac
done

log() {
    [[ $QUIET -eq 0 ]] && echo "$@" || true
}

log_verbose() {
    [[ $VERBOSE -eq 1 ]] && echo "$@" || true
}

log_error() {
    echo "$@" >&2
}

# Check 0: Required environment variables
declare -A REQUIRED_VARS=(
    [SENTRY_AUTH_TOKEN]="sntrys_..."
    [SENTRY_ORG]="<your-org-slug>"
    [SENTRY_PROJECT]="<your-project-slug>"
)
# bash associative arrays don't preserve insertion order
REQUIRED_VAR_ORDER=(SENTRY_AUTH_TOKEN SENTRY_ORG SENTRY_PROJECT)

MISSING_VARS=()
for var in "${REQUIRED_VAR_ORDER[@]}"; do
    if [[ -z "${!var:-}" ]]; then
        MISSING_VARS+=("$var")
    fi
done

if [[ ${#MISSING_VARS[@]} -gt 0 ]]; then
    log_error "ERROR: Missing required environment variable(s): ${MISSING_VARS[*]}"
    log_error ""
    log_error "Add them to your project's .envrc file:"
    log_error ""
    for var in "${MISSING_VARS[@]}"; do
        log_error "  export ${var}=${REQUIRED_VARS[$var]}"
    done
    log_error ""
    log_error "Then run: direnv allow"
    log_error ""
    log_error "Generate an auth token at: https://sentry.io/settings/account/api/auth-tokens/"
    log_error "Find your project ID in Sentry under Settings > Projects > <project> > General"
    exit 4
fi

for var in "${REQUIRED_VAR_ORDER[@]}"; do
    if [[ "$var" == "SENTRY_AUTH_TOKEN" ]]; then
        log_verbose "SENTRY_AUTH_TOKEN: set"
    else
        log_verbose "${var}: ${!var}"
    fi
done

# Check 1: Is sentry-cli installed?
if ! command -v sentry-cli &>/dev/null; then
    log_error "ERROR: sentry-cli is not installed or not in PATH."
    log_error ""
    log_error "Install sentry-cli:"
    log_error "  brew install getsentry/tools/sentry-cli"
    log_error ""
    log_error "Or via npm:"
    log_error "  npm install -g @sentry/cli"
    log_error ""
    log_error "Or via curl:"
    log_error "  curl -sL https://sentry.io/get-cli/ | bash"
    log_error ""
    log_error "Documentation: https://docs.sentry.io/cli/"
    exit 1
fi

log_verbose "Found: $(command -v sentry-cli)"

# Check 2: Is sentry-cli responding?
if ! version=$(timeout "$TIMEOUT_SECS" sentry-cli --version 2>/dev/null); then
    log_error "ERROR: sentry-cli is installed but not responding (timeout after ${TIMEOUT_SECS}s)"
    log_error "Try running 'sentry-cli --version' manually to diagnose."
    exit 2
fi

log_verbose "Version: $version"

# Check 3: Is sentry-cli authenticated?
if [[ $SKIP_AUTH -eq 0 ]]; then
    if ! auth_info=$(timeout "$TIMEOUT_SECS" sentry-cli info 2>&1); then
        log_error "ERROR: sentry-cli is not authenticated."
        log_error ""
        log_error "Set up authentication:"
        log_error "  export SENTRY_AUTH_TOKEN=sntrys_..."
        log_error ""
        log_error "Or create a .sentryclirc file:"
        log_error "  [auth]"
        log_error "  token=sntrys_..."
        log_error ""
        log_error "Generate a token at: https://sentry.io/settings/account/api/auth-tokens/"
        exit 3
    fi

    if echo "$auth_info" | grep -Eqi "not logged in|no auth|error|unauthorized"; then
        log_error "ERROR: sentry-cli is not authenticated."
        log_error ""
        log_error "Set up authentication:"
        log_error "  export SENTRY_AUTH_TOKEN=sntrys_..."
        log_error ""
        log_error "Generate a token at: https://sentry.io/settings/account/api/auth-tokens/"
        exit 3
    fi

    log_verbose "Auth: authenticated"
fi

# Success
if [[ $VERBOSE -eq 1 ]]; then
    log "sentry-cli ready ($version)"
elif [[ $QUIET -eq 0 ]]; then
    log "sentry-cli ready"
fi

exit 0

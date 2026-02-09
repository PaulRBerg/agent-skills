#!/usr/bin/env bash
# lib.sh - Shared helpers for cli-sentry scripts
#
# Usage: source this file from sibling scripts
#   source "$(dirname "$0")/lib.sh"

# Read a value from ~/.sentryclirc within a specific INI section.
# Returns empty string if section/key not found or file missing.
#
# Usage: val=$(rc_value "auth" "token")
#        val=$(rc_value "defaults" "org")
rc_value() {
    local section="$1" key="$2"
    local rc="$HOME/.sentryclirc"
    [[ -f "$rc" ]] || return 0
    awk -v section="$section" -v key="$key" '
        /^\[/ { in_section = ($0 ~ "^\\[" section "\\]"); next }
        in_section && $0 ~ "^[[:space:]]*" key "[[:space:]]*=" {
            sub(/.*=[[:space:]]*/, ""); gsub(/[[:space:]]/, ""); print; exit
        }
    ' "$rc"
}

#!/usr/bin/env bash
# run-taze.sh - Run taze in non-interactive mode
#
# Usage: run-taze.sh [--include pkg1,pkg2] [path]
#
# Automatically detects monorepo projects (workspaces in package.json
# or pnpm-workspace.yaml) and enables recursive mode.
#
# Bun projects with bunfig.toml minimumReleaseAge get matching Taze
# maturity-period flags. Taze does not auto-infer Bun's age gate.
#
# Exit codes:
#   0 - Success (updates displayed)
#   1 - taze not installed
#   2 - No package.json found

set -euo pipefail

include=""
if [[ "${1:-}" == "--include" ]]; then
    include="$2"
    shift 2
fi

target_dir="${1:-.}"

# Check for package.json
if [[ ! -f "$target_dir/package.json" ]]; then
    echo "ERROR: No package.json found in $target_dir" >&2
    exit 2
fi

cd "$target_dir"

# Auto-detect monorepo
recursive=""
if grep -q '"workspaces"' package.json 2>/dev/null \
    || [[ -f pnpm-workspace.yaml ]]; then
    recursive="-r"
fi

# Check taze availability
if ! command -v taze &>/dev/null; then
    cat >&2 <<'EOF'
ERROR: taze CLI is not installed.

Install taze globally:
  npm install -g taze

Or run via npx:
  npx taze

Documentation: https://github.com/antfu-collective/taze
EOF
    exit 1
fi

# Build include flag
include_flag=""
if [[ -n "$include" ]]; then
    include_flag="--include $include"
fi

# Mirror Bun's delayed-resolution policy for direct dependency candidates.
# bunfig.toml stores seconds; Taze expects whole days.
maturity_flag=""
maturity_exclude_flag=""
if [[ -f bunfig.toml ]] && [[ -f bun.lock || -f bun.lockb ]]; then
    minimum_release_age="$(sed -nE 's/^[[:space:]]*minimumReleaseAge[[:space:]]*=[[:space:]]*([0-9]+).*$/\1/p' bunfig.toml | head -n 1)"
    if [[ -n "$minimum_release_age" && "$minimum_release_age" != "0" ]]; then
        maturity_days=$(( (minimum_release_age + 86399) / 86400 ))
        maturity_flag="--maturity-period $maturity_days"
    fi

    if taze --help 2>/dev/null | grep -q -- '--maturity-period-exclude'; then
        maturity_excludes="$(sed -nE 's/^[[:space:]]*minimumReleaseAgeExcludes[[:space:]]*=[[:space:]]*\[(.*)\].*$/\1/p' bunfig.toml | head -n 1 | tr -d '[:space:]"')"
        if [[ -n "$maturity_excludes" ]]; then
            maturity_exclude_flag="--maturity-period-exclude $maturity_excludes"
        fi
    fi
fi

# Run taze major to show ALL available updates (including breaking)
# -l/--include-locked shows fixed versions (no ^ or ~)
# shellcheck disable=SC2086
taze major $recursive $include_flag $maturity_flag $maturity_exclude_flag --include-locked 2>&1

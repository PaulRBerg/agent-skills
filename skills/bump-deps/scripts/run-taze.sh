#!/usr/bin/env bash
# run-taze.sh - Run taze in non-interactive mode
#
# Usage: run-taze.sh [--include pkg1,pkg2] [path]
#
# Automatically detects monorepo projects (workspaces in package.json
# or pnpm-workspace.yaml) and enables recursive mode.
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

# Build include flag
include_flag=""
if [[ -n "$include" ]]; then
    include_flag="--include $include"
fi

# Run taze major to show ALL available updates (including breaking)
# -l/--include-locked shows fixed versions (no ^ or ~)
# shellcheck disable=SC2086
taze major $recursive $include_flag --include-locked 2>&1

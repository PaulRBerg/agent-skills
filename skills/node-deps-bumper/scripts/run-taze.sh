#!/usr/bin/env bash
# run-taze.sh - Run taze in non-interactive mode
#
# Usage: run-taze.sh [--include pkg1,pkg2] [--concurrency n] [--plan|--write] [path]
#
# Automatically detects monorepo projects (workspaces in package.json
# or pnpm-workspace.yaml) and enables recursive mode.
#
# Bun projects with local or global minimumReleaseAge get matching Taze
# maturity-period flags. Taze does not auto-infer Bun's age gate.
#
# Exit codes:
#   0 - Success (updates displayed)
#   1 - taze not installed
#   2 - No package.json found
#   64 - Usage error

set -euo pipefail

include=""
concurrency=""
write=false
plan=false
target_dir="."
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"

while [[ $# -gt 0 ]]; do
  case "$1" in
  --include)
    include="${2:?ERROR: --include requires a value}"
    shift 2
    ;;
  --include=*)
    include="${1#*=}"
    shift
    ;;
  --concurrency)
    concurrency="${2:?ERROR: --concurrency requires a value}"
    shift 2
    ;;
  --concurrency=*)
    concurrency="${1#*=}"
    shift
    ;;
  --write)
    write=true
    shift
    ;;
  --plan)
    plan=true
    shift
    ;;
  -*)
    echo "ERROR: Unknown option: $1" >&2
    exit 64
    ;;
  *)
    if [[ "$target_dir" != "." ]]; then
      echo "ERROR: Only one target path is supported" >&2
      exit 64
    fi
    target_dir="$1"
    shift
    ;;
  esac
done

# Check for package.json
if [[ ! -f "$target_dir/package.json" ]]; then
  echo "ERROR: No package.json found in $target_dir" >&2
  exit 2
fi

cd "$target_dir"

# Auto-detect monorepo
taze_args=(major)
if grep -q '"workspaces"' package.json 2>/dev/null ||
  [[ -f pnpm-workspace.yaml ]]; then
  taze_args+=("-r")
fi

# Check taze availability
if ! command -v taze &>/dev/null; then
  cat >&2 <<'EOF'
ERROR: taze CLI is not installed.

Install taze globally:
  npm install -g taze

Or run via bunx:
  bunx taze

Documentation: https://github.com/antfu-collective/taze
EOF
  exit 1
fi

if [[ -n "$include" ]]; then
  taze_args+=("--include" "$include")
fi

if [[ -n "$concurrency" ]]; then
  taze_args+=("--concurrency" "$concurrency")
fi

if [[ "$write" == true && -z "$include" ]]; then
  echo "ERROR: --write requires --include with the selected package list" >&2
  exit 64
fi

if [[ "$write" == true && "$plan" == true ]]; then
  echo "ERROR: --plan and --write are mutually exclusive" >&2
  exit 64
fi

# Mirror Bun's delayed-resolution policy for direct dependency candidates.
# bunfig.toml stores seconds; Taze expects whole days.
if [[ -f bun.lock || -f bun.lockb ]]; then
  policy_args=()
  if taze --help 2>/dev/null | grep -q -- '--maturity-period-exclude'; then
    policy_args+=("--exclude-supported")
  fi
  # Parse TOML rather than lines: lists may span lines and local keys override global keys.
  maturity_args="$(uv run "$script_dir/bun-maturity.py" ${policy_args[@]+"${policy_args[@]}"})"
  if [[ -n "$maturity_args" ]]; then
    while IFS= read -r argument; do
      taze_args+=("$argument")
    done <<<"$maturity_args"
  fi
fi

if [[ "$write" == true ]]; then
  taze_args+=("--write")
else
  # Scan all available updates, including fixed versions (no ^ or ~).
  taze_args+=("--include-locked")
fi

if [[ "$plan" == true ]]; then
  plan_output="$(mktemp "${TMPDIR:-/tmp}/taze-plan.XXXXXX")"
  trap 'rm -f "$plan_output"' EXIT
  NO_COLOR=1 taze "${taze_args[@]}" --no-group --no-timediff --no-nodecompat --sort name-asc >"$plan_output" 2>&1
  uv run "$script_dir/parse-taze-plan.py" --input "$plan_output"
else
  taze "${taze_args[@]}" 2>&1
fi

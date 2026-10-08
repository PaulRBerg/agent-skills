#!/bin/bash
# List model identifier mentions in catalog and internal skills as sorted, unique file:line:text rows.
# Run after a model release, rename, or retirement to find every file that needs the same update.
set -eu

repo_root=$(cd "$(dirname "$0")/.." && pwd -P)
cd "$repo_root"

# Alias-heavy routing files also name bare model aliases (Luna/high, sol_medium) and the router model.
alias_files='skills/orchestration/scripts/select-model.py skills/orchestration/references/jev-routing.md'

run_rg() {
  set +e
  rg --no-config --line-number --no-heading --with-filename --ignore-case \
    --max-columns 200 --max-columns-preview \
    --glob '!**/node_modules/**' --glob '!**/.cache/**' --glob '!**/tests/**' --glob '!.agents/internal-skills/sync-skills.md' \
    "$@"
  rc=$?
  set -e
  if [ "$rc" -gt 1 ]; then
    echo "model-refs: rg failed with exit code $rc" >&2
    exit "$rc"
  fi
}

{
  run_rg \
    -e 'claude-(opus|sonnet|haiku|fable|mythos|[0-9])[a-z0-9.-]*' \
    -e '\b(opus|sonnet|haiku|fable|mythos)\b( ?[0-9]+(\.[0-9]+)*)?' \
    -e '\bgpt-[0-9][a-z0-9.-]*' \
    -e '\bgpt[- ]?[0-9.]*[ -](luna|sol|astra)\b' \
    -e '\b(luna|sol|astra)[ -]gpt\b' \
    skills .agents/internal-skills
  # shellcheck disable=SC2086 # alias_files is a fixed, space-separated path list.
  run_rg -e '\b(luna|sol|astra)\b' -e 'typesafe-ai/jev' $alias_files
} | LC_ALL=C sort -t: -k1,1 -k2,2n -u

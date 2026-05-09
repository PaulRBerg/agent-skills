#!/bin/bash
# debrief: probe playground skill, validate slug, create findings directory.
# Usage: prepare.sh <slug>
# Stdout (KEY=VALUE per line):
#   PLAYGROUND_DIR  absolute path to playground skill root
#   FINDINGS_DIR    absolute path to ./.agents/findings/<slug>
#   REPORT_PATH     absolute path to ./.agents/findings/<slug>/index.html
#   EXISTS          true if REPORT_PATH already exists, otherwise false
# Exit codes:
#   0  ok
#   2  playground skill not installed
#   3  invalid or missing slug

set -euo pipefail

SLUG="${1:-}"

if [ -z "$SLUG" ]; then
  echo "Error: slug argument required" >&2
  echo "Usage: prepare.sh <kebab-case-slug>" >&2
  exit 3
fi

case "$SLUG" in
  *[!a-z0-9-]*|-*|*-)
    echo "Error: slug must be kebab-case (lowercase letters, digits, dashes; no leading/trailing dash): $SLUG" >&2
    exit 3
    ;;
esac

PLAYGROUND_DIR=""
for candidate in \
  ".agents/skills/playground" \
  ".claude/skills/playground" \
  "$HOME/.agents/skills/playground" \
  "$HOME/.claude/skills/playground"; do
  if [ -f "$candidate/SKILL.md" ]; then
    PLAYGROUND_DIR=$(cd "$candidate" && pwd -P)
    break
  fi
done

if [ -z "$PLAYGROUND_DIR" ]; then
  cat >&2 <<'EOF'
Error: the `playground` skill is not installed.

Install it with:
  npx skills add anthropics/skills --skill playground --global

After installing, retry this skill.
EOF
  exit 2
fi

FINDINGS_DIR="$(pwd)/.agents/findings/$SLUG"
mkdir -p "$FINDINGS_DIR"
REPORT_PATH="$FINDINGS_DIR/index.html"

if [ -f "$REPORT_PATH" ]; then
  EXISTS="true"
else
  EXISTS="false"
fi

printf 'PLAYGROUND_DIR=%s\n' "$PLAYGROUND_DIR"
printf 'FINDINGS_DIR=%s\n'   "$FINDINGS_DIR"
printf 'REPORT_PATH=%s\n'    "$REPORT_PATH"
printf 'EXISTS=%s\n'         "$EXISTS"

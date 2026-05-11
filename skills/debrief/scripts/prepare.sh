#!/bin/bash
# debrief: validate slug, optionally probe playground skill, create reports directory.
# Usage: prepare.sh [--md] <slug>
# Stdout (KEY=VALUE per line):
#   MODE            "html" (default) or "md"
#   PLAYGROUND_DIR  absolute path to playground skill root (empty in --md mode)
#   REPORTS_DIR     absolute path to ./.ai/reports/<slug>
#   REPORT_PATH     absolute path to ./.ai/reports/<slug>/index.{html,md}
#   EXISTS          true if REPORT_PATH already exists, otherwise false
# Exit codes:
#   0  ok
#   2  playground skill not installed (html mode only)
#   3  invalid arguments or missing/bad slug

set -euo pipefail

MODE="html"
SLUG=""
for arg in "$@"; do
  case "$arg" in
    --md)
      MODE="md"
      ;;
    -*)
      echo "Error: unknown flag: $arg" >&2
      echo "Usage: prepare.sh [--md] <kebab-case-slug>" >&2
      exit 3
      ;;
    *)
      if [ -n "$SLUG" ]; then
        echo "Error: unexpected extra argument: $arg" >&2
        exit 3
      fi
      SLUG="$arg"
      ;;
  esac
done

if [ -z "$SLUG" ]; then
  echo "Error: slug argument required" >&2
  echo "Usage: prepare.sh [--md] <kebab-case-slug>" >&2
  exit 3
fi

case "$SLUG" in
  *[!a-z0-9-]*|-*|*-)
    echo "Error: slug must be kebab-case (lowercase letters, digits, dashes; no leading/trailing dash): $SLUG" >&2
    exit 3
    ;;
esac

PLAYGROUND_DIR=""
if [ "$MODE" = "html" ]; then
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
fi

REPORTS_DIR="$(pwd)/.ai/reports/$SLUG"
mkdir -p "$REPORTS_DIR"
if [ "$MODE" = "md" ]; then
  REPORT_PATH="$REPORTS_DIR/index.md"
else
  REPORT_PATH="$REPORTS_DIR/index.html"
fi

if [ -f "$REPORT_PATH" ]; then
  EXISTS="true"
else
  EXISTS="false"
fi

printf 'MODE=%s\n'           "$MODE"
printf 'PLAYGROUND_DIR=%s\n' "$PLAYGROUND_DIR"
printf 'REPORTS_DIR=%s\n'    "$REPORTS_DIR"
printf 'REPORT_PATH=%s\n'    "$REPORT_PATH"
printf 'EXISTS=%s\n'         "$EXISTS"

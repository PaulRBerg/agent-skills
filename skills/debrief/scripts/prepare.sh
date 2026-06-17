#!/bin/bash
# debrief: validate slug, optionally probe playground skill, create debriefs directory.
# Usage: prepare.sh [--md] <slug>
# Stdout (KEY=VALUE per line):
#   MODE            "html" (default) or "md"
#   PLAYGROUND_DIR  absolute path to playground skill root (empty in --md mode)
#   DEBRIEFS_DIR    absolute path to ./.ai/debriefs/<slug>
#   DEBRIEF_PATH    absolute path to ./.ai/debriefs/<slug>/index.{html,md}
#   EXISTS          true if DEBRIEF_PATH already exists, otherwise false
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
  bunx skills add anthropics/skills --skill playground --global

After installing, retry this skill.
EOF
    exit 2
  fi
fi

DEBRIEFS_DIR="$(pwd)/.ai/debriefs/$SLUG"
mkdir -p "$DEBRIEFS_DIR"
if [ "$MODE" = "md" ]; then
  DEBRIEF_PATH="$DEBRIEFS_DIR/index.md"
else
  DEBRIEF_PATH="$DEBRIEFS_DIR/index.html"
fi

if [ -f "$DEBRIEF_PATH" ]; then
  EXISTS="true"
else
  EXISTS="false"
fi

printf 'MODE=%s\n'           "$MODE"
printf 'PLAYGROUND_DIR=%s\n' "$PLAYGROUND_DIR"
printf 'DEBRIEFS_DIR=%s\n'   "$DEBRIEFS_DIR"
printf 'DEBRIEF_PATH=%s\n'   "$DEBRIEF_PATH"
printf 'EXISTS=%s\n'         "$EXISTS"

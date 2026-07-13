#!/bin/bash
# html-debrief: validate slug, probe html-playground skill, create debriefs directory.
# Usage: prepare.sh <slug>
# Stdout (KEY=VALUE per line):
#   PLAYGROUND_DIR  absolute path to html-playground skill root
#   DEBRIEFS_DIR    absolute path to ./.ai/debriefs/<slug>
#   DEBRIEF_PATH    absolute path to ./.ai/debriefs/<slug>/index.html
#   EXISTS          true if DEBRIEF_PATH already exists, otherwise false
# Exit codes:
#   0  ok
#   2  html-playground skill not installed
#   3  invalid arguments or missing/bad slug

set -euo pipefail

SLUG=""
for arg in "$@"; do
  case "$arg" in
    -*)
      echo "Error: unknown flag: $arg" >&2
      echo "Usage: prepare.sh <kebab-case-slug>" >&2
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
  ".agents/skills/html-playground" \
  ".claude/skills/html-playground" \
  "$HOME/.agents/skills/html-playground" \
  "$HOME/.claude/skills/html-playground"; do
  if [ -f "$candidate/SKILL.md" ]; then
    PLAYGROUND_DIR=$(cd "$candidate" && pwd -P)
    break
  fi
done

if [ -z "$PLAYGROUND_DIR" ]; then
  cat >&2 <<'EOF'
Error: the `html-playground` skill is not installed.

Install it with:
  bunx skills add PaulRBerg/agent-skills --skill html-playground --global

After installing, retry this skill.
EOF
  exit 2
fi

DEBRIEFS_DIR="$(pwd)/.ai/debriefs/$SLUG"
mkdir -p "$DEBRIEFS_DIR"
DEBRIEF_PATH="$DEBRIEFS_DIR/index.html"

if [ -f "$DEBRIEF_PATH" ]; then
  EXISTS="true"
else
  EXISTS="false"
fi

printf 'PLAYGROUND_DIR=%s\n' "$PLAYGROUND_DIR"
printf 'DEBRIEFS_DIR=%s\n'   "$DEBRIEFS_DIR"
printf 'DEBRIEF_PATH=%s\n'   "$DEBRIEF_PATH"
printf 'EXISTS=%s\n'         "$EXISTS"

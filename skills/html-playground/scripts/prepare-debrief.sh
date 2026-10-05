#!/bin/bash
# html-playground debrief mode: validate slug and create the debrief directory.
# Usage: prepare-debrief.sh <slug>
# Stdout (KEY=VALUE per line):
#   DEBRIEFS_DIR    absolute path to ./.ai/debriefs/<slug>
#   DEBRIEF_PATH    absolute path to ./.ai/debriefs/<slug>/index.html
#   EXISTS          true if DEBRIEF_PATH already exists, otherwise false
# Exit codes:
#   0  ok
#   3  invalid arguments or missing/bad slug

set -euo pipefail

SLUG=""
for arg in "$@"; do
  case "$arg" in
    -*)
      echo "Error: unknown flag: $arg" >&2
      echo "Usage: prepare-debrief.sh <kebab-case-slug>" >&2
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
  echo "Usage: prepare-debrief.sh <kebab-case-slug>" >&2
  exit 3
fi

case "$SLUG" in
  *[!a-z0-9-]*|-*|*-)
    echo "Error: slug must be kebab-case (lowercase letters, digits, dashes; no leading/trailing dash): $SLUG" >&2
    exit 3
    ;;
esac

DEBRIEFS_DIR="$(pwd)/.ai/debriefs/$SLUG"
mkdir -p "$DEBRIEFS_DIR"
DEBRIEF_PATH="$DEBRIEFS_DIR/index.html"

if [ -f "$DEBRIEF_PATH" ]; then
  EXISTS="true"
else
  EXISTS="false"
fi

printf 'DEBRIEFS_DIR=%s\n' "$DEBRIEFS_DIR"
printf 'DEBRIEF_PATH=%s\n' "$DEBRIEF_PATH"
printf 'EXISTS=%s\n'       "$EXISTS"

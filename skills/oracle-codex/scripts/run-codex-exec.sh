#!/usr/bin/env bash
# run-codex-exec.sh - Safe wrapper for codex exec with flag detection
#
# Reads prompt from stdin, writes the last message to CODEX_OUTPUT if set,
# and prints the output to stdout. If CODEX_OUTPUT is unset, uses a temp file.
#
# Environment variables:
#   MODEL         - Codex model (default: gpt-5.2-codex)
#   EFFORT        - Reasoning effort (default: medium)
#   SANDBOX       - Sandbox mode (default: read-only)
#   CODEX_SEARCH  - Set to 1 to enable web search
#   CODEX_OUTPUT  - Output file path (optional)

set -euo pipefail

MODEL="${MODEL:-gpt-5.2-codex}"
EFFORT="${EFFORT:-medium}"
SANDBOX="${SANDBOX:-read-only}"
CODEX_SEARCH="${CODEX_SEARCH:-0}"

CODEX_BIN="$(command -v codex || true)"
if [[ -z "$CODEX_BIN" ]]; then
  echo "ERROR: codex not found in PATH." >&2
  exit 1
fi

HELP_OUT="$($CODEX_BIN exec --help 2>/dev/null || true)"
if [[ -z "$HELP_OUT" ]]; then
  echo "ERROR: failed to read codex exec help output." >&2
  exit 1
fi

SKIP_FLAG=""
if echo "$HELP_OUT" | grep -q -- "--skip-git-repo-check"; then
  SKIP_FLAG="--skip-git-repo-check"
fi

OUTPUT_FLAG=""
if echo "$HELP_OUT" | grep -q -- "--output-last-message"; then
  OUTPUT_FLAG="--output-last-message"
fi

PROMPT_FILE="$(mktemp -t codex.prompt.XXXXXX)"
ERR_FILE="$(mktemp -t codex.err.XXXXXX)"
OUT_FILE="${CODEX_OUTPUT:-}"
OUT_FILE_TMP=0
if [[ -z "$OUT_FILE" ]]; then
  OUT_FILE="$(mktemp -t codex.out.XXXXXX)"
  OUT_FILE_TMP=1
fi

cleanup() {
  rm -f "$PROMPT_FILE" "$ERR_FILE"
}
trap cleanup EXIT

cat >"$PROMPT_FILE"
if [[ ! -s "$PROMPT_FILE" ]]; then
  echo "ERROR: empty prompt. Provide instructions via stdin." >&2
  exit 1
fi

cmd=(
  "$CODEX_BIN" exec
  -m "$MODEL"
  -c "model_reasoning_effort=$EFFORT"
  -s "$SANDBOX"
)
if [[ -n "$SKIP_FLAG" ]]; then
  cmd+=("$SKIP_FLAG")
fi
if [[ "$CODEX_SEARCH" == "1" ]]; then
  cmd+=("--search")
fi
if [[ -n "$OUTPUT_FLAG" ]]; then
  cmd+=("$OUTPUT_FLAG" "$OUT_FILE")
fi

if [[ -n "$OUTPUT_FLAG" ]]; then
  if ! "${cmd[@]}" <"$PROMPT_FILE" >/dev/null 2>"$ERR_FILE"; then
    cat "$ERR_FILE" >&2
    exit 2
  fi
else
  if ! "${cmd[@]}" <"$PROMPT_FILE" >"$OUT_FILE" 2>"$ERR_FILE"; then
    cat "$ERR_FILE" >&2
    exit 2
  fi
fi

if [[ $OUT_FILE_TMP -eq 1 ]]; then
  cat "$OUT_FILE"
  rm -f "$OUT_FILE"
fi

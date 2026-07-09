#!/usr/bin/env bash
# Human-in-the-loop reproduction template.
# Copy this file, replace the sample steps, then run the copy with Bash.

set -euo pipefail

step() {
  printf '\n>>> %s\n' "$1"
  read -r -p "    [Enter when done] " _
}

capture() {
  local var="$1"
  local question="$2"
  local answer

  printf '\n>>> %s\n' "$question"
  read -r -p "    > " answer
  printf -v "$var" '%s' "$answer"
}

# --- edit below ---------------------------------------------------------

step "Open the app at http://localhost:3000 and sign in."

capture ERRORED "Trigger the suspected bug. Did the reported symptom appear? (y/n)"

capture SYMPTOM "Paste the exact error, wrong output, or timing you observed:"

# --- edit above ---------------------------------------------------------

printf '\n--- Captured ---\n'
printf 'ERRORED=%s\n' "$ERRORED"
printf 'SYMPTOM=%s\n' "$SYMPTOM"

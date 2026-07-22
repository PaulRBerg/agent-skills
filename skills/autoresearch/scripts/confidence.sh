#!/bin/bash
# Compatibility wrapper for MAD confidence output.

set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd -P)"
exec python3 "$script_dir/autoresearch-session.py" status \
  --file "${1:-autoresearch.jsonl}" \
  --format confidence

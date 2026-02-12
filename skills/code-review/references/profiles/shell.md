# Shell Profile

Load for shell scripts and shell-heavy CI blocks.

## Checks

- `SH-001` Unquoted expansion (`HIGH`): word-splitting/globbing can alter command behavior.
- `SH-002` Command injection (`CRITICAL`): `eval` or string-built commands with untrusted input.
- `SH-003` Error masking (`HIGH`): missing `set -euo pipefail` or unchecked critical commands.
- `SH-004` Tempfile race/leak (`MEDIUM`): insecure temp paths or missing cleanup traps.
- `SH-005` Portability mismatch (`LOW`): shebang/syntax mismatch for target shell.
- `SH-006` Secret leakage (`HIGH`): credentials exposed in args, logs, or traces.

## Evidence Expectations

- Show exact expansion/injection vector and resulting command behavior.

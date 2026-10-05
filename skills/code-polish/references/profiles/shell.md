# Shell Profile

Load when the diff touches shell scripts or shell-heavy CI blocks.

## Checks

- `SH-001` Unquoted expansion (`HIGH`): Word-splitting/globbing can alter command behavior.
- `SH-002` Command injection (`CRITICAL`): Code uses `eval` or string-built commands with untrusted input.
- `SH-003` Error masking (`HIGH`): Code lacks `set -euo pipefail` or does not check critical commands.
- `SH-004` Tempfile race/leak (`MEDIUM`): Temporary paths are insecure, or cleanup traps are missing.
- `SH-005` Portability mismatch (`LOW`): The shebang/syntax does not match the target shell.
- `SH-006` Secret leakage (`HIGH`): Args, logs, or traces expose credentials.

## Evidence Expectations

- Show the exact expansion/injection vector and the resulting command behavior.

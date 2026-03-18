#!/usr/bin/env bash
# kill-zombies.sh - Detect and reap zombie processes using procs
#
# Usage: kill-zombies.sh [--kill]
#
# Modes:
#   (default)  Detect-only — list zombie processes
#   --kill     Send SIGCHLD to parent processes to trigger reaping
#
# Exit codes:
#   0 - Success (no zombies, or zombies listed/reaped)
#   1 - procs not installed
#   2 - python3 not available

set -euo pipefail

mode="detect"
if [ "${1:-}" = "--kill" ]; then
    mode="kill"
fi

# Check dependencies
if ! command -v procs >/dev/null 2>&1; then
    cat >&2 <<'EOF'
ERROR: procs is not installed.

Install with Homebrew:
  brew install procs

Or with Cargo:
  cargo install procs
EOF
    exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required but not found." >&2
    exit 2
fi

# Create temporary procs config with State and Ppid columns
config_file="$(mktemp /tmp/procs-zombie-XXXXXX.toml)"
trap 'rm -f "$config_file"' EXIT

cat > "$config_file" <<'TOML'
[[columns]]
kind = "Pid"
numeric_search = true
nonnumeric_search = false

[[columns]]
kind = "Ppid"
numeric_search = true
nonnumeric_search = false

[[columns]]
kind = "State"
numeric_search = false
nonnumeric_search = true

[[columns]]
kind = "User"
numeric_search = false
nonnumeric_search = true

[[columns]]
kind = "Command"
numeric_search = false
nonnumeric_search = true
TOML

# Query procs for all processes with state info
json_output="$(procs --load-config "$config_file" --json --color disable 2>&1)"

# Filter for zombies and optionally reap them
python3 -c "
import json, os, signal, sys, time

data = json.loads(sys.stdin.read())
zombies = [p for p in data if p.get('State') == 'Z']

if not zombies:
    print('No zombie processes found.')
    sys.exit(0)

print(f'Found {len(zombies)} zombie process(es):')
print()
print(f'{\"PID\":<10} {\"PPID\":<10} {\"User\":<15} Command')
print(f'{\"---\":<10} {\"----\":<10} {\"----\":<15} -------')
for z in zombies:
    pid = z.get('PID', '?')
    ppid = z.get('Parent PID', '?')
    user = z.get('User', '?')
    cmd = z.get('Command', '?')
    # Truncate long commands
    if len(str(cmd)) > 60:
        cmd = str(cmd)[:57] + '...'
    print(f'{pid:<10} {ppid:<10} {user:<15} {cmd}')

mode = '$mode'
if mode != 'kill':
    print()
    print('Run with --kill to send SIGCHLD to parent processes.')
    sys.exit(0)

print()
print('Sending SIGCHLD to parent processes...')
parent_pids = set()
for z in zombies:
    ppid = z.get('Parent PID')
    if ppid is not None and ppid > 1:
        parent_pids.add(ppid)

if not parent_pids:
    print('All zombie parents are PID 1 — cannot signal init/launchd.')
    sys.exit(0)

for ppid in sorted(parent_pids):
    try:
        os.kill(ppid, signal.SIGCHLD)
        print(f'  Sent SIGCHLD to PID {ppid}')
    except ProcessLookupError:
        print(f'  PID {ppid} no longer exists')
    except PermissionError:
        print(f'  Permission denied for PID {ppid} (try with sudo)')

# Brief pause then re-check
time.sleep(0.5)
recheck = json.loads(os.popen(
    'procs --load-config \"$config_file\" --json --color disable 2>/dev/null'
    .replace('\$config_file', '$config_file')
).read())
remaining = [p for p in recheck if p.get('State') == 'Z']

if not remaining:
    print()
    print('All zombies reaped successfully.')
else:
    print()
    print(f'{len(remaining)} zombie(s) remain — their parents may be ignoring SIGCHLD.')
    print('Parent PIDs that may need to be killed:')
    for z in remaining:
        ppid = z.get('Parent PID', '?')
        print(f'  PID {ppid}')
" <<< "$json_output"

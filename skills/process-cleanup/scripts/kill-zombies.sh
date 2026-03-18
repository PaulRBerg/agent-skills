#!/usr/bin/env bash
# kill-zombies.sh - Detect and reap zombie processes
#
# Usage: kill-zombies.sh [--kill]
#
# Modes:
#   (default)  Detect-only — list zombie processes
#   --kill     Send SIGCHLD to parent processes to trigger reaping
#
# Exit codes:
#   0 - Success (no zombies, or zombies listed/reaped)
#   1 - python3 not available

set -euo pipefail

mode="detect"
if [ "${1:-}" = "--kill" ]; then
    mode="kill"
fi

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required but not found." >&2
    exit 1
fi

# Use /bin/ps for reliable process state on all platforms (procs reports State
# as '?' on macOS, making zombie detection impossible).
ps_output="$(/bin/ps ax -o pid=,state=,ppid=,user=,command=)"

python3 -c "
import os, signal, sys, time

mode = '$mode'
lines = sys.stdin.read().strip().splitlines()
zombies = []

for line in lines:
    parts = line.split(None, 4)
    if len(parts) < 5:
        continue
    pid, state, ppid, user, cmd = parts[0], parts[1], parts[2], parts[3], parts[4]
    if state.startswith('Z'):
        zombies.append({
            'pid': int(pid),
            'ppid': int(ppid),
            'user': user,
            'cmd': cmd if len(cmd) <= 60 else cmd[:57] + '...',
        })

if not zombies:
    print('No zombie processes found.')
    sys.exit(0)

print(f'Found {len(zombies)} zombie process(es):')
print()
print(f'{\"PID\":<10} {\"PPID\":<10} {\"User\":<15} Command')
print(f'{\"---\":<10} {\"----\":<10} {\"----\":<15} -------')
for z in zombies:
    print(f'{z[\"pid\"]:<10} {z[\"ppid\"]:<10} {z[\"user\"]:<15} {z[\"cmd\"]}')

if mode != 'kill':
    print()
    print('Run with --kill to send SIGCHLD to parent processes.')
    sys.exit(0)

print()
print('Sending SIGCHLD to parent processes...')
parent_pids = {z['ppid'] for z in zombies if z['ppid'] > 1}

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
import subprocess
recheck = subprocess.run(
    ['/bin/ps', 'ax', '-o', 'pid=,state=,ppid=,user=,command='],
    capture_output=True, text=True
).stdout.strip().splitlines()

remaining = []
for line in recheck:
    parts = line.split(None, 4)
    if len(parts) >= 2 and parts[1].startswith('Z'):
        remaining.append(parts)

if not remaining:
    print()
    print('All zombies reaped successfully.')
else:
    print()
    print(f'{len(remaining)} zombie(s) remain — their parents may be ignoring SIGCHLD.')
    print('Parent PIDs that may need to be killed:')
    for parts in remaining:
        print(f'  PID {parts[2]}')
" <<< "$ps_output"

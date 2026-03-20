#!/bin/bash
# Print autoresearch experiment summary dashboard
# Usage: summary.sh [path/to/autoresearch.jsonl]
# Exit codes: 0 = success, 1 = no data, 2 = file not found

set -euo pipefail

JSONL_FILE="${1:-autoresearch.jsonl}"

if [ ! -f "$JSONL_FILE" ]; then
  echo "Error: $JSONL_FILE not found" >&2
  exit 2
fi

python3 -c "
import json, sys, datetime

jsonl_file = sys.argv[1]

results = []
with open(jsonl_file) as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        obj = json.loads(line)
        if 'status' not in obj:
            continue
        results.append(obj)

if not results:
    print('No experiment results found.')
    sys.exit(1)

# Find current segment
max_segment = max((r.get('segment', 0) for r in results), default=0)
segment_results = [r for r in results if r.get('segment', 0) == max_segment]

# Stats
total = len(segment_results)
kept = [r for r in segment_results if r['status'] == 'keep']
discarded = sum(1 for r in segment_results if r['status'] == 'discard')
crashed = sum(1 for r in segment_results if r['status'] == 'crash')
checks_failed = sum(1 for r in segment_results if r['status'] == 'checks_failed')

# Baseline and best
baseline = segment_results[0] if segment_results else None
baseline_metric = baseline['metric'] if baseline else 0

if kept:
    # Infer direction
    best_low = min(kept, key=lambda r: r['metric'])
    best_high = max(kept, key=lambda r: r['metric'])
    if best_low['metric'] < baseline_metric:
        best = best_low
        direction = 'lower'
    else:
        best = best_high
        direction = 'higher'
    best_metric = best['metric']

    if baseline_metric != 0:
        pct = ((best_metric - baseline_metric) / abs(baseline_metric)) * 100
        pct_str = f'{pct:+.2f}%'
    else:
        pct_str = 'N/A'
else:
    best_metric = baseline_metric
    direction = 'N/A'
    pct_str = 'N/A'

# Header
print('=' * 72)
print('AUTORESEARCH SUMMARY')
print('=' * 72)
print(f'Runs: {total} total | {len(kept)} kept | {discarded} discarded | {crashed} crashed | {checks_failed} checks_failed')
print(f'Baseline: {baseline_metric}')
print(f'Best:     {best_metric} ({pct_str}, {direction} is better)')
print('-' * 72)

# Table header
print(f'{\"#\":>4}  {\"Commit\":>7}  {\"Metric\":>12}  {\"Status\":<14}  Description')
print(f'{\"─\"*4}  {\"─\"*7}  {\"─\"*12}  {\"─\"*14}  {\"─\"*30}')

for r in segment_results:
    run = r.get('run', '?')
    commit = r.get('commit', '???????')[:7]
    metric = r['metric']
    status = r['status']
    desc = r.get('description', '')[:40]

    # Mark best with star
    marker = ''
    if kept and r.get('commit') == best.get('commit') and r.get('run') == best.get('run'):
        marker = ' *'

    print(f'{run:>4}  {commit:>7}  {metric:>12.6f}  {status:<14}  {desc}{marker}')

print('-' * 72)

# Confidence (if enough data)
if len(segment_results) >= 3:
    import statistics
    values = [r['metric'] for r in segment_results if r['metric'] > 0]
    if len(values) >= 3:
        median_val = statistics.median(values)
        deviations = [abs(v - median_val) for v in values]
        mad = statistics.median(deviations)
        if mad > 0 and kept and baseline_metric > 0:
            delta = abs(best_metric - baseline_metric)
            conf = delta / mad
            if conf >= 2.0:
                level = 'LIKELY REAL'
            elif conf >= 1.0:
                level = 'MARGINAL'
            else:
                level = 'WITHIN NOISE'
            print(f'Confidence: {conf:.2f}x ({level})')
        elif mad == 0:
            print('Confidence: N/A (zero noise)')
        else:
            print('Confidence: N/A')
    else:
        print('Confidence: N/A (insufficient non-zero values)')
else:
    print(f'Confidence: N/A (need >= 3 runs, have {len(segment_results)})')

print('=' * 72)
" "$JSONL_FILE"

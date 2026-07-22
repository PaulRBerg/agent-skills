#!/bin/bash
# Compute MAD-based confidence score from autoresearch.jsonl
# Usage: confidence.sh [path/to/autoresearch.jsonl]
# Exit codes: 0 = success, 1 = insufficient data, 2 = file not found

set -euo pipefail

JSONL_FILE="${1:-autoresearch.jsonl}"

if [ ! -f "$JSONL_FILE" ]; then
  echo "Error: $JSONL_FILE not found" >&2
  exit 2
fi

python3 -c "
import json, sys, statistics

jsonl_file = sys.argv[1]

# Parse all experiment results (skip config lines)
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

if len(results) < 3:
    print(f'Insufficient data: {len(results)} runs (need >= 3)')
    sys.exit(1)

# Find the current segment (highest segment number, or 0 if no segment field)
max_segment = max((r.get('segment', 0) for r in results), default=0)
segment_results = [r for r in results if r.get('segment', 0) == max_segment]

if len(segment_results) < 3:
    print(f'Insufficient data in current segment: {len(segment_results)} runs (need >= 3)')
    sys.exit(1)

# Extract metric values (exclude <= 0 for crash entries)
values = [r['metric'] for r in segment_results if r['metric'] > 0]

if len(values) < 3:
    print(f'Insufficient non-zero metric values: {len(values)} (need >= 3)')
    sys.exit(1)

# Compute MAD
median_val = statistics.median(values)
deviations = [abs(v - median_val) for v in values]
mad = statistics.median(deviations)

if mad == 0:
    print('MAD is zero — no measurable noise in the data')
    sys.exit(0)

# Find baseline (first run in segment) and best kept result
baseline = segment_results[0]
baseline_metric = baseline['metric']

# Determine direction from whether kept results improve up or down
kept = [r for r in segment_results if r['status'] == 'keep' and r['metric'] > 0]
if not kept:
    print(f'No kept results in current segment')
    print(f'Baseline: {baseline_metric}')
    print(f'MAD: {mad:.6f}')
    sys.exit(0)

# Infer direction: if best kept < baseline, lower is better; otherwise higher
best_kept = min(kept, key=lambda r: r['metric'])
worst_kept = max(kept, key=lambda r: r['metric'])

if best_kept['metric'] < baseline_metric:
    # Lower is better
    direction = 'lower'
    best = best_kept
else:
    # Higher is better
    direction = 'higher'
    best = worst_kept

best_metric = best['metric']
delta = abs(best_metric - baseline_metric)
confidence = delta / mad

# Color interpretation
if confidence >= 2.0:
    level = 'LIKELY REAL'
elif confidence >= 1.0:
    level = 'MARGINAL'
else:
    level = 'WITHIN NOISE'

total = len(segment_results)
kept_count = len(kept)
discarded = sum(1 for r in segment_results if r['status'] == 'discard')
crashed = sum(1 for r in segment_results if r['status'] == 'crash')
checks_failed = sum(1 for r in segment_results if r['status'] == 'checks_failed')

print(f'Confidence: {confidence:.2f}x ({level})')
print(f'Baseline:   {baseline_metric}')
print(f'Best:       {best_metric} ({direction} is better)')
print(f'Delta:      {delta:.6f}')
print(f'MAD:        {mad:.6f}')
print(
    f'Runs:       {total} total, {kept_count} kept, {discarded} discarded, '
    f'{crashed} crashed, {checks_failed} checks_failed'
)
" "$JSONL_FILE"

# Configuration Profile

Load when the diff touches config, infra limits, or rollout controls.

## Checks

- `CFG-001` High-magnitude change (`HIGH`): A value changes significantly without a baseline or justification.
- `CFG-002` Timeout/retry inversion (`HIGH`): The upstream/downstream timeout or retry hierarchy causes cascading
  failures.
- `CFG-003` Pool/limit mismatch (`HIGH`): Connection/thread/concurrency limits can starve or overload dependencies.
- `CFG-004` Env drift (`MEDIUM`): Prod values are copied blindly from dev/staging without proportional scaling.
- `CFG-005` Rollback gap (`MEDIUM`): A risky change lacks an available rollback or rollout-control strategy.
- `CFG-006` Observability gap (`MEDIUM`): The change has no metric/alert for safe validation.

## Evidence Expectations

- Compare new values against previous values.
- Identify the concrete failure mode under load.

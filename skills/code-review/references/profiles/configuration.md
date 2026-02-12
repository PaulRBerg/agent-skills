# Configuration Profile

Load for config, infra limits, or rollout controls.

## Checks

- `CFG-001` High-magnitude change (`HIGH`): significant value shift without baseline or justification.
- `CFG-002` Timeout/retry inversion (`HIGH`): upstream/downstream timeout or retry hierarchy causes cascading failures.
- `CFG-003` Pool/limit mismatch (`HIGH`): connection/thread/concurrency limits can starve or overload dependencies.
- `CFG-004` Env drift (`MEDIUM`): prod values copied blindly from dev/staging without proportional scaling.
- `CFG-005` Rollback gap (`MEDIUM`): risky change lacks feature flag, canary path, or reversible plan.
- `CFG-006` Observability gap (`MEDIUM`): no metric/alert to validate the change safely.

## Evidence Expectations

- Compare new values against previous values.
- Call out concrete failure mode under load.

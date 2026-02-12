# TypeScript / React / Node Profile

Load for `*.ts`, `*.tsx`, `*.js`, `*.jsx`.

## Checks

- `TS-001` Unsafe typing (`HIGH`): `any`/assertion-heavy paths bypass safety at boundaries.
- `TS-002` Hook dependency bugs (`HIGH`): stale closures or missing effect dependencies.
- `TS-003` Async race/leak (`HIGH`): out-of-order writes, missing cancellation, or missing cleanup.
- `TS-004` Event-loop blocking (`HIGH`): sync I/O or CPU-heavy work in request paths.
- `TS-005` Unhandled async failures (`HIGH`): missing `await`/`.catch`/error propagation.
- `TS-006` Subscription/cache leak (`MEDIUM`): listeners, timers, or unbounded caches persist indefinitely.

## Evidence Expectations

- Show failing timeline or stale state path for races.
- Point to concrete runtime impact (crash, stale UI/data, latency spike).

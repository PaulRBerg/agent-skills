# Core Review Profile

Apply on every run.

## Checks

- `CORE-001` Behavior regression (`HIGH`): changed branch/state transition alters external behavior.
- `CORE-002` Error-path safety (`HIGH`): failures can cascade, crash, or return unsafe defaults.
- `CORE-003` Boundary handling (`HIGH`): null/empty/overflow/edge inputs are not handled.
- `CORE-004` Resource hygiene (`MEDIUM`): leaked timers/listeners/handles/connections.
- `CORE-005` Complexity hotspot (`MEDIUM`): change introduces avoidable coupling or hidden side effects.
- `CORE-006` Test gap (`MEDIUM`): changed behavior has no targeted test coverage.

## Evidence Expectations

- Show the concrete input/state that triggers failure.
- Point to changed lines or nearby guards that caused the risk.

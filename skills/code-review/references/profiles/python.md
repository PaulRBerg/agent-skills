# Python Profile

Load for `*.py` and Python service code.

## Checks

- `PY-001` Mutable defaults (`HIGH`): shared state across calls from mutable default args.
- `PY-002` Async blocking (`HIGH`): blocking I/O in coroutine paths.
- `PY-003` Dangerous execution (`CRITICAL`): `eval`/`exec`/unsafe deserialization on untrusted input.
- `PY-004` Injection surfaces (`CRITICAL`): SQL string interpolation, `subprocess(..., shell=True)` with user input.
- `PY-005` Iterator/lifecycle bugs (`MEDIUM`): exhausted iterators reused or context cleanup omitted.
- `PY-006` Type-blind boundaries (`MEDIUM`): weakly validated external payloads.

## Evidence Expectations

- Show exact call path where untrusted input crosses into dangerous API.
- Include deterministic repro condition when possible.

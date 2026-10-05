# Python Profile

Load when the diff touches `*.py` or Python service code.

## Checks

- `PY-001` Mutable defaults (`HIGH`): Mutable default args share state across calls.
- `PY-002` Async blocking (`HIGH`): Coroutine paths perform blocking I/O.
- `PY-003` Dangerous execution (`CRITICAL`): Code uses `eval`/`exec`/unsafe deserialization on untrusted input.
- `PY-004` Injection surfaces (`CRITICAL`): Code uses SQL string interpolation or uses `subprocess(..., shell=True)`
  with user input.
- `PY-005` Iterator/lifecycle bugs (`MEDIUM`): Code reuses exhausted iterators or omits context cleanup.
- `PY-006` Type-blind boundaries (`MEDIUM`): Code weakly validates external payloads.

## Evidence Expectations

- Show the exact call path where untrusted input enters a dangerous API.
- When possible, include a deterministic condition that reproduces the issue.

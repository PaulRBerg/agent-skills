# Security Profile

Load when code touches trust boundaries.

## Checks

- `SEC-001` Injection sink (`CRITICAL`): SQL/shell/template/path input reaches execution without safe binding.
- `SEC-002` Broken auth/authz (`CRITICAL`): missing ownership checks, privilege escalation paths, or trust on client-only checks.
- `SEC-003` Secret exposure (`HIGH`): credentials in source, logs, artifacts, or client bundles.
- `SEC-004` Unsafe execution/parsing (`CRITICAL`): `eval`/unsafe deserialization/untrusted code execution.
- `SEC-005` Session/token weakness (`HIGH`): missing validation/rotation/expiry constraints.
- `SEC-006` Traversal or SSRF (`HIGH`): user-controlled paths/URLs can reach unintended resources.
- `SEC-007` Missing abuse controls (`MEDIUM`): no throttling/lockout/rate limits on brute-force paths.

## Evidence Expectations

- Show attacker-controlled input path to vulnerable sink.
- State preconditions and realistic blast radius.

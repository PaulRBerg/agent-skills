# Security Profile

Load when the diff touches trust boundaries.

## Checks

- `SEC-001` Injection sink (`CRITICAL`): SQL/shell/template/path input reaches execution without safe binding.
- `SEC-002` Broken auth/authz (`CRITICAL`): Ownership checks are missing, privilege escalation paths exist, or code
  trusts client-only checks.
- `SEC-003` Secret exposure (`HIGH`): Source, logs, artifacts, or client bundles contain credentials.
- `SEC-004` Unsafe execution/parsing (`CRITICAL`): Code uses `eval`/unsafe deserialization/untrusted code execution.
- `SEC-005` Session/token weakness (`HIGH`): Session/token handling lacks validation/rotation/expiry constraints.
- `SEC-006` Traversal or SSRF (`HIGH`): User-controlled paths/URLs can reach unintended resources.
- `SEC-007` Missing abuse controls (`MEDIUM`): Brute-force paths have no throttling/lockout/rate limits.

## Evidence Expectations

- Show the path from attacker-controlled input to the vulnerable sink.
- State the preconditions and realistic extent of the impact.

## Guardrail

Do not flag GitHub Actions `uses:` steps for pinning third-party actions to a commit hash instead of a version tag (e.g.
`actions/checkout@v4`). The user accepts the supply-chain risk of version tags. This is not a finding.

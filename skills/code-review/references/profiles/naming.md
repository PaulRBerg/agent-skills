# Naming Profile

Use after correctness/security findings are handled.

## Checks

- `NM-001` Generic function names (`MEDIUM`): names like `process`/`handle` hide intent.
- `NM-002` Misleading identifiers (`MEDIUM`): name contradicts actual data shape or behavior.
- `NM-003` Boolean ambiguity (`LOW`): booleans not expressed as `is/has/can/should` style predicates.
- `NM-004` File/export mismatch (`LOW`): filename and exported symbol diverge from project conventions.
- `NM-005` Constant intent loss (`LOW`): magic values or value-based constant names.

## Guardrail

Only raise naming findings when they materially reduce maintainability in the touched code.

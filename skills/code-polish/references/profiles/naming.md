# Naming Profile

Load last, after correctness and security checks. This profile is optional. Skip it with `--skip-profile naming`.

## Checks

- `NM-001` Generic function names (`MEDIUM`): Names like `process`/`handle` hide intent.
- `NM-002` Misleading identifiers (`MEDIUM`): The name contradicts the actual data shape or behavior.
- `NM-003` Boolean ambiguity (`LOW`): A boolean name conceals the condition it represents.
- `NM-004` File/export mismatch (`LOW`): The filename and exported symbol differ from project conventions.
- `NM-005` Constant intent loss (`LOW`): Code uses magic values or value-based constant names.
- `NM-006` Misleading filename (`LOW`): The file's actual responsibility differs from what its name implies (e.g.,
  `utils.ts` that only formats dates → `date-format.ts`). Suggest a rename with a reason. If the mismatch is likely to
  cause incorrect usage or placement of new code, flag it as `MEDIUM`.

## Guardrail

Only raise naming findings when they materially reduce maintainability in the touched code.

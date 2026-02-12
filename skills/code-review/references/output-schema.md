# Output Schema

Use this structure and order for every review result.

## 1. Scope

List reviewed files and any excluded patterns.

## 2. Findings (ordered)

Order by severity: `CRITICAL -> HIGH -> MEDIUM -> LOW`.

For each finding, use this shape:

- `[SEVERITY] Title — path/to/file.ext:line`
- Impact: concrete user/system impact.
- Evidence: exact code behavior or diff evidence.
- Fix: smallest practical remediation.
- Confidence: `high | medium | low`.

## 3. Suggested Fixes

Include when not using `--fix`.

## 4. Applied Fixes

Include only when `--fix` is used. List each change with file references.

## 5. Verification

List commands run and outcomes. Explicitly list skipped checks.

## 6. Residual Risks / Open Questions

Capture unresolved assumptions and follow-ups.

## Rules

- Do not fabricate locations.
- Merge duplicate findings.
- Keep style-only issues at LOW unless they create operational risk.

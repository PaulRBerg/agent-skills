# Finding Triage and Report

Shared triage process for CodeRabbit findings from any source (PR comments or local CLI). Operates on a normalized list of findings, each with: `path`, `line` (optional), `body`, `source`.

## 1) Categorize

For each finding, categorize into:

- **AI-actionable**: concrete code suggestion with a clear, specific fix
- **Nitpick**: style or naming preference without functional impact
- **Informational**: explanation or context without a specific suggestion

Group AI-actionable findings by file path.

## 2) Classify Each Finding

For each AI-actionable finding, perform a thorough assessment:

1. **Read the actual code** at the referenced file and line number. Do not rely solely on the quoted snippet — it may be outdated or truncated.
2. **Check project conventions** by examining linter configs (ESLint, Prettier, Biome, Ruff, etc.), existing patterns in the codebase, and any style guides or CONTRIBUTING.md files.
3. **Review broader context** by reading surrounding functions, the module's purpose, and related tests to understand whether the suggestion fits.
4. **Assess applicability** — does this suggestion make sense for this specific codebase, or is it generic advice that doesn't apply here?
5. **Evaluate severity** based on real impact to the system, not just theoretical concern.

Assign one of two classifications:

- **Valid** with severity: CRITICAL > HIGH > MEDIUM > LOW
  - **CRITICAL**: exploitable security flaw, data loss path, or outage risk on critical paths
  - **HIGH**: logic defect or performance failure that can break core behavior
  - **MEDIUM**: maintainability or reliability issue likely to cause near-term defects
  - **LOW**: localized clarity, style, or documentation improvements
- **False Positive** with a specific reason:
  - Incorrect assumption about the code's behavior or purpose
  - Project convention mismatch (suggestion contradicts established patterns)
  - Already handled elsewhere in the codebase
  - Out of scope for the current diff's intent

## 3) Confirm Ambiguous Classifications

After classifying all findings, identify those where confidence is **Medium** or **Low** and the code is on a critical path (auth, payments, data integrity, core business logic). For each such finding, use `AskUserQuestion` to ask the user:

- header: Truncated filename (max 12 chars)
- question: "CodeRabbit flagged `<file>:<line>` — <one-line summary of suggestion>. Is this valid?"
- options:
  - "Valid — fix it" with description of what the fix entails
  - "False positive — skip" with description of why it might not apply
  - "Needs context — defer" with description that it will be added to Residual Risks
- multiSelect: false

Batch up to 4 questions per `AskUserQuestion` call to minimize interruptions. Update classifications based on user responses before generating the fix plan.

Skip this step when all medium/low-confidence findings are on non-critical paths — classify those using your best judgment and note the confidence level in the report.

## 4) Generate Fix Plan

For each valid finding, ordered by severity (CRITICAL first, LOW last), produce a structured entry:

| Field                 | Content                                       |
| --------------------- | --------------------------------------------- |
| Location              | `file:line`                                   |
| CodeRabbit suggestion | Brief summary of what was suggested           |
| Assessment            | Why this is valid and what the real impact is |
| Proposed fix          | Concrete code change description              |
| Confidence            | High / Medium / Low                           |

Confidence reflects certainty that the fix is correct and safe to apply:

- **High**: clear defect with an obvious fix, no ambiguity
- **Medium**: likely correct but depends on runtime behavior or external state not fully visible
- **Low**: plausible improvement but may have side effects or require domain knowledge to validate

## 5) Report

Output a structured report with the following sections:

**Scope**: Source (PR number/URL or local review), number of findings, number of files.

**Summary**: N findings from source, N classified as valid, N as false positive, N nitpicks skipped.

**Valid Findings** (grouped by severity):

For each severity level present, list findings with location, CodeRabbit's original suggestion, your assessment, and the proposed fix. CRITICAL and HIGH findings should include explicit reasoning about blast radius and failure modes.

**False Positives**:

For each dismissed finding, list the location, what CodeRabbit suggested, and the specific rationale for dismissal.

**Fix Plan**:

Ordered implementation steps for all valid findings. Group related fixes that touch the same file. Note any fixes that should be applied together to avoid intermediate broken states.

**Nitpicks (skipped)**:

Brief list of nitpick findings that were excluded from triage — mention location and what was suggested, but no fix plan entry.

**Residual Risks**:

Flag anything that needs human judgment or falls beyond automated triage — ambiguous intent, architectural concerns, suggestions that require product decisions, or findings where classification confidence is low on critical-path code.

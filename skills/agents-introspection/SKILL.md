---
argument-hint: <task>
disable-model-invocation: true
name: agents-introspection
user-invocable: true
description:
  Retrospect on a task against local Codex/Claude Code transcripts; propose durable fixes (AGENTS.md, skills).
---

# Agents Introspection

Determine whether prior Codex and Claude Code work in the current project establishes a recurrence risk for the user's
task, then recommend the smallest durable intervention justified by the evidence.

Success means the report identifies transcript coverage, separates observed behavior from inference, applies a
consistent evidence bar, and either proposes a concrete prevention step or explains why no durable change is justified.

## Input

- `<task>` (required): the task, decision, incident, or workflow to evaluate. If omitted but the current conversation
  states it clearly, use that task; otherwise ask for the missing task.

## Scope and Authority

- Inspect only Codex and Claude Code transcripts whose metadata or cwd resolves to the current project. Include another
  project only when the user explicitly names its path.
- Treat every transcript as sensitive plaintext. Summarize instead of quoting; redact secrets, private addresses,
  tokens, emails, and personal or customer data.
- Inspect and report by default. Edit AGENTS.md or skills only when the user explicitly asks to apply or implement
  fixes, then make the smallest in-scope local change and validate it.
- Never modify transcript stores, write transcript excerpts or derived private data into repository files, or perform
  external writes.

## Bounded Retrieval

Read `references/transcript-sources.md`, resolve the current project with `pwd -P`, and choose 3–6 short, discriminative
keywords from relevant filenames, commands, tools, errors, package names, issue IDs, and skill names.

1. Run the bundled miner for the current project, active sessions only, with the chosen keywords and `--max-sessions 8`.
2. Treat miner scores, themes, and correction, failure, verification, tool, or privacy counts only as candidate-ranking
   signals. They are heuristic and are never evidence by themselves.
3. Validate project metadata or cwd before opening a candidate. Inspect up to five highest-relevance transcript bodies,
   stopping earlier when the evidence bar is met. Include a comparable successful session when available.
4. If evidence is insufficient, retry once with broader keywords. If active history still lacks signal, retry once with
   `--include-archived`.
5. Exceed these bounds only to resolve contradictory evidence or satisfy an explicitly exhaustive request. If the helper
   fails, use one project-scoped manual fallback from the reference.

Stop and report the coverage gap when the bounded fallbacks still lack useful evidence. Absence of evidence is not
evidence that a failure never occurred.

For unusually long searches, send sparse progress updates only when a retrieval fallback begins, a finding changes the
likely intervention, or the search reaches its explicit bound. Use an outcome-first line such as
`🔎 Broadening transcript search — <verified reason and bound>` or
`⏳ Checking archived sessions — <verified active/archived coverage>`. Ground counts and coverage claims in miner/tool
output; do not narrate routine transcript reads.

## Evidence Contract

For each relevant session, record only concise, auditable observations about:

- ignored or misread AGENTS.md or skill instructions;
- wrong cwd, project root, source path, or path encoding;
- over-broad edits, unrelated churn, overwritten user work, or destructive commands;
- tooling, shell, parsing, retry, or verification mistakes;
- invented claims, vague reports, or missing tests and checks;
- successful patterns that prevented mistakes.

Connect each observation to the current task and label any causal or recurrence claim as inference. State conflicts and
weak coverage rather than averaging them away.

Use these confidence levels:

- **High**: at least two independent relevant sessions directly support the same pattern and its relevance to the
  current task.
- **Medium**: one unambiguous relevant session supports the finding, or multiple sessions provide mixed support.
- **Low**: only indirect, ambiguous, or heuristic signals exist. Do not recommend a durable repository change from
  low-confidence evidence.

A durable change requires either the same failure in at least two independent sessions or one unambiguous high-impact
failure that exposes a missing stable invariant. Treat lower-impact one-offs as manual guardrails.

## Choose the Smallest Intervention

- Update AGENTS.md when the lesson is stable, project-wide, and useful to agents working in that scope.
- Update an existing skill when the failure belongs clearly inside its current workflow.
- Propose a new skill only for a repeated, reusable procedure that spans projects or repositories.
- Add a script only when deterministic discovery, parsing, or validation would otherwise be reimplemented.
- Recommend no durable change for one-off mistakes, weak evidence, or rules already stated clearly; report the risk and
  manual guardrail instead.

## Report and Stop

Lead with `### 🔎 Introspection complete — <intervention or coverage-gap outcome>` for read-only work or
`### ✅ Introspection fixes applied — <outcome>` when explicitly requested fixes were written, then report only:

1. `🗂 Historical coverage`: project paths, sources checked, fallbacks used, and sessions inspected.
2. `🔎 Findings`: a compact table with confidence, observed evidence, inference, relevance, and intervention. Keep
   confidence visibly separate from severity or impact.
3. `🛡 Durable recommendations`: apply now, consider later, or no change, with the target and prevention mechanism.
4. `🧪 Validation and gaps`: commands run, checks performed, privacy limitations, and missing evidence.

When fixes were explicitly requested, include exact files changed and validation outcomes. Stop after the current task
has an evidence-backed recommendation or an explicit coverage gap; do not mine additional history merely to add examples
or strengthen prose. Keep transcript references, paths, counters, redactions, and miner JSON exact and undecorated.

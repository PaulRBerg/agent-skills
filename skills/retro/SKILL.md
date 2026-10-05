---
argument-hint: "[session-or-focus]"
compatibility:
  Requires Claude Code or Codex CLI. Transcript history requires the agents-introspection skill and its requirements.
coordination: exempt
disable-model-invocation: true
name: retro
skill-dependencies:
  - agents-introspection
description:
  Run a retrospective on Codex or Claude Code sessions. Propose changes to steering files, checks, review rules, skills,
  tools, or information access. Check recurrence across transcripts with agents-introspection.
---

# Retro

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

If a slash or dollar invocation already placed these instructions in the conversation, follow them directly. In that
case, do not invoke this skill again through a skill tool.

Review agent work. Propose changes to the agent's environment that improve future runs. The environment includes
steering files, skills, automated checks, review guidance, tools, and information access. Do not grade the code or the
user's decisions.

Success means every candidate has session evidence, a recurrence confidence, and a concrete target. The report orders
candidates by severity, or it explains why no change is justified.

## Input

- `[session-or-focus]` (optional): the sessions, time window, incident, or theme to review. Without it, review the
  current session.

## Scope and Authority

- Inspect and report by default. Change files only when the user explicitly asks to apply candidates.
- Read past transcripts only through agents-introspection. Follow its retrieval, secret-handling, and disclosure rules.

## 1. Resolve the Primary Sessions

The primary sessions are the sessions under review.

- Without an argument, the primary session is the current session. Use the conversation in context. If compaction
  removed necessary detail, have agents-introspection inspect the live transcript.
- For named sessions, a time window, or a theme, the matching sessions are the primary sessions. agents-introspection
  finds and reads them.

## 2. Choose the History Scope

Decide how much past transcript history the request needs. Do not use a fixed session count. A history scope can set a
time window, a session count, specific sessions, archived sessions, and other relevant projects.

| Request                                                | History scope                                                                       |
| ------------------------------------------------------ | ----------------------------------------------------------------------------------- |
| Current session, no stated pattern                     | Recent sessions in the same project. Use only enough to test each candidate.        |
| Named sessions                                         | The named sessions, plus a recurrence check for their candidates.                   |
| A time window, such as "this week"                     | All sessions in that window, in every relevant project.                             |
| A claim that a problem repeats                         | A wide window. Include archived sessions and other projects where the workflow ran. |
| Friction in a shared skill or a global steering file   | Every project where that skill or file applied.                                     |
| An explicit session count, window, or exhaustive scope | Exactly the scope that the user requested.                                          |

Start with the smallest scope that can confirm or reject recurrence. When the user stated the scope, mark it as fixed.
Otherwise, agents-introspection may widen it through its fallbacks. State the scope and the reason in the report.

## 3. Collect Candidates

Read the primary sessions. Record each friction point with its evidence. Look for these signals:

- The agent searched for a long time to find a file, a fact, or an owner.
- The agent made a mistake that an automated check could catch.
- The review missed a mistake, or the user corrected the agent.
- The agent made an expensive, repeated, or high-output tool call.
- The agent did not have necessary information.
- A skill or steering instruction was wrong, unclear, or had no effect.

## 4. Check Recurrence

Invoke the agents-introspection skill (`$agents-introspection`) one time for all candidates. Do not invoke it once per
candidate. Give it the candidates with their evidence, the primary sessions, and the history scope.

Use its confidence levels and evidence bar for each behavior candidate. Recommend a durable change only when that bar
supports it. Report other behavior candidates as manual guardrails or watch items.

Environment-state candidates do not need recurrence evidence from transcripts. Examples are a missing guardrail, an
unwired check, an oversized steering file, or a no-op instruction. Verify them directly in the repository.

## 5. Inspect the Environment

Before you propose a change, read the current mechanism that the change replaces or extends:

- Steering files: the global, repository, and nested `AGENTS.md` or `CLAUDE.md` files that applied to the sessions.
- Check commands: `justfile`, `package.json` scripts, `Makefile`, pre-commit hooks, and CI workflows.
- Review guidance: `CODING_STANDARDS.md`, review skills, or review configuration, when present.
- Skills that the sessions loaded.

If an existing check or rule is unwired, broken, or ignored, report that as the finding. Do not propose a duplicate.

## 6. Classify Candidates

Assign one category to each candidate. Use the category to choose the change.

- **Navigation**: the agent spent time to find a file, a fact, or a hidden dependency between files. Add a navigation
  pointer to the nearest steering file or doc that the agent already reads.
- **Automated checks**: an automated check could catch the mistake. Read the repository's own check commands first. A
  repository with no guardrail is a finding by itself. A guardrail is a pre-commit hook, CI job, or documented local
  gate that runs lint, type-check, or tests.
- **Coding standards**: the review missed a mistake, or a review rule is wrong or unclear. Classify the violation first.
  A mechanical violation gets a deterministic check, such as a lint rule, a pre-commit hook, or a gate job. Mechanical
  violations include a fixed syntax pattern, a banned API, an import shape, or a file-location rule. Prefer the cheapest
  check that the repository's language and guardrails support. Write a review rule only for a judgment call that no
  check can enforce.
- **Steering files**: a repository or global steering file is large. Move review-only rules into review guidance. Move
  mechanical rules into checks.
- **No-ops**: when steering files are large, find instructions that do not change agent behavior. For example, an
  instruction restates a default, or a check already enforces it. Recommend its removal.
- **Tool economy**: a tool call used many tokens or much time, or a custom CLI or MCP tool returned too much output.
  Recommend a narrower command, a bounded helper, or a recipe.
- **Information access**: the agent did not have a necessary fact. Recommend a way to expose it, such as dev-server logs
  in a readable file or read-only access to a third-party service.
- **Skills**: a skill gave outdated, unclear, or missing instructions, or caused avoidable manual work. Recommend the
  smallest fix in the skill source.

### Placement Rules

Implementation agents have the most context pressure. They explore, write code, and debug. Review agents receive a diff,
so they have the least context pressure. When the workflow has a review stage, put coding standards there.

- `AGENTS.md` and `CLAUDE.md` load into the context of every agent in their scope. Use them sparingly, mostly for
  navigation pointers and project-wide invariants.
- Review guidance loads during review, not during implementation. When it becomes long, move parts into docs and add
  navigation pointers.
- Docs are reference files that other files point to. Find existing docs before you write new ones.
- Skills hold reusable procedures, on-demand reference knowledge, or user-invoked commands. Only the skill description
  stays in context.

## 7. Report and Stop

Lead with `### 🔎 Retro complete — <outcome>`. When the user explicitly asked to apply candidates, lead with
`### ✅ Retro fixes applied — <outcome>`. Then report only:

1. `🗂 Scope`: the primary sessions, the history scope and its reason, and the transcript coverage.
2. `🔎 Candidates`: a table in severity order with severity, category, evidence, confidence, and the proposed change
   with its target. Keep confidence separate from severity.
3. `🛡 Recommendations`: apply now, consider later, or no change.
4. `🧪 Gaps`: missing evidence and unverified assumptions.

Rank severity by impact. Correctness or safety risk ranks first, then repeated cost in time or tokens, then minor
friction. Present one combined report. Do not repeat the agents-introspection report separately.

Stop after the report. When the user asks to apply candidates, make the smallest change in the owning file. Validate it
with the repository's own checks. Report the changed files and the check outcomes.

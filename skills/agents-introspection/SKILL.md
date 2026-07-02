---
argument-hint: <task>
disable-model-invocation: true
name: agents-introspection
user-invocable: true
description: Retrospect on a task against local Codex/Claude Code transcripts; propose durable fixes (AGENTS.md, skills).
---

# Agents Introspection

Analyze the user's task against prior Codex and Claude Code work in the current directory, then turn repeated agent failure modes into concrete prevention steps.

## Arguments

- `<task>` (required): the task, decision, incident, or proposed workflow to evaluate in light of prior agent transcripts. If omitted but the current conversation clearly states a task, use that task.

## Workflow

### 1. Define scope

1. Resolve the current project path with `pwd -P`.
2. Restate the task in one sentence.
3. Identify likely keywords: filenames, commands, tools, domains, errors, package names, issue IDs, and skill names.
4. Read [transcript sources](references/transcript-sources.md) before touching transcript directories.

### 2. Discover project transcripts

Look only at Codex and Claude Code transcripts for the current project unless the user explicitly names additional project paths. Run the miner per [transcript sources](references/transcript-sources.md); pass `--max-sessions N` to keep the evidence set small. Open transcript bodies only after a session plausibly matches the current project or task.

### 3. Select evidence

Sample enough history to distinguish a pattern from a one-off:

- Prioritize sessions in the same cwd/workspace and recent sessions with task-keyword overlap.
- Include at least one successful comparable session when available, not only failures.
- Stop early when additional transcripts repeat the same evidence without changing the conclusion.
- Treat all transcript content as sensitive plaintext. Do not paste raw transcript excerpts unless a short quote is essential; redact secrets, private addresses, tokens, emails, and personal data.

### 4. Classify agent behavior

For each relevant session, extract concise evidence for:

- Misread instructions or ignored `AGENTS.md` / skill guidance.
- Wrong cwd, wrong project root, wrong transcript/source path, or bad path encoding.
- Over-broad edits, unrelated file churn, reverted user work, or destructive commands.
- Tooling mistakes: skipped `just`, wrong shell dialect, brittle parsing, missing narrow verification.
- Repeated loops: same failed command, stale assumption, no escalation after errors.
- Quality gaps: missing tests, unverified claims, vague final reports, invented facts.
- Positive patterns that avoided mistakes and should be preserved.

Target only Codex and Claude Code.

### 5. Connect history to the task

Answer these questions:

1. What has gone wrong before on tasks like this?
2. Which prior failures are likely to recur for the current task?
3. Which constraints or checks would have prevented them?
4. Which observed successes are worth making standard?

Separate evidence-backed findings from speculation. If transcript coverage is thin, say so and lower confidence.

### 6. Choose durable fixes

Recommend the smallest durable intervention:

- Update `AGENTS.md` when the lesson is project-wide, stable, and useful to every agent working here.
- Create a new skill when the pattern is procedural, repeated, and reusable across projects or repos.
- Update an existing skill when the failure belongs clearly inside that skill's current scope.
- Add a script only when deterministic transcript discovery, parsing, or validation would otherwise be reimplemented repeatedly.
- Do nothing durable for one-off mistakes; report the risk and the manual guardrail.

If the current invocation explicitly asks to apply fixes, make the edits and verify them. Otherwise, report recommendations first and wait for confirmation before changing `AGENTS.md`, creating skills, or editing existing skills.

### 7. Report

Use this structure:

```md
## Historical Scope

- Project path:
- Sources checked:
- Sessions sampled:

## Findings

- [confidence] Failure mode:
  Evidence:
  Relevance to current task:
  Prevention:

## Durable Fixes

- Apply now:
- Consider later:
- Not worth changing:

## Verification

- Commands run:
- Gaps:
```

Keep the report terse and evidence-led. Mention exact files changed and checks run when fixes are applied.

## Guard Rails

- Never read transcripts outside the current project scope unless the user explicitly expands scope.
- Never write transcript excerpts or derived private data into repo files.
- Never create broad policy from a single ambiguous transcript.
- Never blame "the agent" or "the model" generically — name the concrete failure mode: the specific instruction, command, check, or workflow step that broke.
- Prefer `rg`, `fd`, `jq`, and structured parsing over ad hoc pipelines.

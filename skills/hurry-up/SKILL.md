---
argument-hint: "[deadline-or-note]"
compatibility:
  Requires an interactive Claude Code or Codex CLI session. Producing the final handoff requires the task-handoff skill
  and its requirements.
coordination: exempt
disable-model-invocation: true
name: hurry-up
user-invocable: true
description:
  Wind down a long-running session fast when the user must leave — freeze new work, collect wrap-up reports from active
  subagents, secure finished work, and hand off the remainder via task-handoff.
---

# Hurry Up

This skill is coordination-exempt: skip the ai-coord gate (`git status` / `ai-coord status` / `ai-coord start`) for this
skill's own work.

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

The user must leave now. Convert everything in flight — the session's own work and any subagents' — into either secured
completed work or a decision-complete handoff, without losing progress. Optimize for wall-clock time over polish: target
roughly five minutes end to end. Use `$ARGUMENTS` as the user's deadline or departure note when present; a stated
deadline overrides the default budget.

This skill applies equally to orchestrated sessions with active subagents and to a single agent working alone. In the
single-agent case, skip Worker Wind-Down and apply the same wrap-up rules to your own in-progress work.

## Freeze

Immediately and before anything else:

- Launch no new subagents, waves, or scopes. Reclassify queued or not-yet-started work as remaining work for the
  handoff.
- Stop your own expansion into new files or subsystems; finish only the edit currently mid-flight so no file is left
  syntactically broken or half-rewritten.
- Start no new long-running validation. Record unverified claims as unverified instead of proving them.

## Worker Wind-Down

Enumerate active subagents with the host's agent listing, then message every one of them — in Claude Code through
SendMessage, in Codex CLI through the native subagent messaging tool — with a wrap-up order:

- Finish the smallest safe unit: complete the current edit, leave no file half-written, start nothing new.
- Skip remaining validation; report what ran and what was skipped instead.
- Reply with exactly these named fields: `status` (`completed`, `partial`, or `not-started`), `summary`, `changed files`
  listing only files actually touched, `verification` listing commands run with outcomes plus checks skipped,
  `remaining work`, and `risks/blockers`.

Give workers a soft deadline of two to three minutes, or less when the user's stated deadline demands it. When a worker
stays silent past the deadline, stop it and reconstruct its state from `git status` and `git diff` within its assigned
scope; mark that reconstruction as lower-confidence in the synthesis.

Do not assign new work, redesigns, or fixes through these messages, even when a report reveals a problem. A discovered
problem becomes remaining work in the handoff.

## Secure

Inventory the working tree with `git status` and `git diff`, and map every change to a workstream using the wrap-up
reports. Classify each workstream as completed (edits done and validated), partial (edits or validation unfinished), or
untouched.

Follow the session's existing commit policy: when committing is already authorized for this session, commit each
completed, validated workstream now as its own small commit so it cannot be lost, and leave partial work uncommitted but
fully described in the handoff. Never run tree-wide git commands that could sweep unrelated or concurrent work, and
never commit half-applied changes to make the tree look clean.

## Hand Off

Synthesize one state picture across your own work and every wrap-up report: what completed, what is partial and exactly
how far it got — keyed to stable paths and symbols, never line numbers — what remains, which claims are unverified, and
known blockers.

Invoke the task-handoff skill (`$task-handoff`) with the remaining work, and make each handoff body decision-complete
per that skill's contract, folding in the synthesis: current tree state, per-workstream progress, remaining steps, and
the validation the resumed agent must run first. Default to one handoff; use coordinated handoffs only when remaining
workstreams have independent outcomes or owners. If the task-handoff skill is unavailable, deliver the same
decision-complete content in the final report instead.

## Report

Finish with `### 🏁 Hurry-up — session parked` followed by, compactly: completed workstreams with commit hashes when
committed, partial workstreams and their exact stopping points, the handoff command(s) verbatim from task-handoff's
report, and an always-present risks line (`none` when empty). Keep it short — the user is walking out the door.

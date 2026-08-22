---
argument-hint: "[deadline-or-note]"
compatibility:
  Requires an interactive Claude Code or Codex CLI session. Producing the final handoff requires the task-handoff skill
  and its requirements.
coordination: exempt
disable-model-invocation: true
name: wrap-up
skill-dependencies:
  - task-handoff
description:
  Wind down a long-running session fast when the user must leave — freeze new work, preserve active subagents, secure
  finished work, and hand off the remainder via task-handoff.
---

# Wrap Up

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

The user must leave now. Convert everything in flight — the session's own work and any subagents' — into either secured
completed work or a decision-complete handoff, without losing progress. Optimize for wall-clock time over polish: target
roughly five minutes end to end. Use `$ARGUMENTS` as the user's deadline or departure note when present; a stated
deadline overrides the default budget.

In a single-agent session, skip Worker Wind-Down and apply the same rules to the session's own in-progress work.

## Freeze

Immediately and before anything else:

- Launch no new subagents, waves, or scopes. Reclassify queued or not-yet-started work as remaining work for the
  handoff.
- Stop your own expansion into new files or subsystems; finish only the edit currently mid-flight so no file is left
  syntactically broken or half-rewritten.
- Start no new long-running validation. Record unverified claims as unverified instead of proving them.

## Worker Wind-Down

Enumerate active subagents with the host's agent listing, then message every messageable worker — in Claude Code through
SendMessage, in Codex CLI through the native subagent messaging tool — with a wrap-up order:

- Finish the smallest safe unit: complete the current edit, leave no file half-written, start nothing new.
- Skip remaining validation; report what ran and what was skipped instead.
- Reply with exactly these named fields: `status` (`completed`, `partial`, or `not-started`), `summary`, `changed files`
  listing only files actually touched, `verification` listing commands run with outcomes plus checks skipped,
  `remaining work`, and `risks/blockers`.

Give messageable workers a soft deadline of two to three minutes, or less when the user's stated deadline demands it.
The deadline limits how long this skill waits; it is never permission to cancel a worker. When any worker remains active
at the deadline, leave its agent, subprocess runner, and watcher running under their existing timeouts. Never use a
stop, interrupt, kill, or termination action merely because wrap-up was invoked, the worker is silent, or it has no
message channel.

Classify such a worker as `active-unsettled`. Record its assigned scope, last observed activity, and existing timeout or
settlement condition. On-disk edits and peer messages are provisional while that worker can still write: do not
reconstruct a final result, launch replacement work, or claim that its scope is complete. If time permits, wait for
natural settlement and collect the normal result; otherwise defer result reconciliation to the handoff.

Do not assign new work, redesigns, or fixes through these messages, even when a report reveals a problem. A discovered
problem becomes remaining work in the handoff.

## Secure

Inventory the working tree with `git status --short` and `git diff --stat`, and map every change to a workstream using
the wrap-up reports. Classify each workstream as completed (edits done and validated), partial (edits or validation
unfinished), or untouched.

Exclude every `active-unsettled` scope from validation, staging, and commits. Preserve its current coordination claims
and avoid overlapping writes until the worker settles; a snapshot may describe visible paths only as provisional
evidence, never as a substitute for its result.

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

For each `active-unsettled` worker, make settlement and result reconciliation a prerequisite to any resumed edits in its
scope. Do not turn its still-running assignment into immediately actionable replacement work.

## Report

Finish with `### 🏁 Hurry-up — session parked` followed by, compactly: completed workstreams with their `$commit`
receipt OIDs when committed (never a follow-up `git log`), partial workstreams and their exact stopping points,
active-unsettled workers with their settlement conditions, the handoff command(s) verbatim from task-handoff's report,
and an always-present risks line (`none` when empty). Keep it short — the user is walking out the door.

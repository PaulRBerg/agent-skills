---
argument-hint: "[task-to-handoff]"
compatibility:
  Requires Bash 3.2, Git, local file-write access, and macOS pbcopy, pbpaste, and trash. The generated launch command
  requires an authenticated Codex CLI.
disable-model-invocation: true
name: task-handoff
user-invocable: true
description:
  Create decision-complete single- or multi-repository task handoffs under `.ai/task-handoffs/` and return commands for
  fresh interactive Codex sessions.
---

# Task Handoff

If these instructions are already present from a slash or dollar invocation, follow them directly; do not invoke this
skill again through a skill tool.

Turn one continuation task into self-contained task handoffs for fresh Codex chats. Write handoffs only; do not
implement, edit tracked files, commit, push, launch Codex, or change ignore configuration.

## Select the work

Use `$ARGUMENTS` when present. Otherwise infer the next unfinished step from the last assistant response and relevant
transcript. Treat the task as a scope selector: retain only evidence needed for that work.

Resolve discoverable facts before writing. Inspect every involved repository's instructions, relevant task surface, and
working-tree state; distinguish completed work, partial work, and remaining work while preserving user and
concurrent-agent changes. If the task is already complete, report the evidence and create nothing. Ask only when an
unresolved choice materially changes scope, safety, execution approach, verification, repository membership, or handoff
placement.

Classify each handoff from the user's requested outcome, expected evidence, and authorized actions before choosing its
topology. Reason about the task; do not keyword-match it. Select exactly one lowercase category:

- `implementation`: change code, configuration, documentation, or data to reach a defined end state.
- `investigation`: explain a bounded symptom, behavior, or failure with evidence and a recommended next action.
- `research`: answer or compare options using collected evidence and deliver a recommendation or decision record.
- `audit`: assess a defined surface against explicit criteria and report prioritized, evidence-backed findings.
- `operations`: carry out a bounded maintenance, release, migration, or coordination workflow with state checks and
  rollback boundaries.

When a task contains supporting work from another category, use the category of its primary deliverable. Split it into
coordinated handoffs only when the work has independent outcomes or owners.

Infer repositories from local paths and relevant context. Do not include the current repository merely because the skill
runs there when the task selects only other repositories. Choose the topology from the task:

- Default to one handoff owned by the current repository.
- A cross-repository handoff uses the clearly requested owner and covers every involved repository.
- Multiple coordinated handoffs each belong to their clearly requested owner.
- Stop before writing when the repository set, one-versus-many topology, or owner mapping is unclear.

Choose a meaningful unique filename per handoff matching `^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*[.]md$`; do not force a `PLAN_`
prefix. If a target exists, add a semantic qualifier, falling back to `_YYYY_MM_DD_HHMMSS` only when no meaningful
qualifier distinguishes it.

## Prepare drafts

Resolve `scripts/task-handoff.sh` relative to this loaded `SKILL.md`. Invoke it without reading its implementation:

```sh
bash <skill-dir>/scripts/task-handoff.sh prepare \
  --repo '<candidate-repository>' \
  --plan '<owner-candidate>' '<HANDOFF_NAME.md>' '<task-category>' '<concise-task>' \
  [--repo ...] [--plan ...]
```

Pass every involved repository with `--repo` and every handoff in intended report/clipboard order with `--plan`. Pass
the inferred category exactly as one of the five values above. The helper validates and canonicalizes physical Git
roots, owners, categories, ignored new targets, filenames, and required macOS tools before creating mode-0700 temporary
state. Use its shell-quoted `run_dir`, `repo`, and `plan` records as authoritative; each `plan` record gives its
canonical owner, relative target, absolute target, category, and draft path.

Write only the semantic handoff body to each draft. Never add `## Handoff category`, `## Execution status`, or
`## Handoff cleanup`; `finalize` reserves and appends them. Make every body decision-complete for an agent with access
to the named repositories but none of this transcript. Include:

- objective, success criteria, and explicit exclusions;
- verified current state, partial changes, and completed prerequisites;
- changes keyed to stable paths, symbols, interfaces, schemas, or commands rather than line numbers;
- task-specific evidence, procedure, material edge cases, and failure behavior;
- targeted validation, acceptance scenarios, and rollout, compatibility, or authority requirements;
- assumptions resolved from repository evidence or explicit user decisions.

Tailor the body to its category. An implementation handoff specifies the intended change, data flow, and compatibility.
An investigation handoff specifies the question or symptom, available evidence, reproduction or observation method, and
the decision the findings must support. A research handoff specifies questions, sources or evidence to collect,
alternatives, and recommendation criteria. An audit handoff specifies the assessed surface, criteria, evidence method,
and severity or prioritization model. An operations handoff specifies preconditions, ordered state transitions,
authority boundaries, observability, and rollback or recovery.

For cross-repository or coordinated handoffs, also name every canonical root, its role and exact write scope, dependency
and execution order, repository-local validation, combined acceptance criteria, and repository-relative related-handoff
paths. Summarize relevant context instead of quoting the transcript. Leave no placeholders, open task choices, or
references that require the old chat.

## Finalize or cancel

After all bodies are complete, run:

```sh
bash <skill-dir>/scripts/task-handoff.sh finalize '<run-dir>'
```

`finalize` re-runs preflight, rejects empty or reserved-heading drafts, appends the fixed category, execution-status,
and per-handoff cleanup contracts, validates the complete structures, publishes every new handoff without overwriting,
and copies and byte-verifies the ordered Codex commands. It rolls back helper-created targets and now-empty directories
on handled errors, `INT`, or `TERM`; it cannot make publication atomic across filesystems or survive power loss or
`SIGKILL`.

Each final `plan` record contains `relative=`, canonical `owner=`, `category=`, and the exact command after `command=`.
Never execute the command. If a correctable draft error occurs, edit the draft and retry. If abandoning or blocking
before successful finalization, remove only helper-created temporary state with:

```sh
bash <skill-dir>/scripts/task-handoff.sh cancel '<run-dir>'
```

Never create or remove targets yourself. A failed or cancelled run must leave no helper-created handoff set.

## Report

On success, finish with `### ✅ Task handoff ready — <task>`. For each handoff, list the final record's
repository-relative path, canonical owner, category, and exact command in a code block, preserving handoff order. Do not
repeat handoff bodies or mention clipboard copying or verification.

For a blocker, finish with `### ⛔ Task handoff not written — <reason>` and state that no handoff file was created.

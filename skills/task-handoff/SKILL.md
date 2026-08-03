---
argument-hint: "[task-to-handoff]"
compatibility:
  Requires Bash 3.2, Git, local file-write access, and macOS pbcopy, pbpaste, and trash. The generated launch command
  requires an authenticated Codex CLI.
disable-model-invocation: true
name: task-handoff
user-invocable: true
description:
  Create decision-complete single- or multi-repository task plans under `.ai/task-handoffs/` and return commands for
  fresh interactive Codex sessions.
---

# Task Handoff

If these instructions are already present from a slash or dollar invocation, follow them directly; do not invoke this
skill again through a skill tool.

Turn one continuation task into self-contained implementation plans for fresh Codex chats. Write plans only; do not
implement, edit tracked files, commit, push, launch Codex, or change ignore configuration.

## Select the work

Use `$ARGUMENTS` when present. Otherwise infer the next unfinished step from the last assistant response and relevant
transcript. Treat the task as a scope selector: retain only evidence needed for that work.

Resolve discoverable facts before writing. Inspect every involved repository's instructions, relevant implementation,
and working-tree state; distinguish completed work, partial work, and remaining work while preserving user and
concurrent-agent changes. If the task is already complete, report the evidence and create nothing. Ask only when an
unresolved choice materially changes scope, safety, implementation, verification, repository membership, or plan
placement.

Infer repositories from local paths and relevant context. Do not include the current repository merely because the skill
runs there when the task selects only other repositories. Choose the topology from the task:

- Default to one plan owned by the current repository.
- A cross-repository plan uses the clearly requested owner and covers every involved repository.
- Multiple coordinated plans each belong to their clearly requested owner.
- Stop before writing when the repository set, one-versus-many topology, or owner mapping is unclear.

Choose a meaningful unique filename per plan matching `^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*[.]md$`; do not force a `PLAN_`
prefix. If a target exists, add a semantic qualifier, falling back to `_YYYY_MM_DD_HHMMSS` only when no meaningful
qualifier distinguishes it.

## Prepare drafts

Resolve `scripts/task-handoff.sh` relative to this loaded `SKILL.md`. Invoke it without reading its implementation:

```sh
bash <skill-dir>/scripts/task-handoff.sh prepare \
  --repo '<candidate-repository>' \
  --plan '<owner-candidate>' '<PLAN_NAME.md>' '<concise-task>' \
  [--repo ...] [--plan ...]
```

Pass every involved repository with `--repo` and every plan in intended report/clipboard order with `--plan`. The helper
validates and canonicalizes physical Git roots, owners, ignored new targets, filenames, and required macOS tools before
creating mode-0700 temporary state. Use its shell-quoted `run_dir`, `repo`, and `plan` records as authoritative; each
`plan` record gives its canonical owner, relative target, absolute target, and draft path.

Write only the semantic plan body to each draft. Never add `## Execution status` or `## Handoff cleanup`; `finalize`
reserves and appends them. Make every body decision-complete for an agent with access to the named repositories but none
of this transcript. Include:

- objective, success criteria, and explicit exclusions;
- verified current state, partial changes, and completed prerequisites;
- changes keyed to stable paths, symbols, interfaces, schemas, or commands rather than line numbers;
- data flow, material edge cases, and failure behavior;
- targeted validation, acceptance scenarios, and rollout or compatibility requirements;
- assumptions resolved from repository evidence or explicit user decisions.

For cross-repository or coordinated plans, also name every canonical root, its role and exact write scope, dependency
and execution order, repository-local validation, combined acceptance criteria, and repository-relative related-plan
paths. Summarize relevant context instead of quoting the transcript. Leave no placeholders, open implementation choices,
or references that require the old chat.

## Finalize or cancel

After all bodies are complete, run:

```sh
bash <skill-dir>/scripts/task-handoff.sh finalize '<run-dir>'
```

`finalize` re-runs preflight, rejects empty or reserved-heading drafts, appends the fixed execution-status and per-plan
cleanup contracts, validates the complete structures, publishes every new plan without overwriting, and copies and
byte-verifies the ordered Codex commands. It rolls back helper-created targets and now-empty directories on handled
errors, `INT`, or `TERM`; it cannot make publication atomic across filesystems or survive power loss or `SIGKILL`.

Each final `plan` record contains `relative=`, canonical `owner=`, and the exact command after `command=`. Never execute
the command. If a correctable draft error occurs, edit the draft and retry. If abandoning or blocking before successful
finalization, remove only helper-created temporary state with:

```sh
bash <skill-dir>/scripts/task-handoff.sh cancel '<run-dir>'
```

Never create or remove targets yourself. A failed or cancelled run must leave no helper-created plan set.

## Report

On success, finish with `### ✅ Task handoff ready — <task>`. For each plan, list the final record's repository-relative
path, canonical owner, and exact command in a code block, preserving plan order. Do not repeat plan bodies or mention
clipboard copying or verification.

For a blocker, finish with `### ⛔ Task handoff not written — <reason>` and state that no plan file was created.

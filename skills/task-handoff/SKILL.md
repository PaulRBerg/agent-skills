---
argument-hint: "[task-to-handoff]"
compatibility:
  Requires Git, local file-write access, and macOS pbcopy, pbpaste, and trash. The generated launch command requires an
  authenticated Codex CLI.
disable-model-invocation: true
name: task-handoff
user-invocable: true
description:
  Create decision-complete single- or multi-repository task plans under `.ai/task-handoffs/` and return commands for
  fresh interactive Codex sessions.
---

# Task Handoff

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

Turn one continuation task from the current session into one or more self-contained implementation plans for fresh Codex
chats. A handoff may stay within one repository, span repositories in one plan, or use multiple coordinated plans. Write
plans only; do not implement them or launch Codex.

## Input

Use `$ARGUMENTS` as the task to hand off when present. When the user supplies no task argument, infer the next
unfinished step or partial task from the last assistant response and the existing chat transcript. Treat the selected
task as a scope selector: retain only relevant conversation context and repository evidence. Infer repositories and plan
placement from the prompt and relevant conversation; do not expose repository-selection flags. If the task, repository
set, or plan placement is materially ambiguous, ask for the missing decision and stop without writing any files.

## Contract

- Resolve discoverable facts from the repository before writing. Distinguish completed work, existing partial work, and
  work still required.
- Preserve relevant user or concurrent-agent changes. Do not edit tracked files, implement the task, commit, push, or
  run the generated Codex command.
- Ask about an unresolved choice only when it materially changes scope, safety, implementation, or verification.
- If the requested task is already complete, report the evidence and finish as a no-op without creating a handoff.
- Write only new plans under each plan owner's `.ai/task-handoffs/`. Never overwrite or update an existing handoff.
- Make each plan instruct its implementing agent to move only that plan to Trash after all of its work is complete and
  every required validation passes. The plan must remain in place when work or required validation is incomplete or
  unsuccessful.
- Make each plan include an `Execution status` section initialized with no recorded implementation attempt. If work
  stops before successful completion, the plan must instruct the implementing agent to replace that status with a
  concise handback instead of appending an attempt history, then retain the plan.
- Treat the complete plan set and clipboard delivery as one operation: a failed preflight, write, validation, or
  clipboard copy must leave no partial plan set.

## Workflow

1. Resolve the current repository with `git rev-parse --show-toplevel`. Infer every other repository named or clearly
   identified by the prompt or relevant conversation. Resolve each candidate from a local path or subdirectory to its
   canonical, physical Git root; stop if any candidate is not a Git worktree. Deduplicate roots that resolve to the same
   repository, including aliases and symlinked paths. Do not include the current repository merely because the skill
   runs there when the task clearly selects only other repositories.
2. Inspect the relevant conversation plus every resolved repository's instructions, relevant implementation, and
   working-tree state. Distinguish completed work, existing partial work, and work still required without pulling
   unrelated context into the handoff.
3. Derive the plan topology from the prompt:

   - By default, create one plan owned by the current repository.
   - For one plan spanning repositories, use the clearly requested owner and describe every involved repository.
   - For multiple plans, assign each plan to its clearly requested owner and place it under that owner's
     `.ai/task-handoffs/`.
   - If the repository set, one-plan-versus-many choice, or owner mapping is unclear, ask before creating anything.

4. Choose a bespoke filename for each plan that accurately names its task and matches
   `^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)*[.]md$`. Filenames must be unique within the plan set. Do not force a generic `PLAN_`
   prefix. If a target exists, add a meaningful task qualifier; if no semantic qualifier can distinguish it, append
   `_YYYY_MM_DD_HHMMSS`.
5. Preflight the complete plan set before creating any directory or file:

   - Reconfirm that every involved root and every plan owner is a canonical Git worktree root.
   - Reconfirm that every prospective target is new, has a valid unique filename, and belongs to its assigned owner.
   - Confirm `/usr/bin/pbcopy`, `/usr/bin/pbpaste`, and `/usr/bin/trash` are executable.
   - From each owner, verify its prospective repository-relative path is ignored:

         git -C '<canonical-owner-root>' check-ignore -q -- ".ai/task-handoffs/<PLAN_NAME>.md"

   If any check fails, stop without writing any plan or changing `.gitignore`, `.git/info/exclude`, or other ignore
   configuration.

6. Create the required `.ai/task-handoffs/` directories and write every plan. Make each plan decision-complete for an
   agent that has access to the named repositories but none of the current transcript. Include:

   - the objective, success criteria, and explicit exclusions;
   - verified current state, including relevant partial changes or completed prerequisites;
   - implementation changes keyed to stable paths, symbols, interfaces, schemas, or commands rather than line numbers;
   - data flow plus material edge cases and failure behavior;
   - targeted validation, acceptance scenarios, and any rollout or compatibility requirements;
   - assumptions already resolved from evidence or explicit user decisions.

   Immediately before `Handoff cleanup`, include this `Execution status` section:

   ```md
   ## Execution status

   Current status: No implementation attempt has been recorded.

   If work stops before successful completion, replace the current status—not append an attempt history—with a concise
   record of completed work, remaining work, validation commands and outcomes, the blocker, and the next concrete
   action.
   ```

   End every plan with a `Handoff cleanup` section that identifies only that plan by its exact canonical absolute path.
   Instruct the implementing agent to run `/usr/bin/trash '<canonical-plan-path>'` only after every success criterion is
   satisfied and every required validation passes, then verify the original path no longer exists. Instruct it to keep
   the plan when work remains, implementation or validation fails, or required validation is skipped, and never trash
   the `.ai/task-handoffs/` directory or any other handoff.

   For every plan that spans repositories or coordinates with repository-specific plans, also include:

   - every canonical repository root, its role, and its exact expected write scope;
   - dependency and execution-order constraints across repositories;
   - repository-local validation plus combined acceptance criteria;
   - repository-relative references to related handoff plans, when the plan set contains them.

   Summarize relevant context instead of quoting the transcript. Leave no unresolved placeholders or implementation
   choices. If any write fails, remove only plans and now-empty directories created by this run, then report that no
   plan set was written.

7. Re-read every plan and verify that the complete set exists, every path remains ignored, every filename is valid and
   unique, owner mappings and related-plan references agree, and each plan contains enough scoped evidence and
   validation for implementation without the old chat. Verify that every `Execution status` section immediately precedes
   `Handoff cleanup`, uses the exact initial status, and requires replacement rather than appended history with
   completed work, remaining work, validation outcomes, the blocker, and the next action. Verify that every
   `Handoff cleanup` section names its own exact canonical absolute path, gates Trash on completed work and successful
   required validation, preserves the plan for incomplete or unsuccessful work, and prohibits removing the directory or
   other handoffs. On failure, remove only artifacts created by this run so no partial plan set remains.
8. Produce exactly one shell-safe command per plan for the active shell using this exact shape and that plan's canonical
   owner root:

       codex -C '<plan-owning-repo>' 'A previous agent worked on <concise-task> and produced an implementation plan under .ai/task-handoffs/<PLAN_NAME>.md. Implement it.'

   Use bare `codex` so each command opens the interactive TUI in the plan-owning repository. Do not add `--repo`,
   `--add-dir`, `exec`, model, reasoning-effort, approval, sandbox, profile, search, or display flags. Quote the
   repository path and prompt without exposing them to shell interpolation.

9. Build the clipboard payload from the exact command strings that will appear in the completion report, preserving plan
   order. For one plan, the payload is exactly its command. For multiple plans, separate commands with one newline. Do
   not include code fences or a trailing newline. Copy the payload to the macOS clipboard with `/usr/bin/pbcopy` without
   executing any generated command. Read it back with `/usr/bin/pbpaste` and verify the bytes match exactly. If the copy
   or verification fails, remove only plans and now-empty directories created by this run, then report that no plan set
   was written.

## Completion

Finish with `### ✅ Task handoff ready — <task>` and state that clipboard copy was verified. For every plan, list its
repository-relative path, canonical owner root, and exact Codex command in a code block. Emit one command per plan and
do not repeat plan bodies. For a blocker, use `### ⛔ Task handoff not written — <reason>` and state that no plan file
was created.

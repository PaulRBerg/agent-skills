---
argument-hint: <task-to-handoff>
compatibility:
  Requires Git and local file-write access. The generated launch command requires an authenticated Codex CLI.
disable-model-invocation: true
name: task-handoff
user-invocable: true
description:
  Create a decision-complete partial-task plan under `.ai/task-handoffs/` and return a command for a fresh interactive
  Codex session.
---

# Task Handoff

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

Turn one continuation task from the current session into a self-contained implementation plan for a fresh Codex chat.
Write the plan only; do not implement it or launch Codex.

## Input

Use `$ARGUMENTS` as the task to hand off. Treat it as a scope selector: retain only relevant conversation context and
repository evidence. If it is missing or too ambiguous to identify one task, ask for the task and stop without writing a
file.

## Contract

- Resolve discoverable facts from the repository before writing. Distinguish completed work, existing partial work, and
  work still required.
- Preserve relevant user or concurrent-agent changes. Do not edit tracked files, implement the task, commit, push, or
  run the generated Codex command.
- Ask about an unresolved choice only when it materially changes scope, safety, implementation, or verification.
- If the requested task is already complete, report the evidence and finish as a no-op without creating a handoff.
- Write exactly one new plan under `<repo-root>/.ai/task-handoffs/`. Never overwrite or update an existing handoff.

## Workflow

1. Resolve the repository root with `git rev-parse --show-toplevel`. If it fails, stop: the ignored-file guarantee
   requires a Git worktree.
2. Inspect the relevant conversation, repository instructions, current implementation, and working-tree state. Do not
   pull unrelated context into the handoff.
3. Choose a bespoke filename that accurately names the task and matches `^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)*[.]md$`. Do not
   force a generic `PLAN_` prefix. If the target exists, add a meaningful task qualifier; if no semantic qualifier can
   distinguish it, append `_YYYY_MM_DD_HHMMSS`.
4. Before creating the directory or file, verify the prospective repository-relative path is ignored:

       git check-ignore -q -- ".ai/task-handoffs/<PLAN_NAME>.md"

   If it is not ignored, stop without changing `.gitignore`, `.git/info/exclude`, or any other ignore configuration.

5. Create `.ai/task-handoffs/` and write the plan. Make it decision-complete for an agent that has the repository but
   none of the current transcript. Include:

   - the objective, success criteria, and explicit exclusions;
   - verified current state, including relevant partial changes or completed prerequisites;
   - implementation changes keyed to stable paths, symbols, interfaces, schemas, or commands rather than line numbers;
   - data flow plus material edge cases and failure behavior;
   - targeted validation, acceptance scenarios, and any rollout or compatibility requirements;
   - assumptions already resolved from evidence or explicit user decisions.

   Summarize relevant context instead of quoting the transcript. Leave no unresolved placeholders or implementation
   choices.

6. Re-read the file and verify that it exists, remains ignored, matches the filename convention, addresses only the
   requested task, and contains enough evidence and validation for implementation without the old chat.
7. Produce one shell-safe command for the active shell using this exact shape:

       codex -C '<absolute-repo-root>' 'A previous agent worked on <concise-task> and produced an implementation plan under .ai/task-handoffs/<PLAN_NAME>.md. Implement it.'

   Use bare `codex` so the command opens the interactive TUI. Include no `exec`, model, reasoning-effort, approval,
   sandbox, profile, search, or display flags. Quote the repository path and prompt without exposing them to shell
   interpolation.

## Completion

Finish with `### ✅ Task handoff ready — <task>`, the repository-relative plan path, and the exact Codex command in a
code block. Do not repeat the plan body. For a blocker, use `### ⛔ Task handoff not written — <reason>` and state that
no file was created.

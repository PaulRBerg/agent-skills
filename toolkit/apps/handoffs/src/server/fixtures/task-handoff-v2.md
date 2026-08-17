---
category: 'implementation'
created: '2026-08-10T08:00:00Z'
launch_repo: '/Users/example/projects/app'
repos:
  - '/Users/example/projects/app'
origin: '/Users/example/projects/app/.ai/task-handoffs/TASK_HANDOFF_COMPATIBILITY_V2.md'
task: 'Validate task handoff compatibility'
---
# Validate task handoff compatibility

Confirm that the viewer accepts the complete handoff format emitted by the task-handoff skill.

## Success criteria

- Parse the handoff as the frontmatter format.
- Preserve the semantic body and generated lifecycle sections as rendered Markdown.

## Handoff category

Category: `implementation`

This handoff is categorized above. Complete the requested task according to its stated outcome, boundaries, authority
constraints, and validation requirements.

## Execution status

Current status: No task attempt has been recorded.

If work stops before successful completion, replace the current status—not append an attempt history—with a concise
record of completed work, remaining work, validation commands and outcomes, the blocker, and the next concrete action.

## Handoff cleanup

Archive this handoff only after the requested work is complete and task-scoped validation passes:

```sh
ai-handoff archive '/Users/example/projects/app/.ai/task-handoffs/TASK_HANDOFF_COMPATIBILITY_V2.md'
```

A broader required check may remain non-green only when evidence attributes every failure to pre-existing or unrelated
work outside this task's scope. Record each non-green command, its outcome, and that attribution in the final report,
then verify the original path no longer exists. Keep this handoff when work remains, task-scoped validation fails or is
skipped, or any broader failure may have been caused by this task. Archive only this handoff, never
`.ai/task-handoffs/` or any other handoff.

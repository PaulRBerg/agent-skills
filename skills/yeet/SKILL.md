---
argument-hint: <create-pr|update-pr|create-issue|update-issue|comment-issue|create-discussion> [options]
disable-model-invocation: false
effort: high
name: yeet
user-invocable: true
description: 'Use for GitHub PR/issue/discussion workflows: create/update PRs or issues, post comments, start discussions; triggers include create/open PR, file/update issue, yeet.'
---

# GitHub Contribution Workflows

Facilitate GitHub-based open source contribution workflows including pull requests, issues, and discussions. Emphasizes semantic analysis over mechanical operations — understand the intent and context of changes before generating titles, descriptions, or selecting templates. All generated content should be conversational and informal.

## Prerequisites

Use the first required read-only `gh` command in each workflow as authentication validation. Prefer `scripts/yeet-context.sh` when the workflow needs repository, template, discussion, label, or issue/PR thread context.

For pull request workflows, also verify:

- Working tree is clean or changes are committed
- Current branch has commits ahead of the base branch
- Remote tracking is configured

## Related Skills

For detailed GitHub CLI command syntax, flags, and patterns, activate the `cli-gh` skill.

## Workflows

Each workflow is fully documented in its reference file. Load the appropriate reference based on user intent.

| Workflow          | Trigger                                                | Reference                         |
| ----------------- | ------------------------------------------------------ | --------------------------------- |
| Create PR         | "create PR", "open PR", "yeet a PR"                    | `references/create-pr.md`         |
| Update PR         | "update PR", "edit PR"                                 | `references/update-pr.md`         |
| Create Issue      | "create issue", "file issue" (generic repo)            | `references/create-issue.md`      |
| Update Issue      | "update issue", "edit issue", "relabel issue"          | `references/update-issue.md`      |
| Claude Code Issue | "Claude Code issue", "report bug in CC"                | `references/issue-claude-code.md` |
| Codex CLI Issue   | "Codex issue", "report bug in Codex"                   | `references/issue-codex-cli.md`   |
| Sablier Issue     | "Sablier issue", "sablier-labs issue"                  | `references/issue-sablier.md`     |
| Comment on Issue  | "comment on issue", "reply on issue", "post a comment" | `references/comment-issue.md`     |
| Create Discussion | "create discussion", "start discussion"                | `references/create-discussion.md` |

Shared patterns (auth validation, admonitions, HEREDOC syntax, semantic analysis, tone, platform normalization, error handling, file links) are in `references/commons.md`.

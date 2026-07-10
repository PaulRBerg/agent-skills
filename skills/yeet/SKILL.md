---
argument-hint: <create-pr|update-pr|create-issue|update-issue|comment-issue|create-discussion> [options]
disable-model-invocation: false
effort: high
name: yeet
user-invocable: true
description:
  "Use for GitHub PR/issue/discussion workflows: create/update PRs or issues, post comments, start discussions; triggers
  include yeet."
---

# GitHub Contribution Workflows

Create or update GitHub contributions from repository evidence, using the matching workflow's templates, idempotency
rules, and Paul's writing voice.

## Prerequisites

Use the first required read-only `gh` command in each workflow as authentication validation. The
`scripts/yeet-context.sh` helper is bundled with this skill, not the target repository. Resolve it to an absolute path
relative to the directory containing this `SKILL.md`, and never search for it in the target repository. Prefer the
helper when the workflow needs repository, template, discussion, label, or issue/PR thread context.

For pull request workflows, also verify:

- Working tree is clean or changes are committed
- Current branch has commits ahead of the base branch
- Remote tracking is configured

Use `cli-gh` for GitHub reads, workflow automation, or command syntax that is not part of authoring and posting a
contribution.

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

Each workflow reference links only the shared context, writing, or posting guidance it needs. Post directly when the
user requested creation or update; do not add a confirmation gate. After a failed write, run the linked idempotency
check before any retry.

## Completion

Complete when the requested contribution exists in its final authored state and the returned GitHub URL has been
verified. For updates/comments, report the changed artifact once; for failures, report the idempotency check and next
action without claiming a write succeeded.

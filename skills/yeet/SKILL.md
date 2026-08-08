---
argument-hint:
  <create-pr|update-pr|create-issue|update-issue|comment-issue|create-discussion|update-discussion|comment-discussion>
  [options]
compatibility: Authenticated GitHub CLI >= 2.97.0
coordination: exempt
disable-model-invocation: false
effort: high
name: yeet
skill-dependencies:
  - cli-gh
user-invocable: true
description:
  "Use for GitHub PR/issue/discussion workflows: create/update PRs, issues, or discussions and post issue or discussion
  comments; triggers include yeet."
---

# GitHub Contribution Workflows

This skill is coordination-exempt: skip the ai-coord gate (`git status` / `ai-coord status` / `ai-coord start`) for this
skill's own work.

Create or update GitHub contributions from repository evidence, using the matching workflow's templates, idempotency
rules, and Paul's writing voice.

## Prerequisites

Use the first required read-only `gh` command in each workflow as authentication validation. Resolve `<skill-dir>` once
to the absolute directory containing this `SKILL.md`. The `yeet-context.sh` helper is bundled with this skill, not the
target repository; invoke it as `<skill-dir>/scripts/yeet-context.sh` and never search for it in the target repository.
Prefer the helper when the workflow needs repository, template, discussion, label, or issue/PR thread context.

For YAML issue forms, invoke `<skill-dir>/scripts/issue-form.py`. `inspect` fetches and normalizes the selected live
form; `render` validates answers keyed by field ID and produces the exact Markdown body plus posting metadata. The
helper never selects a template, writes answers or titles, performs an external-disclosure review, or posts externally.

For pull request workflows, also verify:

- Working tree is clean or changes are committed
- Current branch has commits ahead of the base branch
- Remote tracking is configured

Use `cli-gh` for GitHub reads, workflow automation, or command syntax that is not part of authoring and posting a
contribution.

## Workflows

Each workflow is fully documented in its reference file. Load the appropriate reference based on user intent.

| Workflow           | Trigger                                                                   | Reference                          |
| ------------------ | ------------------------------------------------------------------------- | ---------------------------------- |
| Create PR          | "create PR", "open PR", "yeet a PR"                                       | `references/create-pr.md`          |
| Update PR          | "update PR", "edit PR"                                                    | `references/update-pr.md`          |
| Create Issue       | "create issue", "file issue" (generic repo)                               | `references/create-issue.md`       |
| Update Issue       | "update issue", "edit issue", "relabel issue"                             | `references/update-issue.md`       |
| Claude Code Issue  | "Claude Code issue", "report bug in CC"                                   | `references/issue-claude-code.md`  |
| Codex CLI Issue    | "Codex issue", "report bug in Codex"                                      | `references/issue-codex-cli.md`    |
| Sablier Issue      | "Sablier issue", "sablier-labs issue"                                     | `references/issue-sablier.md`      |
| Comment on Issue   | "comment on issue", "reply on issue", "post a comment"                    | `references/comment-issue.md`      |
| Create Discussion  | "create discussion", "start discussion"                                   | `references/create-discussion.md`  |
| Update Discussion  | "update discussion", "edit discussion"                                    | `references/update-discussion.md`  |
| Comment Discussion | "comment on discussion", "reply on discussion", "edit discussion comment" | `references/comment-discussion.md` |

Each workflow reference links only the shared context, writing, or posting guidance it needs. Post directly when the
user requested creation or update; do not add a confirmation gate. After a failed write, run the linked idempotency
check before any retry.

Never check an external template attestation unless repository or user evidence verifies it. If a required attestation
or field cannot be verified, ask for that missing fact rather than inventing agreement. Agent-status decoration belongs
outside the authored contribution; add emoji to a PR, issue, discussion, or comment only when the user's content or the
thread's register calls for it.

## Completion

Complete when the requested contribution exists in its final authored state and the returned GitHub URL has been
verified. For updates/comments, report the changed artifact once; for failures, report the idempotency check and next
action without claiming a write succeeded.

Use `### 🚀 <artifact> created`, `### ✅ <artifact> updated`, `### ✅ Comment posted`, or `### ✅ Comment updated`,
followed by one Markdown link containing the repository, number, and title or action. Add a compact field list only when
base, draft state, reviewers, labels, or changed fields matter. On failure, lead with `### ⛔ <artifact> not <action>`,
then state the attempted target, concrete error, idempotency result, and next action. Keep `gh` output, JSON,
diagnostics, template fields, URLs, and authored contribution text exact and undecorated.

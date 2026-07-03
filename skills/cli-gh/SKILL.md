---
disable-model-invocation: false
name: cli-gh
user-invocable: false
description: 'Use for GitHub CLI automation: repo info/browsing, workflow triggers, GitHub search, codespaces, PR status, or issues.'
---

# GitHub CLI (gh)

## Overview

Expert guidance for GitHub CLI (gh) operations and workflows. Use this skill for command-line GitHub operations including pull request management, issue tracking, repository operations, workflow automation, and codespace management.

**Key capabilities:**

- Create and manage pull requests from the terminal
- Track and organize issues efficiently
- Search across all of GitHub (repos, issues, PRs)
- Manage labels and project organization
- Trigger and monitor GitHub Actions workflows
- Work with codespaces
- Automate repository operations and releases
- Browse repositories, PRs, and files in the browser

## Safety Rules

**CRITICAL: This skill forbids destructive gh CLI operations except the controlled label-deletion workflow below.**

This skill focuses on safe, read-only, or reversible GitHub operations. Treat destructive operations as prohibited unless this file explicitly defines an exception.

**Controlled destructive exception:**

`gh label delete` may be used only when every condition is met:

- The user explicitly asks to delete one or more repository labels.
- Before running any deletion command, present a deletion plan listing the target repository, exact label names, exact command(s), and the consequence that the labels will be removed from the repository and from existing issues and pull requests.
- The user explicitly approves that deletion plan in a subsequent message. The original deletion request, broad cleanup approval, prior consent, or silence is not approval.
- Run only the approved `gh label delete <name> --repo OWNER/REPO --yes` commands. If the repository, labels, or commands change, stop and present a revised plan for approval.

The following commands are **PROHIBITED** and must **NEVER** be used:

**Permanently destructive commands:**

- `gh repo delete` - Repository deletion
- `gh repo archive` - Repository archival
- `gh release delete` - Release deletion
- `gh release delete-asset` - Asset deletion
- `gh run delete` - Workflow run deletion
- `gh cache delete` - Cache deletion
- `gh secret delete` - Secret deletion
- `gh variable delete` - Variable deletion
- `gh ssh-key delete` - SSH key deletion (can lock out users)
- `gh gpg-key delete` - GPG key deletion
- `gh codespace delete` - Codespace deletion
- `gh extension remove` - Extension removal
- `gh gist delete` - Gist deletion
- Bulk deletion operations using `xargs` with any destructive commands
- Shell commands: `rm -rf` (except for temporary file cleanup)

**Allowed operations:**

- Creating resources (PRs, issues, releases, labels, repos)
- Viewing and listing (status, logs, information, searches)
- Updating and editing existing resources
- Closing PRs/issues (reversible - can be reopened)
- Reverting pull requests (creates a new revert PR)
- Canceling workflow runs (stops execution without deleting data)
- Merging pull requests (after proper review)
- Deleting repository labels only through the controlled destructive exception above
- Read-only git operations (`git status`, `git log`, `git diff`)

## Installation & Setup

```bash
# Login to GitHub
gh auth login

# Login and copy OAuth code to clipboard automatically
gh auth login --clipboard

# Check authentication status
gh auth status

# Check auth status with JSON output
gh auth status --json

# Configure git to use gh as credential helper
gh auth setup-git
```

## Pull Requests

### Creating PRs

```bash
# Create PR interactively
gh pr create

# Create PR with title and body
gh pr create --title "Add feature" --body "Description"

# Create PR to specific branch
gh pr create --base main --head feature-branch

# Create draft PR
gh pr create --draft

# Create PR from current branch
gh pr create --fill  # Uses commit messages

# Create PR with Copilot Code Review
gh pr create --reviewer @copilot
```

### Viewing PRs

```bash
# List PRs
gh pr list

# List my PRs
gh pr list --author @me

# View PR details
gh pr view 123

# View PR in browser
gh pr view 123 --web

# View PR diff
gh pr diff 123

# View PR diff excluding specific files
gh pr diff 123 --exclude "*.lock"

# Check PR status
gh pr status
```

### Managing PRs

```bash
# Checkout PR locally
gh pr checkout 123

# Review PR
gh pr review 123 --approve
gh pr review 123 --comment --body "Looks good!"
gh pr review 123 --request-changes --body "Please fix X"

# Request Copilot Code Review
gh pr edit 123 --add-reviewer @copilot

# Merge PR
gh pr merge 123
gh pr merge 123 --squash
gh pr merge 123 --rebase
gh pr merge 123 --merge

# Close PR
gh pr close 123

# Reopen PR
gh pr reopen 123

# Ready draft PR
gh pr ready 123

# Update PR branch with base branch
gh pr update-branch 123

# Revert a merged PR (creates a new revert PR)
gh pr revert 123
```

### PR Checks

```bash
# View PR checks
gh pr checks 123

# Watch PR checks
gh pr checks 123 --watch
```

## Issues

### Creating Issues

```bash
# Create issue interactively
gh issue create

# Create issue with title and body
gh issue create --title "Bug report" --body "Description"

# Use a Markdown issue template as interactive/editor starting body text
gh issue create --template "Bug Report"

# Create issue with labels
gh issue create --title "Bug" --label bug,critical

# Assign issue
gh issue create --title "Task" --assignee @me

# Set the issue type (GitHub.com and GHES 3.17+)
gh issue create --type Bug
```

`--template` cannot be combined with `--body` or `--body-file`; use it with prompts, `--editor`, or `--web`. For YAML issue forms, fetch and render the form fields yourself for non-interactive automation, or finish in the browser.

### Viewing Issues

```bash
# List issues
gh issue list

# List my issues
gh issue list --assignee @me

# List by label
gh issue list --label bug

# Filter by issue type
gh issue list --type Bug

# Advanced issue search
gh issue list --search "is:open label:bug sort:created-desc"

# View issue details
gh issue view 456

# View in browser
gh issue view 456 --web
```

### Managing Issues

```bash
# Close issue
gh issue close 456

# Close as duplicate, linking to the original issue
gh issue close 123 --duplicate-of 456

# Reopen issue
gh issue reopen 456

# Edit issue
gh issue edit 456 --title "New title"
gh issue edit 456 --add-label bug
gh issue edit 456 --add-assignee @user

# Comment on issue
gh issue comment 456 --body "Update"

# Create branch to work on issue
gh issue develop 456 --checkout
```

## Discussions

When the user asks to list, view, create, edit, or comment on GitHub Discussions, see [references/discussions.md](references/discussions.md). The `gh discussion` command set is in preview and subject to change.

## Repository Operations

### Repository Info

```bash
# View repository
gh repo view

# View in browser
gh repo view --web

# Clone repository
gh repo clone owner/repo

# Clone without adding upstream remote
gh repo clone owner/repo --no-upstream

# Fork repository
gh repo fork owner/repo

# List repositories
gh repo list owner
```

### Repository Management

```bash
# Create repository
gh repo create my-repo --public
gh repo create my-repo --private

# Sync fork
gh repo sync owner/repo

# Set default repository
gh repo set-default

# Configure the squash-merge commit message default
gh repo edit --squash-merge-commit-message COMMIT_MESSAGES
```

## Workflows & Actions

When the user asks to trigger, monitor, cancel, rerun, or download artifacts from GitHub Actions workflows and runs, see [references/workflows-actions.md](references/workflows-actions.md).

## Search

When the user asks to search GitHub repositories, issues, or pull requests, see [references/search.md](references/search.md).

## Labels

When the user asks to list, create, edit, delete, or clone repository labels, see [references/labels.md](references/labels.md).

## Codespaces

When the user asks to list, create, connect to, or manage files within GitHub Codespaces, see [references/codespaces.md](references/codespaces.md).

## Browse

Open repositories, files, and resources in the browser.

```bash
# Open current repo in browser
gh browse

# Open specific file
gh browse src/main.go

# Open file at specific line
gh browse src/main.go:42

# Open blame view for a file
gh browse --blame src/main.go

# Open Actions tab
gh browse --actions

# Open specific branch
gh browse --branch feature
```

## Releases

When the user asks to create, list, view, or download GitHub releases, see [references/releases.md](references/releases.md).

## Gists

When the user asks to create, list, view, or edit GitHub gists, see [references/gists.md](references/gists.md).

## Configuration

```bash
# Set default editor
gh config set editor vim

# Set default git protocol
gh config set git_protocol ssh

# View configuration
gh config list

# Set browser
gh config set browser firefox
```

## Advanced Features

When the user needs aliases, direct API access, extensions, secrets/variables, SSH/GPG keys, organization or project management, repository rulesets, attestations, Copilot Agent Tasks, `gh skill` management, reading repo file/directory contents without cloning, or issue types/sub-issues/blocking relationships, see [references/advanced-features.md](references/advanced-features.md).

## Automation & Scripting

When the user wants ready-made gh CLI automation patterns — code review workflows, issue triage, daily/weekly reports, bulk operations, CI monitoring, or team collaboration scripts, see [references/automation-workflows.md](references/automation-workflows.md).

## Troubleshooting

When gh fails with auth/permission/rate-limit errors, or hits command errors, installation/config issues, or network problems, see [references/troubleshooting.md](references/troubleshooting.md).

## Additional Resources

### Example Scripts

Practical automation scripts (see `examples/` directory):

- `auto-pr-create.sh` - Automated PR creation workflow
- `issue-triage.sh` - Bulk issue labeling and assignment
- `workflow-monitor.sh` - Watch and notify on workflow completion
- `release-automation.sh` - Complete release workflow automation

### External Documentation

- **Official Manual**: https://cli.github.com/manual
- **GitHub Community**: https://github.com/cli/cli/discussions
- **API Documentation**: https://docs.github.com/en/rest
- **Extension Marketplace**: https://github.com/topics/gh-extension

## JSON Output

When the user wants to use `--json` flags or needs the correct gh CLI JSON field names, see [references/json-output.md](references/json-output.md).

## Tips

1. Leverage interactive prompts by omitting parameters - most commands support interactive mode
2. Use `gh repo create --template` to scaffold from template repositories
3. Enable auto-merge with `gh pr merge --auto` for PRs that pass checks

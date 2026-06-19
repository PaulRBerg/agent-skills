---
disable-model-invocation: false
name: cli-gh
user-invocable: false
description: 'Use for GitHub CLI automation: gh commands, repo info, workflow triggers, GitHub search, codespaces, PR status, issues, repo browsing, or command-line GitHub tasks.'
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

**CRITICAL: This skill NEVER uses destructive gh CLI operations.**

This skill focuses exclusively on safe, read-only, or reversible GitHub operations. The following commands are **PROHIBITED** and must **NEVER** be used:

**Permanently destructive commands:**

- `gh repo delete` - Repository deletion
- `gh repo archive` - Repository archival
- `gh release delete` - Release deletion
- `gh release delete-asset` - Asset deletion
- `gh run delete` - Workflow run deletion
- `gh cache delete` - Cache deletion
- `gh secret delete` - Secret deletion
- `gh variable delete` - Variable deletion
- `gh label delete` - Label deletion
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

### Issue Types, Sub-Issues & Relationships

Issue types and sub-issues require GitHub.com or GHES 3.17+; blocking relationships require GHES 3.19+.

```bash
# Set or remove the issue type
gh issue edit 456 --type Bug
gh issue edit 456 --remove-type

# Create a sub-issue under a parent
gh issue create --parent 100

# Organize existing issues into a parent/child hierarchy
gh issue edit 100 --add-sub-issue 123,124
gh issue edit 100 --remove-sub-issue 123
gh issue edit 123 --parent 100
gh issue edit 123 --remove-parent

# Track blocked-by / blocking relationships
gh issue create --blocked-by 200,201 --blocking 300
gh issue edit 123 --add-blocked-by 200 --add-blocking 300,301
gh issue edit 123 --remove-blocked-by 200 --remove-blocking 301
```

## Discussions

When the user asks to list, view, create, edit, or comment on GitHub Discussions, see [references/discussions.md](references/discussions.md). The `gh discussion` command set is in preview and subject to change.

## Copilot Agent Tasks

Delegate work to the Copilot coding agent and track its sessions. The `gh agent-task` command set (aliases `gh agent`, `gh agents`) is in preview.

```bash
# Create an agent task on the current repository
gh agent-task create "Improve the performance of the data processing pipeline"

# List your most recent agent tasks
gh agent-task list
gh agent-task list --json id,name,state

# View an agent task session (by PR number, session ID, or URL)
gh agent-task view 123
gh agent-task view <session-id> --json state --jq '.state'
```

## Agent Skills

Discover, install, and publish agent skills from GitHub repositories. The `gh skill` command set (alias `gh skills`) is in preview.

```bash
# Search for skills across GitHub
gh skill search terraform

# Preview a skill before installing
gh skill preview github/awesome-copilot documentation-writer

# Install a skill (default scope: project)
gh skill install github/awesome-copilot documentation-writer
gh skill install owner/repo skill-name --scope user --pin v1.2.0

# Include skills in hidden dirs (.claude/skills/, .agents/skills/, .github/skills/)
gh skill install owner/repo skill-name --allow-hidden-dirs

# List installed skills and update them
gh skill list
gh skill update --all

# Validate and publish your own skills
gh skill publish --dry-run
```

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

### Reading Repo Contents

Read files and directories without cloning. The `gh repo read-file` and `gh repo read-dir` commands are in preview and subject to change.

```bash
# Read a file from the default branch (paged in a TTY, raw when piped)
gh repo read-file README.md --repo cli/cli

# Read from a specific branch, tag, or commit
gh repo read-file go.mod --ref v2.94.0 --repo cli/cli

# Write to disk instead of stdout (--clobber to overwrite)
gh repo read-file README.md --output ./README.md --clobber

# Refuse escape sequences by default; opt in for TTY/piped output
gh repo read-file script.sh --allow-escape-sequences

# List a directory (root when no path given)
gh repo read-dir script --repo cli/cli

# Inspect entries as JSON for scripting
gh repo read-dir docs --repo cli/cli --json name,path,type,size
```

## Search

When the user asks to search GitHub repositories, issues, or pull requests, see [references/search.md](references/search.md).

## Labels

When the user asks to list, create, edit, or clone repository labels, see [references/labels.md](references/labels.md).

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

## Quick Reference

Common gh operations at a glance:

| Operation         | Command                    | Common Flags                                |
| ----------------- | -------------------------- | ------------------------------------------- |
| Create PR         | `gh pr create`             | `--draft`, `--fill`, `--reviewer @copilot`  |
| List PRs          | `gh pr list`               | `--author @me`, `--label`, `--search`       |
| View PR           | `gh pr view <number>`      | `--web`, `--comments`                       |
| Merge PR          | `gh pr merge <number>`     | `--squash`, `--rebase`, `--delete-branch`   |
| Revert PR         | `gh pr revert <number>`    | `--body`                                    |
| Create issue      | `gh issue create`          | `--title`, `--body`, `--template`, `--type` |
| List issues       | `gh issue list`            | `--assignee @me`, `--label`, `--type`       |
| Close issue       | `gh issue close <number>`  | `--duplicate-of`, `--reason`                |
| View issue        | `gh issue view <number>`   | `--web`, `--comments`                       |
| Link sub-issue    | `gh issue edit <number>`   | `--parent`, `--add-sub-issue`               |
| Block issue       | `gh issue edit <number>`   | `--add-blocked-by`, `--add-blocking`        |
| List discussions  | `gh discussion list`       | `--answered`, `--sort`, `--json`            |
| Create agent task | `gh agent-task create`     | `--json` (on `list`/`view`)                 |
| Install skill     | `gh skill install`         | `--scope`, `--pin`, `--allow-hidden-dirs`   |
| Browse repo       | `gh browse`                | `--blame`, `--actions`, `--branch`          |
| Clone repo        | `gh repo clone <repo>`     | `--no-upstream`                             |
| Fork repo         | `gh repo fork`             | `--clone`, `--remote`                       |
| View repo         | `gh repo view`             | `--web`                                     |
| Read repo file    | `gh repo read-file <path>` | `--ref`, `--output`, `--clobber`, `--json`  |
| Read repo dir     | `gh repo read-dir [path]`  | `--ref`, `--json`                           |
| Create release    | `gh release create <tag>`  | `--title`, `--notes`, `--draft`             |
| Verify release    | `gh release verify <tag>`  | `--repo`                                    |
| Run workflow      | `gh workflow run <name>`   | `--ref`, `--field`                          |
| Watch run         | `gh run watch <id>`        | `--exit-status`                             |
| Search repos      | `gh search repos <query>`  | `--language`, `--stars`                     |
| Create label      | `gh label create <name>`   | `--color`, `--description`                  |
| Create codespace  | `gh codespace create`      | `--repo`, `--branch`                        |

## Additional Resources

### Reference Guides

For detailed patterns and advanced usage, see:

- **[Discussions](references/discussions.md)** - List, view, create, edit, and comment on GitHub Discussions (preview)
- **[Workflows & Actions](references/workflows-actions.md)** - GitHub Actions workflows, runs, cache management, and CI/CD integration patterns
- **[Advanced Features](references/advanced-features.md)** - Aliases, API access, extensions, secrets, SSH/GPG keys, organizations, projects, and advanced scripting
- **[Automation Workflows](references/automation-workflows.md)** - Common automation patterns, daily reports, release automation, and team collaboration workflows
- **[Troubleshooting](references/troubleshooting.md)** - Solutions for authentication, permissions, rate limiting, and common errors

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

1. Use `--web` flag to open items in browser for detailed view
2. Leverage interactive prompts by omitting parameters - most commands support interactive mode
3. Apply filters with `--author`, `--label`, `--state` to narrow down lists efficiently
4. Add `--json` flag to enable scriptable output for automation
5. **Always check `--help` for valid JSON field names** - they differ from GitHub API
6. Use `gh repo create --template` to scaffold from template repositories
7. Enable auto-merge with `gh pr merge --auto` for PRs that pass checks

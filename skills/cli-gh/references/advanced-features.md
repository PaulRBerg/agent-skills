# Advanced Features

> When to read: Read when the user needs aliases, direct API access, extensions, secrets/variables, SSH/GPG keys, organization or project management, repository rulesets, attestations, Copilot Agent Tasks, `gh skill` management, reading repo file/directory contents without cloning, or issue types/sub-issues/blocking relationships.

Advanced gh CLI capabilities for power users and automation.

## Aliases

Create custom shortcuts for frequently used commands.

```bash
# Create alias
gh alias set pv "pr view"
gh alias set bugs "issue list --label bug"

# List aliases
gh alias list

# Use alias
gh pv 123
```

## API Access

Direct access to GitHub's REST API through gh CLI.

```bash
# Make API call
gh api repos/:owner/:repo/issues

# With JSON data
gh api repos/:owner/:repo/issues -f title="Bug" -f body="Description"

# Paginated results
gh api --paginate repos/:owner/:repo/issues
```

## Extensions

Extend gh CLI functionality with community extensions.

```bash
# List extensions
gh extension list

# Install extension
gh extension install owner/gh-extension

# Upgrade extensions
gh extension upgrade --all
```

## Secrets and Variables

Manage GitHub Actions secrets and variables.

### Secrets

```bash
# List secrets
gh secret list

# Set secret
gh secret set SECRET_NAME

# Set secret from file
gh secret set SECRET_NAME < secret.txt
```

### Variables

```bash
# List variables
gh variable list

# Set variable
gh variable set VAR_NAME --body "value"
```

## SSH and GPG Keys

### SSH Keys

```bash
# List SSH keys
gh ssh-key list

# Add SSH key
gh ssh-key add ~/.ssh/id_ed25519.pub --title "My laptop"
```

### GPG Keys

```bash
# List GPG keys
gh gpg-key list

# Add GPG key
gh gpg-key add <key-file>
```

## Organizations

Manage organization settings and resources.

```bash
# List organizations
gh org list

# View organization info
gh org view <org-name>
```

## Projects

Work with GitHub Projects.

```bash
# List projects
gh project list --owner <org-name>

# View project
gh project view <project-number>

# Create project
gh project create --owner <org-name> --title "Project Name"

# List project items with a filter query
gh project item-list <project-number> --query "status:Done"
```

## Repository Rulesets

View information about repository rulesets.

```bash
# List rulesets
gh ruleset list

# View ruleset
gh ruleset view <ruleset-id>
```

## Attestations

Work with artifact attestations for supply chain security.

```bash
# Verify release attestation
gh release verify <tag>

# Verify specific asset
gh release verify-asset <file> --repo owner/repo
```

## Issue Types, Sub-Issues & Relationships

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

## Reading Repository Contents

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

## Advanced Scripting Patterns

### Using jq with gh

```bash
# Extract specific fields from JSON output
gh pr list --json number,title,author --jq '.[] | select(.author.login=="username")'

# Count open PRs
gh pr list --json state --jq 'length'

# Get PR numbers only
gh pr list --json number --jq '.[].number'
```

### Error Handling in Scripts

```bash
# Check if PR exists before operating
if gh pr view 123 &>/dev/null; then
  gh pr merge 123
else
  echo "PR not found"
fi
```

### Batch Operations

```bash
# Add label to multiple issues
gh issue list --assignee @me --json number --jq '.[].number' | xargs -I {} gh issue edit {} --add-label "in-progress"
```

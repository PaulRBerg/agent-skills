# Sablier Issue Workflow

This reference document describes the workflow for creating issues in `sablier-labs/*` repositories. The workflow always applies labels (user is org owner) and uses a default issue template since Sablier repos don't use GitHub issue templates.

## Validate Authentication

Check if GitHub CLI is authenticated:

```bash
gh auth status 2>&1 | rg -q "Logged in"
```

If not authenticated, error with: "Run `gh auth login` first"

## Parse Repository Argument

The **first token** in arguments is the repository name (without the org prefix).

- Extract the first token as `repo_name`
- Set `repository = sablier-labs/{repo_name}`
- Remove the first token from arguments (remaining text is the issue description)

Example: `lockup "Bug in cliff streams"` -> `repository = sablier-labs/lockup`

## Apply Labels

Labels are always applied for `sablier-labs/*` repositories. From content analysis, determine:

- **Type**: Primary category (bug, feature, docs, etc.)
- **Work**: Complexity via Cynefin (clear, complicated, complex, chaotic)
- **Priority**: Urgency (0=critical to 3=nice-to-have)
- **Effort**: Size (low, medium, high, epic)
- **Scope**: Domain area (only for `sablier-labs/command-center`)

### Label Reference

#### Type

- `type: bug` - Something isn't working
- `type: feature` - New feature or request
- `type: perf` - Performance or UX improvement
- `type: docs` - Documentation
- `type: test` - Test changes
- `type: refactor` - Code restructuring
- `type: build` - Build system or dependencies
- `type: ci` - CI configuration
- `type: chore` - Maintenance work
- `type: style` - Code style changes

#### Work (Cynefin)

- `work: clear` - Known solution
- `work: complicated` - Requires analysis but solvable
- `work: complex` - Experimental, unclear outcome
- `work: chaotic` - Crisis mode

#### Priority

- `priority: 0` - Critical blocker
- `priority: 1` - Important
- `priority: 2` - Standard work
- `priority: 3` - Nice-to-have

#### Effort

- `effort: low` - \<1 day
- `effort: medium` - 1-3 days
- `effort: high` - Several days
- `effort: epic` - Weeks, multiple PRs

#### Scope (sablier-labs/command-center only)

- `scope: frontend`
- `scope: backend`
- `scope: evm`
- `scope: solana`
- `scope: data`
- `scope: devops`
- `scope: integrations`
- `scope: marketing`
- `scope: business`
- `scope: other`

## Generate Title and Body

From remaining arguments, create:

### Title

Clear, concise summary (5-10 words).

### Body

Use this default template:

```
## Problem

[Extracted from user description]

## Solution

[If provided, otherwise "TBD"]

## Files Affected

<details><summary>Toggle to see affected files</summary>
<p>

- [{filename1}](https://github.com/sablier-labs/{repo_name}/blob/main/{path1})
- [{filename2}](https://github.com/sablier-labs/{repo_name}/blob/main/{path2})

</p>
</details>
```

**Admonitions**: Add GitHub-style admonitions when appropriate:

- `> [!NOTE]` - For context, dependencies, or implementation details users should notice
- `> [!TIP]` - For suggestions on testing, workarounds, or best practices
- `> [!IMPORTANT]` - For breaking changes, required migrations, or critical setup steps
- `> [!WARNING]` - For potential risks, known issues, or things that could go wrong
- `> [!CAUTION]` - For deprecated features, temporary solutions, or things to avoid

Place admonitions after the relevant section.

File links:

- **MUST** use markdown format: `[{filename}](https://github.com/sablier-labs/{repo_name}/blob/main/{path})`
- **Link text** should be the relative file path (e.g., `src/file.ts`, `docusaurus.config.ts`)
- **URL** must be the full GitHub URL
- List one per line if multiple files
- **OMIT the entire "## Files Affected" section** if no files are specified (e.g., for feature requests or planning issues)

## Create the Issue

```bash
gh issue create \
  --repo "sablier-labs/{repo_name}" \
  --title "$title" \
  --body "$body" \
  --label "label1,label2,label3"
```

Display: "Created: $URL"

On failure: show error and suggest fix

## Examples

```bash
# Bug report
lockup "Bug in stream creation for cliff durations"

# Feature request
command-center "Add dark mode toggle to dashboard"

# With --check flag
lockup --check "Support dynamic durations"

# Docs update
docs "Update integration guide for v2.2"
```

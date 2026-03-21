# Issue Creation Workflow

Create GitHub issues with automatic labeling, template detection, and intelligent content generation.

## Validate Prerequisites

See `commons.md > Auth Validation`.

## Parse Repository Argument

Determine repository from arguments:

- IF the first token matches "owner/repo": use it as repository and remove it from arguments
- ELSE: infer the current repository from the working directory (error if not in a repo)

> [!IMPORTANT]
> When dispatched via a repo-specific subcommand (e.g., `issue-codex`, `issue-cc`, `issue-biome`, `issue-sablier`), the target repository is predetermined by that subcommand's reference. Skip this section — do NOT infer from the working directory.

## Parse Optional Flags

IF arguments contain `--check`: remove it, set `check_mode = true`, continue to similarity check.
ELSE: skip similarity check and continue to template detection.

## Determine Authenticated User

```bash
gh api user -q .login
```

Store as `$AUTHENTICATED_USER` for later permission checks.

## Check for Similar Issues

**ONLY if `check_mode = true`:**

1. Extract key terms from remaining arguments

2. Search:

   ```bash
   gh search issues "{key_terms}" --repo "{owner}/{repo}" --state open --limit 10 --json number,title,url
   ```

3. IF found: display list, use `AskUserQuestion` ("Similar issues found. Proceed?"), cancel if "No"
4. IF none found: inform user, continue

## Check for Issue Templates

```bash
gh api repos/{owner}/{repo}/contents/.github/ISSUE_TEMPLATE --jq '.[].name | select(endswith(".md") or endswith(".yml") or endswith(".yaml"))' 2>/dev/null || true
```

Returns 404 for repos without templates — `|| true` ensures success. Exclude `config.yml`.

IF templates found:

### Select Template

Infer best match from user's description keywords (bug, feature, docs, etc.). Prefer YAML over Markdown if both exist.

### Parse Template

**YAML (`.yml`/`.yaml`):**

1. Fetch raw content:

   ```bash
   gh api repos/{owner}/{repo}/contents/.github/ISSUE_TEMPLATE/{template_name} --jq '.content' | base64 -d
   ```

2. Parse: `name`, `description`, `title` (prefix), `labels`, `body` array (fields with `type`, `id`, `attributes`)

3. Field types: `textarea`/`input` → section header from `attributes.label`; `dropdown` → select option from context; `checkboxes` → auto-acknowledge; `markdown` → skip

**Markdown (`.md`):** Fetch and populate template structure.

**No templates:** Use default structure (see Generate Title and Body).

## Check if Labels Should Be Applied

Extract owner from repository.

- IF owner = `$AUTHENTICATED_USER` OR owner = `sablier-labs`: **apply labels**. For `sablier-labs`, also apply Scope labels for `command-center` (see `issue-sablier.md`).
- ELSE: **skip labels** (only apply template-defined labels if any)

## Apply Labels

**ONLY if owner matches or is `sablier-labs`.**

From content analysis, determine Type, Work, Priority, and Effort.

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

## Generate Title and Body

See `commons.md > Informal Tone` for tone guidance.

### Title

If YAML template has `title` field (e.g., "[BUG] "), prepend it to a clear, concise summary (5-10 words).

### Body

**YAML template:** Generate markdown sections matching the `body` array fields — `### {field.attributes.label}` with content based on arguments and `field.attributes.description`. Skip `markdown` type fields.

**Markdown template:** Populate template structure with content from arguments.

**No template — default:**

```
## Problem

[Extracted from user description]

## Solution

[If provided, otherwise "TBD"]

## Files Affected

<details><summary>Toggle to see affected files</summary>
<p>

- [{filename}](https://github.com/{owner}/{repo}/blob/main/{path})

</p>
</details>
```

See `commons.md > GitHub Admonitions` for when/how to add admonitions. See `commons.md > File Link Formatting` for link rules. See `commons.md > Platform String Normalization` for OS fields.

## Create the Issue

Merge template-defined labels with auto-generated labels (deduplicate). Only include auto-labels if owner matches `$AUTHENTICATED_USER` or is `sablier-labs`:

```bash
gh issue create \
  --repo "$repository" \
  --title "$title" \
  --body "$body" \
  --label "label1,label2"  # omit --label entirely if no labels apply
```

Display: "Created: $URL"

On failure: show specific error and fix.

## Examples

```bash
# Basic usage (infers current repo)
"Bug in auth flow causing token expiration in src/auth/token.ts"

# Specify repository
PaulRBerg/dotfiles "Add zsh configuration for tmux startup"

# External repository
facebook/react "Add useDebounce hook to react-dom"

# With --check flag
--check "Bug in auth flow causing token expiration"

# External repo with --check
vercel/next.js --check "Improve error overlay for server components"
```

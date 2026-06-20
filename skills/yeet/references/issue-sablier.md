# Sablier Issue Workflow

Create issues in `sablier-labs/*` repositories. Labels are always applied (user is org owner). Sablier repos don't use GitHub issue templates.

## Validate Prerequisites

See `commons.md > Auth Validation`. The label fetch below is the auth check.

## Parse Repository Argument

The **first token** is the repo name (without org prefix) → `sablier-labs/{repo_name}`. Remove it from arguments; remaining text is the issue description.

Example: `lockup "Bug in cliff streams"` → `repository = sablier-labs/lockup`

## Apply Labels

Sablier repos are owner-managed — labels always apply. Follow `commons.md > Fetch Repo Labels` to fetch the live label set once and pick labels semantically. Scope labels (e.g., `scope: frontend`, `scope: evm`) are discovered organically from the fetched list for repos that define them — no special-case for `command-center`.

## Generate Title and Body

### Title

Clear, concise summary (5-10 words).

### Body

Default template:

```
## Problem

[Extracted from user description]

## Solution

[If provided, otherwise "TBD"]

## Files Affected

<details><summary>Toggle to see affected files</summary>
<p>

- [{filename}](https://github.com/sablier-labs/{repo_name}/blob/main/{path})

</p>
</details>
```

See `commons.md > GitHub Admonitions` for admonitions. See `commons.md > Task List Syntax` for progress-tracking checklists (`- [ ]` / `- [x]`). See `commons.md > Markdown Tables` for rendering tabular content as tables. See `commons.md > File Link Formatting` for link rules. Omit "Files Affected" if no files specified.

## Create the Issue

```bash
gh issue create \
  --repo "sablier-labs/{repo_name}" \
  --title "$title" \
  --body "$body" \
  --label "label1,label2,label3"
```

Display: "Created: $URL"

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

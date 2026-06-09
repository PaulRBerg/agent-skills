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

IF arguments contain `--image <path>`: remove each repeated flag/value pair, validate every path exists and is a file, store as `$image_paths`.

IF arguments contain `--image-release`: remove it, set `image_release_mode = true`. Use only when `--image` is also present; otherwise ignore it.

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

3. IF found: display the list as a heads-up, then continue (don't block on confirmation)

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

## Apply Labels

Extract owner from repository.

- IF owner = `$AUTHENTICATED_USER` OR owner = `sablier-labs`: continue.
- ELSE: **skip this section.** Only template-defined labels apply.

Fetch the repo's live label set per `commons.md > Fetch Repo Labels`, then pick labels by semantically matching the user's request against the fetched `name + description` pairs. One label per dimension when a clear axis exists in the repo; skip dimensions that don't apply; never invent labels.

Stash the selected labels for the `gh issue create` call below.

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

See `commons.md > GitHub Admonitions` for when/how to add admonitions. See `commons.md > Task List Syntax` for progress-tracking checklists (`- [ ]` / `- [x]`). See `commons.md > File Link Formatting` for link rules. See `commons.md > Platform String Normalization` for OS fields.

## Attach Images

**ONLY if `$image_paths` is non-empty.**

See `commons.md > Image Uploads` for constraints and fallback rules.

Default path: require `gh img` and upload before creating the issue.

```bash
gh img --repo "$repository" "$image_path"
```

For multiple images, run once with all image paths or once per image. Capture stdout as `$image_markdown`; each line is already ready to embed.

If `--image-release` was passed, use the GitHub Releases fallback instead. Create or reuse a clearly named asset release because the flag explicitly requested that side effect (`gh-issue-assets` is a reasonable default tag). Upload with unique filenames to avoid clobbering existing assets.

Insert the uploaded markdown into the generated body:

- If the selected template has a screenshot, image, reproduction, or media field, place the markdown there.

- Otherwise append:

  ```markdown
  ## Images

  ![image-name](https://github.com/user-attachments/assets/...)
  ```

On any upload failure, stop before issue creation and surface the upload error. Do not create the issue without the requested images.

## Create the Issue

Merge template-defined labels with the labels picked in "Apply Labels" (deduplicate). Omit `--label` entirely if no labels apply.

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

# With images
--image ./before.png --image ./after.png "CLI output is unreadable after resizing"
```

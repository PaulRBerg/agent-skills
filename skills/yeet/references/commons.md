# Common Patterns

Shared conventions used across all GitHub contribution workflows.

## Auth Validation

```bash
gh auth status 2>&1 | rg -q "Logged in"
```

If not authenticated, error with: "Run `gh auth login` first"

## Fetch Repo Labels

For owner-managed repos (owner = `$AUTHENTICATED_USER` or `sablier-labs`), fetch the live label set before picking labels — never hardcode taxonomies. External repos: skip this step entirely (only template-defined labels apply).

```bash
gh label list \
  --repo "{owner}/{repo}" \
  --limit 200 \
  --json name,description \
  --jq '.[] | "\(.name)\t\(.description // "")"'
```

`--limit 200` covers the largest owner-controlled repos (default is 30, which truncates silently). Both `name` and `description` are required — match the user's request semantically against descriptions, not just names.

### Picking labels

Treat the fetched list as the **only** source of truth. Read each `name + description` pair and select the smallest set that fits the request:

- **One label per dimension** if the repo's label set exposes a clear axis (e.g., a `type:` prefix, a `priority:` prefix, a `scope:` prefix). Skip dimensions the repo doesn't use.
- **Match on intent, not keywords.** "Auth flow is broken" → a `bug`-flavored label. "Add dark mode" → a `feature`/`enhancement`-flavored label. The exact slug comes from the repo, not from memory.
- **Never invent labels.** If nothing matches, apply none.
- **Skip workflow labels.** Labels like `good first issue`, `help wanted`, `needs triage`, `duplicate`, `wontfix`, `stale` are for maintainers, not for filing. Do not apply them on issue creation.
- **Respect template-defined labels.** If a template already assigns labels, merge (deduplicate) — don't override.

### Error handling

- **Empty result** (repo has no labels): proceed without `--label`. Do not fail the workflow.
- **Command fails** (auth, network, missing repo): fail loud. Surface the `gh` error and stop before issue creation — silent fallback hides misconfiguration. Per `Error Handling` below, do not retry.

## HEREDOC Syntax

Use HEREDOC when passing multi-line bodies to gh commands. Single quotes around `'EOF'` prevent variable expansion:

```bash
gh pr create --title "Title" --body "$(cat <<'EOF'
First paragraph

Second paragraph
EOF
)"
```

## GitHub Admonitions

Use GitHub-flavored admonitions to highlight important information. Apply judiciously — overuse reduces impact.

```markdown
> [!NOTE]
> Useful information that users should know

> [!TIP]
> Helpful advice for doing things better

> [!IMPORTANT]
> Key information users need to know

> [!WARNING]
> Urgent info that needs immediate attention

> [!CAUTION]
> Advises about risks or negative outcomes
```

## Task List Syntax

When an issue tracks progress on a task with multiple items — a checklist, a multi-step plan, acceptance criteria, subtasks, or a "remaining work" list — render those items as GitHub task lists, not plain bullets:

```markdown
- [ ] Pending item
- [x] Completed item
```

GitHub renders these as interactive checkboxes and surfaces a progress badge (e.g., `2 of 5`) in issue listings and references. Use them whenever the user frames the issue around tracking what's done vs. outstanding.

Use plain bullets (`- `) for non-trackable lists — affected files, links, options, context — where completion state is meaningless.

## Semantic Change Analysis

Read the actual diff to understand what changed — never generate content based solely on filenames or commit messages.

```bash
git diff origin/$base_branch...HEAD        # full diff
git diff --stat origin/$base_branch...HEAD  # summary
git log --pretty=format:"%s%n%b" origin/$base_branch...HEAD  # commit messages
```

Analyze:

- What files are affected and their purposes
- Bug fixes, features, refactors, or maintenance
- Core purpose and intent of the changes
- Breaking changes, migrations, or API changes
- The actual code to understand intent

**Title**: Concise summary using conventional commit format (e.g., `feat: add webhook retry mechanism`, `fix: prevent race condition in auth flow`). Use custom title from args if provided.

**Description**: Keep MINIMAL. 3-5 sentences total:

1. One sentence: what changed
2. One sentence: why it matters
3. Optional: one sentence about notable implementation detail or follow-up

**Issue references**: Extract from branch name (`git branch --show-current | rg -o '#?\d+'`) and commit messages. Format as "Closes #123" for fixes, "Related to #123" for references.

## Platform String Normalization

When templates include OS/platform fields:

- **macOS**: Use `scripts/get-macos-version.sh` → format `macOS <Name> v<Version>` (e.g., `macOS Tahoe v26.2`). Do not use `uname` output.
- **Linux**: Use `uname -mprs`
- **Windows**: Use PowerShell platform command output

> [!IMPORTANT]
> Skip platform/environment info entirely when the repo owner matches `$AUTHENTICATED_USER` or is `sablier-labs`. Omit the field in templates, and drop "Environment" sections in free-form bodies/comments. The user already knows their own machine — the noise only belongs in issues filed against external projects.

## Informal Tone

Write in an informal, casual style. Be direct and conversational — like explaining to a colleague, not drafting a spec.

**Good**: "This PR adds support for parsing YAML frontmatter in issue templates. Previously, we only supported markdown format, which meant users couldn't take advantage of GitHub's newer template features."

**Bad**: "This pull request implements functionality for YAML frontmatter parsing in the issue template processing subsystem. The implementation enhances the system's capabilities regarding template format support."

## File Link Formatting

- Use markdown format: `[{filename}](https://github.com/{owner}/{repo}/blob/main/{path})`
- Link text = relative file path (e.g., `src/file.ts`)
- One per line for multiple files
- Omit the "Files Affected" section entirely if no files are specified

## Image Uploads

GitHub has no public REST/GraphQL attachment API for issue or PR bodies. `gh issue create` only accepts markdown text, so images must be uploaded first and embedded as markdown.

Prefer `gh img` from `theolundqvist/gh-img` when available. It returns ready-to-embed markdown backed by GitHub `user-attachments/assets` URLs:

```bash
gh img --repo "{owner}/{repo}" "{image_path}"
```

If `gh img` is unavailable, check for `gh attach` from `atani/gh-attach` and use URL-only mode:

```bash
gh attach --repo "{owner}/{repo}" --issue "{issue_or_pr_number}" --image "{image_path}" --url-only
```

`gh attach --url-only` needs an existing issue or PR as browser context, so it is useful for comments and updates, not first-pass issue creation. Do not create a placeholder issue just to get an upload context.

Official API fallback: upload to a GitHub Release and embed the release asset URL. This creates or mutates release artifacts, so use it only when the user explicitly asks for an API-only fallback or the repo is owner-managed and this side effect is acceptable.

When image upload was requested and no acceptable upload path works, stop before posting/updating. Do not file an issue with missing screenshots.

## Error Handling

When operations fail, provide:

1. What was being attempted
2. What went wrong
3. What the user should do to fix it

Do not retry automatically.

## Posting and Feedback

- Post, create, and update directly when the user asks — do not gate the operation behind a confirmation prompt.
- After the operation, report what happened and link to the result.
- For `--check` search operations: surface any similar results as a heads-up, then proceed; don't block on confirmation and don't auto-skip on duplicates.

## Comment on Existing Issue

If a similar issue exists and the user prefers commenting over creating a duplicate:

```bash
gh issue comment {number} \
  --repo "{repo}" \
  --body "$(cat <<'EOF'
{comment body}
EOF
)"
```

Display: "Commented: https://github.com/{repo}/issues/{number}"

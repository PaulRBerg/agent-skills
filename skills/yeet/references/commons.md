# Common Patterns

Shared conventions used across all GitHub contribution workflows.

## Auth Validation

```bash
gh auth status 2>&1 | rg -q "Logged in"
```

If not authenticated, error with: "Run `gh auth login` first"

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

## Error Handling

When operations fail, provide:

1. What was being attempted
2. What went wrong
3. What the user should do to fix it

Do not retry automatically.

## Confirmation and Feedback

- Show what will happen before executing significant operations
- Ask for confirmation when appropriate
- For `--check` search operations: display results, let user decide, don't auto-skip on duplicates

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

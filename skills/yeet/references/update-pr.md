# Update Pull Request Workflow

Update existing GitHub pull requests with semantic change analysis, regenerating titles and descriptions based on actual
code changes.

## Validate Prerequisites

Same as `create-pr.md > Validate Prerequisites`, except the `gh pr view` read below can serve as the first auth check.

## Check for Existing PR

```bash
gh pr view --json number,url,title,baseRefName 2>/dev/null
```

IF no PR found: ERROR "No PR exists for this branch. Use `/yeet create-pr` to create one first." IF found: parse number,
URL, title, and base branch without emitting a separate discovery message.

## Parse Arguments Naturally

Interpret as natural language:

- References to "title" → update title
- References to "description" or "body" → regenerate description
- Quoted text → use as new title or append to description
- Everything else → additional context for description

## Semantic Change Analysis

Follow `writing.md > Semantic Change Analysis` with these differences:

1. Get base branch from PR metadata (not args)
2. Fetch only that base branch: `git fetch origin "+refs/heads/$base_branch:refs/remotes/origin/$base_branch"`
3. Preserve existing issue references (Closes #X, Related to #X) when regenerating

If user provided additional context in args, append it naturally to the description. Write the regenerated title and
body in the voice from `writing.md > Informal Tone`.

## Execute Update

```bash
# Title only
gh pr edit --title "$generated_title"

# Description only
gh pr edit --body "$generated_body"

# Both
gh pr edit --title "$generated_title" --body "$generated_body"
```

Display the verified URL with the `### ✅ PR updated` receipt from `SKILL.md` and name what changed.

**Push local commits:**

```bash
git push
```

On failure: check specific error, provide fix. Do not retry.

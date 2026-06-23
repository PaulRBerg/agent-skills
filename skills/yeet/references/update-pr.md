# Update Pull Request Workflow

Update existing GitHub pull requests with semantic change analysis, regenerating titles and descriptions based on actual code changes.

## Validate Prerequisites

Same as `create-pr.md > Validate Prerequisites`, except the `gh pr view` read below can serve as the first auth check.

## Check for Existing PR

```bash
gh pr view --json number,url,title,baseRefName 2>/dev/null
```

IF no PR found: ERROR "No PR exists for this branch. Use `/yeet create-pr` to create one first."
IF found: parse number, URL, title, base branch. Display: "Found PR #$number: $title"

## Parse Arguments Naturally

Interpret as natural language:

- References to "title" → update title
- References to "description" or "body" → regenerate description
- Quoted text → use as new title or append to description
- Everything else → additional context for description

## Semantic Change Analysis

Follow `commons.md > Semantic Change Analysis` with these differences:

1. Get base branch from PR metadata (not args)
2. Fetch only that base branch: `git fetch origin "+refs/heads/$base_branch:refs/remotes/origin/$base_branch"`
3. Preserve existing issue references (Closes #X, Related to #X) when regenerating

If user provided additional context in args, append it naturally to the description. Write the regenerated title and body in the voice from `commons.md > Informal Tone`.

## Execute Update

```bash
# Title only
gh pr edit --title "$generated_title"

# Description only
gh pr edit --body "$generated_body"

# Both
gh pr edit --title "$generated_title" --body "$generated_body"
```

Display: "Updated PR #$number: $PR_URL" with what was updated.

**Push local commits:**

```bash
git push 2>&1 || echo "No new commits to push"
```

On failure: check specific error, provide fix. Do not retry.

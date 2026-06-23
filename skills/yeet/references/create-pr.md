# Pull Request Workflow

Create GitHub pull requests with semantic change analysis, intelligent defaults, and minimal friction.

## Validate Prerequisites

See `commons.md > Auth Validation` for GitHub authentication. The repository context read below is the auth check.

**Check git repository state:**

- `git remote get-url origin` — confirm remote exists
- `git rev-parse --show-toplevel` — confirm we're in a repo

**Collect repo context once:**

```bash
scripts/yeet-context.sh repo
```

Use `repository.defaultBranchRef.name` as the default base branch unless args specify a base.

## Parse Arguments Naturally

- "draft" or "--draft" → draft mode
- "test-plan" or "--test-plan" → include test plan section
- "to X" or "base=X" → target branch X (default: repo default branch)
- If no base was passed, use `repository.defaultBranchRef.name` from repo context
- "review=X" or "reviewers=X" → add reviewer(s)
- Quoted text → custom title
- Everything else → additional context for description

## Fetch Base And Check Commits

Fetch the selected base branch only:

```bash
git fetch origin "+refs/heads/$base_branch:refs/remotes/origin/$base_branch"
```

- Commits ahead: `git rev-list --count origin/$base_branch..HEAD 2>/dev/null`
- IF 0 commits: ERROR "No commits to create PR from"

## Semantic Change Analysis

Follow the process in `commons.md > Semantic Change Analysis`. Write the title and body in the voice from `commons.md > Informal Tone`.

**Test Plan** (only if `--test-plan` flag): Add a "## Test Plan" section with testing/validation approach, manual steps, or checklist.

**Identify reviewers:**

- Check for CODEOWNERS file: `git ls-files | rg CODEOWNERS`
- If exists, extract owners for changed files
- Otherwise use git blame for frequent contributors
- Combine with reviewers from arguments

See `commons.md > GitHub Admonitions` for admonition usage in PR descriptions.

**Issue linking:** If an issue number was referenced in the conversation (e.g., "fixes #42", "for issue #100"), append `Closes #NUMBER` to the PR body so the issue auto-closes on merge.

## Check for Existing PR

```bash
gh pr list --head $(git branch --show-current) --json number,url --jq '.[0]' 2>/dev/null
```

IF existing PR found: ERROR "PR already exists for this branch: $URL". Do not create or update.

## Create New PR

**Push branch:**

```bash
git push -u origin $(git branch --show-current) 2>&1 || echo "Already pushed"
```

**Create PR:**

```bash
gh pr create \
  --title "$generated_title" \
  --body "$generated_body" \
  --base "$base_branch" \
  $(test "$draft_mode" = "true" && echo "--draft") \
  $(test -n "$reviewers" && echo "--reviewer $reviewers")
```

Display: "Created PR: $PR_URL"

On failure: check the specific error (auth, branch protection, validation) and follow [commons.md > Error Handling](commons.md#error-handling) — run the idempotency check before any retry.

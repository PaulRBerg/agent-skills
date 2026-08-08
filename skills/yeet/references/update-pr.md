# Update Pull Request Workflow

Update an existing pull request with semantic change analysis. Load [posting.md](posting.md) before any external write.

## Validate Prerequisites

Use the same checks as `create-pr.md > Validate Prerequisites`; the `gh pr view` read below is the authentication check.
Resolve `<skill-dir>` to the absolute directory containing the owning `SKILL.md`. Do not push by default: metadata
update is the default outcome.

## Check for Existing PR and Fetch Its Body

```bash
gh pr view --json number,url,title,body,baseRefName
```

If no PR is found, stop with `No PR exists for this branch. Use /yeet create-pr to create one first.` Cache the number,
URL, title, existing body, and base branch. The existing body is required input: preserve `Closes #X`, `Fixes #X`,
`Resolves #X`, `Related to #X`, and other issue references when regenerating the description. Never regenerate from the
user's instruction alone.

## Parse Arguments Naturally

Interpret as natural language:

- References to `title` → update title
- References to `description` or `body` → regenerate description
- Quoted text → use as a new title or append to the description
- `push`, `publish`, `publish code`, `publish commits`, or `publish branch` → explicit code-publication intent; push
  only after the disclosure review
- Everything else → additional context for the description

Without an explicit push or code-publication intent, update PR metadata only. Do not run unconditional `git push`.

## Semantic Change Analysis

Follow `writing.md > Semantic Change Analysis` with these differences:

1. Get the base branch from PR metadata, not args.
2. Fetch only that base branch:

   ```bash
   git fetch origin "+refs/heads/$base_branch:refs/remotes/origin/$base_branch"
   ```

3. Read the current PR body fetched above and preserve its issue references and any requested sections.
4. If the user provided additional context, append it naturally to the description.

Write the regenerated title and body in the voice from `writing.md > Informal Tone`.

## Execute Update

Run `posting.md > External-disclosure Review` on every changed field before this command:

```bash
# Title only
gh pr edit --title "$generated_title"

# Description only
gh pr edit --body-file "$generated_body_file"

# Both
gh pr edit --title "$generated_title" --body-file "$generated_body_file"
```

Verify the URL and requested fields with `gh pr view` and display the `### ✅ PR updated` receipt from `SKILL.md`.

## Explicit Code Publication

Only when the parsed intent includes push or publish, review the branch diff, commits, PR body, and any generated
artifacts again under `posting.md > External-disclosure Review`, then run:

```bash
git push
```

If the push or metadata edit fails ambiguously, follow `posting.md > Error Handling and Idempotency`, inspect the PR in
all states, and do not rerun a write until the resulting state is known.

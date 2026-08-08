# Issue Update Workflow

Update an existing GitHub issue — title, body, labels, assignees, or state. Mirrors `update-pr.md` in spirit: regenerate
content semantically when asked, otherwise apply targeted edits.

## Validate Prerequisites

See `context.md > Auth Validation`. The issue context read below is the auth check.

## Parse Arguments

Expected forms (same as `comment-issue.md`):

- `{owner}/{repo}#{number} {update instructions}`
- `{owner}/{repo} {number} {update instructions}`
- `#{number} {update instructions}` (infer repo from working directory)
- `{number} {update instructions}` (infer repo from working directory)
- `{url} {update instructions}` (parse owner/repo/number from the issue URL)

Rules:

- IF first token matches `https://github.com/{owner}/{repo}/issues/{number}` or `.../pull/{number}`: parse owner, repo,
  number from URL
- ELSE IF first token matches `{owner}/{repo}#{number}`: split on `#`
- ELSE IF first token matches `{owner}/{repo}`: use it as repository; next token must be the issue number (strip leading
  `#`)
- ELSE IF first token matches `#?{number}`: use it as issue number, infer repo from the local `origin` remote via
  `<skill-dir>/scripts/yeet-context.sh issue`
- ELSE: ERROR "Couldn't figure out the issue. Pass `owner/repo#123` or a GitHub issue URL."

Everything after the issue identifier is the **update instructions** — natural-language description of what to change.

## Fetch Issue Context

Always read the issue before editing — never regenerate based on the user's instructions alone.

```bash
<skill-dir>/scripts/yeet-context.sh issue "{owner}/{repo}" {number}
```

Resolve `<skill-dir>` to the absolute directory containing the owning `SKILL.md`. Before any write, load
`posting.md > External-disclosure Review` and `posting.md > Error Handling and Idempotency`.

## Interpret Update Instructions

Parse the instructions naturally — multiple intents may apply at once:

| Intent            | Cue words                                          | `gh issue edit` flag                                 |
| ----------------- | -------------------------------------------------- | ---------------------------------------------------- |
| Update title      | "title", "rename", quoted text passed as new title | `--title`                                            |
| Regenerate body   | "description", "body", "rewrite"                   | `--body`                                             |
| Append to body    | "add to body", "append"                            | `--body` (preserve existing + append)                |
| Add images        | `--image <path>`                                   | `--body` after shared image upload                   |
| Add labels        | "label X", "tag as X", "add label"                 | `--add-label`                                        |
| Remove labels     | "unlabel", "remove label"                          | `--remove-label`                                     |
| Set issue type    | "type X", "classify as X"                          | `--type`                                             |
| Remove issue type | "remove type"                                      | `--remove-type`                                      |
| Set parent        | "make sub-issue of X", "parent X"                  | `--parent`                                           |
| Remove parent     | "remove parent"                                    | `--remove-parent`                                    |
| Add sub-issues    | "add sub-issue X"                                  | `--add-sub-issue`                                    |
| Remove sub-issues | "remove sub-issue X"                               | `--remove-sub-issue`                                 |
| Add blocked-by    | "blocked by X"                                     | `--add-blocked-by`                                   |
| Remove blocked-by | "remove blocked-by X"                              | `--remove-blocked-by`                                |
| Add blocking      | "blocking X"                                       | `--add-blocking`                                     |
| Remove blocking   | "remove blocking X"                                | `--remove-blocking`                                  |
| Assign user       | "assign X", "assign to X", `@user`                 | `--add-assignee`                                     |
| Unassign user     | "unassign X"                                       | `--remove-assignee`                                  |
| Set milestone     | "milestone X"                                      | `--milestone`                                        |
| Close             | "close", "resolve"                                 | `gh issue close` (separate command)                  |
| Close duplicate   | "duplicate of X", "close as duplicate"             | `gh issue close --duplicate-of X --reason duplicate` |
| Reopen            | "reopen"                                           | `gh issue reopen` (separate command)                 |

If user provides only an issue identifier with no instructions, ERROR: "Tell me what to update — title, body, images,
labels, assignees, or state."

## Regenerate Title or Body

Only when the user explicitly asks for regeneration ("rewrite the body", "fix the title").

Follow `create-issue.md > Generate Title and Body` and `writing.md > Informal Tone`. Preserve any existing template
structure (sections, admonitions, file links). If the issue uses a YAML template's section headers, keep them.

For appends, preserve the existing body verbatim, then append the new content with a separator (blank line) — do not
rewrite or echo the full existing body to the user.

## Images

If images were requested, complete `context.md > Image Uploads` before editing the issue. Treat the resulting body as
the body update and combine it with any other requested edits in one `gh issue edit` command. If another instruction
regenerates the body, place the images in that regenerated body rather than the superseded original.

## Validate Labels Before Adding

If adding labels, fetch the repo's label set per `context.md > Fetch Repo Labels` and confirm the requested labels exist
(case-sensitive match on `name`). Skip this read for non-label edits.

IF a requested label doesn't exist: ERROR with the list of valid label names. Do not auto-create labels.

For owner-managed repos (owner = `viewer.login` from issue context or `sablier-labs`), when the user asks for a label by
intent rather than exact name ("tag this as a bug"), match semantically against the fetched `name + description` pairs
per the rubric in `context.md > Fetch Repo Labels`.

## Execute Update

```bash
# Title only
gh issue edit {number} --repo "{owner}/{repo}" --title "$new_title"

# Body only
gh issue edit {number} --repo "{owner}/{repo}" --body "$(cat <<'EOF'
{new body}
EOF
)"

# Labels
gh issue edit {number} --repo "{owner}/{repo}" \
  --add-label "label1,label2" \
  --remove-label "label3"

# Assignees
gh issue edit {number} --repo "{owner}/{repo}" \
  --add-assignee "user1" \
  --remove-assignee "user2"

# Type, hierarchy, and dependency metadata
gh issue edit {number} --repo "{owner}/{repo}" --type "Bug"
gh issue edit {number} --repo "{owner}/{repo}" --parent 100
gh issue edit {number} --repo "{owner}/{repo}" --add-sub-issue 123
gh issue edit {number} --repo "{owner}/{repo}" --add-blocked-by 200 --add-blocking 300

# Combined edit
gh issue edit {number} --repo "{owner}/{repo}" \
  --title "$new_title" \
  --body "$new_body" \
  --add-label "type: bug"
```

State changes use separate commands:

```bash
gh issue close {number}  --repo "{owner}/{repo}" [--comment "..."] [--reason "completed|not planned"]
gh issue reopen {number} --repo "{owner}/{repo}" [--comment "..."]

# Close as a duplicate and record the canonical issue
gh issue close {number} --repo "{owner}/{repo}" \
  --duplicate-of {canonical-number-or-url} --reason duplicate
```

See `writing.md > HEREDOC Syntax` for why the quoted `'EOF'` matters.

Display the verified URL with the `### ✅ Issue updated` receipt from `SKILL.md` and a one-line summary of what changed.

On failure: reread the issue, follow `posting.md > Error Handling and Idempotency`, and show the specific error (auth,
permissions, missing label, locked issue) and next action. Do not retry automatically.

## Examples

```bash
# Rename the title
42 "title: fix flaky token expiration"

# Add a label and assign yourself
vercel/next.js#12345 "label as type: bug, assign me"

# Regenerate the body with new context
sablier-labs/command-center#10 "rewrite body — root cause is the cache key, not the TTL"

# Close as not planned
https://github.com/facebook/react/issues/99999 "close, not planned"

# Append a note to the body
#42 "append: also reproduces on macOS Tahoe 26.2"
```

# Discussion Comment Workflow

Add a top-level comment, reply to a discussion comment, or edit an existing comment with `gh discussion comment`. Load
[posting.md](posting.md) before the write.

## Validate and Parse the Target

Requires authenticated GitHub CLI >= 2.97.0. Use the first discussion read as auth validation and resolve `<skill-dir>`
to the absolute directory containing the owning `SKILL.md`.

Accept these forms:

- `owner/repo#123 <comment>`
- `owner/repo 123 <comment>`
- `#123 <comment>` or `123 <comment>` (infer the repository from `origin`)
- `https://github.com/owner/repo/discussions/123 <comment>`
- A discussion comment URL or node ID for a reply or edit

Everything after the target is the comment context. Reject a missing target or empty body with a precise error. A
discussion number or URL targets a top-level comment. A comment URL or node ID targets a reply by default. Use `--edit`
only with an existing comment URL or node ID; deletion is not part of this workflow.

## Read Context

For a discussion target, fetch the discussion and recent comments before writing:

```bash
gh discussion view <number-or-url> --repo "<owner>/<repo>" --comments --json number,url,title,body,comments
```

For a comment target, fetch its thread and the parent discussion:

```bash
gh discussion view <comment-url-or-id> --repo "<owner>/<repo>" --json number,url,comments
```

Match the thread's register, avoid duplicate or unnecessary pings, and follow `writing.md > Informal Tone`.

## Review and Post

Run `posting.md > External-disclosure Review` on the exact comment body before posting. Use `--body-file` for multiline
content (or a quoted heredoc when supplying `--body`):

```bash
# Top-level comment
gh discussion comment <discussion-number-or-url> --repo "<owner>/<repo>" --body-file "<comment-file>"

# Reply to a comment
gh discussion comment <comment-url-or-id> --repo "<owner>/<repo>" --body-file "<reply-file>"

# Edit a comment or reply
gh discussion comment <comment-url-or-id> --repo "<owner>/<repo>" --edit --body-file "<new-body-file>"
```

Use `posting.md > Error Handling and Idempotency` after any nonzero, timeout, or connection-loss result. Reread the
discussion or comment thread and treat a matching authored body as a possible partial success; do not post a duplicate.
Verify the resulting comment URL or body with `gh discussion view`, then display the `### ✅ Comment posted` or
`### ✅ Comment updated` receipt from `SKILL.md`.

## Examples

```text
owner/repo#123 "This also reproduces on macOS."
https://github.com/owner/repo/discussions/123#discussioncomment-456 "Here's the missing reproduction step."
https://github.com/owner/repo/discussions/123#discussioncomment-456 --edit "Updated with the verified command."
```

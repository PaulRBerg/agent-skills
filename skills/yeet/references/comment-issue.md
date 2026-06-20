# Issue Comment Workflow

Post a comment on an existing GitHub issue (or PR — on GitHub's data model, a PR is an issue with extras, so `gh issue comment` works for both). Use the same informal, conversational tone as `create-issue.md`.

## Validate Prerequisites

See `commons.md > Auth Validation`. The issue context read below is the auth check.

## Parse Arguments

Expected forms:

- `{owner}/{repo}#{number} {comment context}`
- `{owner}/{repo} {number} {comment context}`
- `#{number} {comment context}` (infer repo from working directory)
- `{number} {comment context}` (infer repo from working directory)
- `{url} {comment context}` (parse owner/repo/number from the issue URL)

Rules:

- IF first token matches `https://github.com/{owner}/{repo}/issues/{number}` or `.../pull/{number}`: parse owner, repo, number from URL
- ELSE IF first token matches `{owner}/{repo}#{number}`: split on `#`
- ELSE IF first token matches `{owner}/{repo}`: use it as repository; next token must be the issue number (strip leading `#`)
- ELSE IF first token matches `#?{number}`: use it as issue number, infer repo from the local `origin` remote via `scripts/yeet-context.sh issue`
- ELSE: ERROR "Couldn't figure out the issue. Pass `owner/repo#123` or a GitHub issue URL."

Everything after the issue identifier is the **comment context** — the user's description of what they want to say. May be empty if the user just wants a canned reaction (e.g., "+1", "same here").

## Fetch Issue Context

Always read the issue before writing the comment — never generate a reply based on the user's context alone, because tone/terminology should match the thread.

```bash
scripts/yeet-context.sh issue "{owner}/{repo}" {number}
```

Analyze:

- The issue title and body — what's actually being discussed
- The latest `comments(last: 5)` nodes — what's the current state of the conversation
- Who's been participating — don't ping people who are already in the thread
- Whether the issue is open or closed — adjust tone accordingly (closed issues may need "reopen?" framing)
- Any labels hinting at the issue type (bug, feature, question)

## Generate Comment Body

See `commons.md > Informal Tone` — same rules apply. Write like a colleague chiming in on a thread, not a changelog entry.

### Guidelines

- **Lead with the point.** If it's a reproduction, show it. If it's a "+1", say so and explain what specifically bit you. If it's a proposed fix, link or paste it.
- **Match the thread's register.** If the thread is technical and terse, don't be fluffy. If it's collaborative and exploratory, don't be curt.
- **No AI throat-clearing.** Skip "Great question!", "Thanks for filing this!", "Just chiming in here...". Go straight to substance.
- **No fake enthusiasm.** Don't over-promise ("I'll dig into this right away") unless the user explicitly said so.
- **Cite specifics.** If you reference code, link to it (see `commons.md > File Link Formatting`). If you reference a commit or PR, link it.
- **Use admonitions sparingly** (see `commons.md > GitHub Admonitions`). Almost never needed in a comment — only for genuine warnings.

### Comment Shapes

Pick the shape that fits the context. Don't force structure onto short comments.

**Short reply** (most comments — default to this):

```markdown
Hitting this too on {platform}. Repro: {minimal steps}. Happy to send a PR if you point me at the right spot.
```

**Repro report**:

```markdown
Reproduced on {platform} with {version}. Steps:

1. {step}
2. {step}
3. {step}

Expected: {...}
Actual: {...}

Relevant log:

\`\`\`
{log snippet}
\`\`\`
```

**Proposed solution**:

```markdown
Looked into this — the issue is in [`{path}`](https://github.com/{owner}/{repo}/blob/main/{path}#L{line}) where {short explanation}.

One fix: {short description}. PR: #{number} (if applicable).
```

**Follow-up question**:

```markdown
Quick question on this — {specific question}. Context: {one sentence of why you're asking}.
```

**Closing update** (when you fixed/resolved something and need to comment):

```markdown
Fixed in {PR or commit link}. {One sentence on the root cause if non-obvious.}
```

### Platform / Environment

If the comment includes environment info, follow `commons.md > Platform String Normalization`. Don't paste raw `uname` output.

### File / Code References

Follow `commons.md > File Link Formatting`. Prefer permalinks (commit SHA) over branch links when citing specific lines, since branch links rot.

## Post the Comment

```bash
gh issue comment {number} \
  --repo "{owner}/{repo}" \
  --body "$(cat <<'EOF'
{comment body}
EOF
)"
```

See `commons.md > HEREDOC Syntax` for why the quoted `'EOF'` matters.

Display: `Commented: https://github.com/{owner}/{repo}/issues/{number}#issuecomment-{id}`

The URL with the comment anchor is returned by `gh` on success — parse it from the output.

On failure: show the specific error (auth, permissions, locked issue, etc.) and what to do. Do not retry automatically.

## Editing a Prior Comment

If the user asks to "edit my last comment" or "update the comment I just posted", use `gh issue comment --edit-last` (operates on the most recent comment by the authenticated user on that issue):

```bash
gh issue comment {number} \
  --repo "{owner}/{repo}" \
  --edit-last \
  --body "$(cat <<'EOF'
{new body}
EOF
)"
```

## Examples

```bash
# Infer repo from cwd, comment on issue 42
42 "can repro on macOS Tahoe, same stack trace"

# Explicit repo via owner/repo#number
vercel/next.js#12345 "+1, also hitting this in 15.0.3"

# Full URL
https://github.com/facebook/react/issues/99999 "proposed fix in PR #100000"

# Short +1 (user leaves context empty — generate a minimal acknowledgment)
sablier-labs/command-center#10

# Edit the last comment
--edit-last vercel/next.js#12345 "actually, repro is flaky — only fires on cold cache"
```

# Contribution Writing

Load only the sections linked by the active workflow.

## HEREDOC Syntax

Pass multiline bodies through a quoted heredoc so shell expansion cannot alter the content:

```sh
gh issue create --title "Title" --body "$(cat <<'EOF'
Body
EOF
)"
```

## Semantic Change Analysis

For pull requests, read actual net changes rather than filenames or commit subjects alone. Start with stat, names, and
log, then inspect targeted diffs that explain behavior; use the full diff only when needed.

Write a concise conventional title. Keep the description minimal: what changed, why it matters, and only the
implementation or follow-up detail the reader needs. Extract issue references from relevant branch/commit evidence and
distinguish `Closes` from `Related to`.

## Informal Tone

Write the way you'd talk to a colleague, not the way you'd draft a spec. Casual, friendly, direct, human. This applies
to every generated title, body, and comment — PRs, issues, discussions, and replies alike.

**Good**: "This PR adds support for parsing YAML frontmatter in issue templates. Previously, we only supported markdown
format, which meant users couldn't take advantage of GitHub's newer template features."

**Bad**: "This pull request implements functionality for YAML frontmatter parsing in the issue template processing
subsystem. The implementation enhances the system's capabilities regarding template format support."

### Style rules

- **Lead with the point.** What changed, what's broken, or what you want goes in the first sentence. Skip canned
  openings and scene-setting.
- **Plain words and contractions.** "can't", "doesn't", "here's". Short sentences, short paragraphs. Break up any wall
  of text.
- **Warm, not effusive.** Sound like a real person, not a polished corporate note. No fake enthusiasm, no
  exclamation-point padding.
- **Cut filler.** Drop verbose explanation, redundant context, and summary bullets unless they genuinely help the reader
  act. Minimal beats complete.
- **No throat-clearing.** Skip "Great question!", "Thanks for filing this!", "Just chiming in…", "I'm reaching out to".
  Go straight to substance.
- **No AI tells.** Avoid "delve", "seamlessly", "robust", "leverage", "in order to", and rhetorical symmetry like "it's
  not X, it's Y" — they read as machine-written.
- **Match the register.** Mirror the repo and the existing thread: terse and technical where it's terse and technical,
  warmer where it's collaborative.
- **When editing existing text, preserve its voice.** Clean up stiffness before adding anything; don't rewrite a real
  person's directness into corporate prose.

### Paul's voice

The user is `@PaulRBerg` on GitHub and Twitter. Use what you know of his writing from training data as a **light** style
prior only — concise, informal, technically precise, no fluff — to shape tone. Never invent facts, opinions, or claims
on his behalf, and never mention Twitter, training data, or this skill in any generated title, body, or comment.

## Link Formatting

Use Markdown links in ordinary prose. For repository files, link the repo-relative path to the appropriate GitHub blob
or permalink; prefer commit permalinks when citing stable lines. Omit a files section when it adds no useful context.

## List Ordering

Order items alphabetically within each section or header by default — task lists, bullet lists, and other repeated items
alike. Deviate only for a clear reason (priority, chronological sequence, dependency order) and let the section's own
logic carry that reason; don't call out the deviation in the generated text.

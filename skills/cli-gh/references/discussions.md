# Discussions

> When to read: Read when the user asks to list, view, create, edit, or comment on GitHub Discussions.

The `gh discussion` command set is in preview and subject to change. A discussion is supplied by number (`123`) or URL.

```bash
# List discussions (open by default; use --state all when checking all outcomes)
gh discussion list
gh discussion list --state all --answered
gh discussion list --sort created --order asc
gh discussion list --search "cache invalidation" --json number,title,category,answerChosenAt

# View a discussion, its comments, or replies to a comment
gh discussion view 123
gh discussion view 123 --comments
gh discussion view 123 --order oldest

# Create a discussion non-interactively
gh discussion create --title "My question" --category "Q&A" --body "Details here"
gh discussion create --title "Notes" --category general --body-file notes.md --label question

# Edit a discussion
gh discussion edit 123 --title "New title"
gh discussion edit 123 --add-label answered --remove-label question

# Add a top-level comment or reply to a comment URL/node ID
gh discussion comment 123 --body "Thanks!"
gh discussion comment <comment-url-or-id> --body "Reply text"

# Edit a comment or reply
gh discussion comment <comment-url-or-id> --edit --body "Updated"
```

Use `gh discussion list --search` for discussion searches. For JSON, use current fields such as `answerChosenAt`.

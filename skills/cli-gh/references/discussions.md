# Discussions

> When to read: Read when the user asks to list, view, create, edit, or comment on GitHub Discussions.

The `gh discussion` command set is in preview and subject to change. A discussion is supplied by number (`123`) or URL.

```bash
# List discussions
gh discussion list
gh discussion list --answered
gh discussion list --sort created --order asc
gh discussion list --json number,title,category,answeredAt

# View a discussion, its comments, or replies to a comment
gh discussion view 123
gh discussion view 123 --comments
gh discussion view 123 --order oldest

# Create a discussion (interactive when flags omitted)
gh discussion create
gh discussion create --title "My question" --category "Q&A" --body "Details here"
gh discussion create --title "Notes" --category "General" --body-file notes.md --label question

# Edit a discussion
gh discussion edit 123 --title "New title"
gh discussion edit 123 --add-label answered --remove-label question

# Comment on a discussion, or reply to a comment using its URL
gh discussion comment 123 --body "Thanks!"
gh discussion comment <comment-url> --body "Reply text"

# Edit or delete a comment
gh discussion comment <comment-url> --edit --body "Updated"
gh discussion comment <comment-url> --delete --yes
```

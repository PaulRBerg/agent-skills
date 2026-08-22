# Discussions

> When to read: Read when the user asks to list or view GitHub Discussions, or needs `gh discussion` syntax outside an
> authored contribution workflow. Use `yeet` to create, update, or comment on a discussion.

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

```

Use `gh discussion list --search` for discussion searches. For JSON, use current fields such as `answerChosenAt`.

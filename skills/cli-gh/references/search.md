# Search

> When to read: Read when the user asks to search GitHub repositories, issues, or pull requests using the `gh search` family of commands.

Search across all of GitHub for repositories, issues, and pull requests.

## Search Repositories

```bash
# Search for repositories
gh search repos "machine learning" --language=python

# Search with filters
gh search repos --stars=">1000" --topic=kubernetes
```

## Search Issues

```bash
# Search issues across GitHub
gh search issues "bug" --label=critical --state=open

# Exclude results (note the -- to prevent flag interpretation)
gh search issues -- "memory leak -label:wontfix"
```

## Search Pull Requests

```bash
# Search PRs
gh search prs --author=@me --state=open

# Search with date filters
gh search prs "refactor" --created=">2024-01-01"
```

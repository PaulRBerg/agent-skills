# Search

> When to read: Read when the user asks to search GitHub repositories, issues, or pull requests using the `gh search`
> family of commands.

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

### Scope to one repository

Two repo-scoped paths; pick by whether you want search-ranked results or a plain filtered list.

```bash
# Search API, scoped to a repo (relevance-ranked; repeat --repo for several)
gh search issues "panic" --repo cli/cli --state=open

# List a single repo's issues with a search filter (defaults to current repo)
gh issue list --repo cli/cli --search "is:open label:bug sort:created-desc"
```

`gh search issues` hits GitHub's search index (cross-repo, relevance-ranked, ~1000-result cap). `gh issue list --search`
lists one repo's issues with the same `is:`/`label:`/`sort:` qualifiers and honors `--limit`.

## Search Pull Requests

```bash
# Search PRs
gh search prs --author=@me --state=open

# Search with date filters
gh search prs "refactor" --created=">2024-01-01"
```

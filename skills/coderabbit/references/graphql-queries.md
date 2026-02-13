# GraphQL Queries for CodeRabbit Review Threads

Reference for fetching and filtering CodeRabbit review comments via GitHub's GraphQL and REST APIs.

## Fetch Review Threads

Primary query to get all review threads on a PR, including comment bodies and author info:

```bash
gh api graphql -f query='
  query($owner: String!, $repo: String!, $pr: Int!) {
    repository(owner: $owner, name: $repo) {
      pullRequest(number: $pr) {
        reviewThreads(first: 100) {
          pageInfo {
            hasNextPage
            endCursor
          }
          nodes {
            isResolved
            isOutdated
            resolvedBy {
              login
            }
            path
            line
            startLine
            diffSide
            comments(first: 50) {
              nodes {
                body
                author {
                  login
                }
                createdAt
                url
              }
            }
          }
        }
      }
    }
  }
' -f owner="{owner}" -f repo="{repo}" -F pr={pr_number}
```

Note: `-f` for string variables, `-F` for integer variables (this is a gh CLI convention — `-F` parses the value as a non-string JSON type).

The `isOutdated` field indicates whether the comment's referenced code has been updated since the comment was posted. Outdated threads are lower priority during classification — the code may have already been fixed by subsequent pushes.

## Author Filtering

Filter threads to CodeRabbit comments by checking the author login:

- GraphQL returns `coderabbitai` (without `[bot]` suffix)
- REST API returns `coderabbitai[bot]`
- Match both: check if author login starts with `coderabbitai`

Example jq filter for GraphQL response (checks if ANY comment in the thread is from CodeRabbit, not just the first):

```bash
| jq '[.data.repository.pullRequest.reviewThreads.nodes[] | select(any(.comments.nodes[]; .author.login == "coderabbitai"))]'
```

## REST Fallback: Inline Review Comments

When GraphQL returns incomplete data or for simpler access to inline comments:

```bash
gh api repos/{owner}/{repo}/pulls/{pr_number}/comments --paginate \
  --jq '[.[] | select(.user.login == "coderabbitai[bot]")]'
```

Returns flat list of inline comments with `path`, `line`, `body`, `created_at`, and `diff_hunk` fields.

## PR Review Bodies (Walkthrough Summaries)

CodeRabbit posts a top-level review body with a walkthrough summary. Fetch all reviews and filter:

```bash
gh api repos/{owner}/{repo}/pulls/{pr_number}/reviews --paginate \
  --jq '[.[] | select(.user.login == "coderabbitai[bot]") | {id: .id, body: .body, state: .state, submitted_at: .submitted_at}]'
```

The walkthrough summary typically appears in the first review body and contains:

- Summary of changes
- File-by-file change descriptions
- Sequence diagrams (when applicable)

## Resolution Detection

Three layers of resolution checking, applied in order:

**1. GraphQL `isResolved` field**

The primary signal. When `true`, the thread was explicitly resolved in GitHub's UI.

**2. Text markers**

Scan comment bodies for resolution indicators:

- "Addressed in commit"
- "Fixed in"
- "Resolved by"
- "This has been addressed"

**3. Checkbox markers**

CodeRabbit sometimes uses task lists in its comments:

- `[x]` — addressed/completed
- `[ ]` — still open

Layer 1 (`isResolved`) is authoritative. Layers 2 and 3 are heuristic — treat them as strong signals but verify by reading the full thread context before skipping. A comment saying "Fixed in a different PR" does not mean the issue is resolved in *this* PR.

## Pagination

For PRs with more than 100 review threads, use cursor-based pagination:

```bash
gh api graphql -f query='
  query($owner: String!, $repo: String!, $pr: Int!, $cursor: String) {
    repository(owner: $owner, name: $repo) {
      pullRequest(number: $pr) {
        reviewThreads(first: 100, after: $cursor) {
          pageInfo {
            hasNextPage
            endCursor
          }
          nodes {
            isResolved
            isOutdated
            path
            line
            startLine
            diffSide
            comments(first: 50) {
              nodes {
                body
                author {
                  login
                }
                createdAt
                url
              }
            }
          }
        }
      }
    }
  }
' -f owner="{owner}" -f repo="{repo}" -F pr={pr_number} -f cursor="{endCursor}"
```

Loop while `pageInfo.hasNextPage` is `true`, passing the `endCursor` value as the `cursor` variable.

For the initial request, omit the `-f cursor` parameter (or pass `null`). The API returns threads from the first page when no cursor is provided.

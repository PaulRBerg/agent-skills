# Discussion Creation Workflow

Create GitHub discussions using the GraphQL API with automatic category selection and optional template support.

## Validate Prerequisites

See `context.md > Auth Validation`. The repository context read below is the auth check.

## Parse Repository Argument

- If first token matches "owner/repo": use it as repository
- Otherwise: infer from the local `origin` remote via `{skill-dir}/scripts/yeet-context.sh repo`
- Error if not in a repo and no explicit repository provided

Parse `--check`; handle image options through `context.md > Image Uploads`.

## Collect Repository Context

Fetch repo id, categories, and discussion-template tree once:

```bash
{skill-dir}/scripts/yeet-context.sh repo "{owner}/{repo}" --discussion-categories --discussion-templates
```

If the repository was inferred from the current Git repo, omit `{owner}/{repo}`. Store `repository.id`,
`repository.discussionCategories.nodes`, and `repository.discussionTemplateTree.entries`.

## Check for Similar Discussions (Optional)

If `--check` flag is present:

1. Extract key terms from description

2. Search:

   ```bash
   gh search discussions "{key_terms}" --repo "{owner}/{repo}" --limit 10 --json number,title,url
   ```

3. IF found: display a compact `### 🔎 Similar discussions` table, say `Creation is continuing`, then continue without
   blocking on confirmation

4. IF none found: inform user, continue

## Select Discussion Category

Infer best category from description:

| Keywords                                                  | Category      |
| --------------------------------------------------------- | ------------- |
| "idea", "proposal", "suggest", "would be nice", "feature" | Ideas         |
| "how do I", "help", "question", "why does", "what is"     | Q&A           |
| "built", "made", "created", "sharing", "check out"        | Show and Tell |
| "vote", "poll", "which", "prefer"                         | Polls         |
| General conversation, feedback, meta-discussion           | General       |

Default to **General** or **Ideas** if uncertain.

## Check for Discussion Templates

Use `repository.discussionTemplateTree.entries` from the cached context. Keep entries ending in `.yml` or `.yaml`.

### If Templates Found

1. Select template matching category slug (e.g., `ideas.yml` for Ideas)

2. After selecting the template, fetch and parse:

   ```bash
   gh api repos/{owner}/{repo}/contents/.github/DISCUSSION_TEMPLATE/{template_name} --jq '.content' | base64 -d
   ```

3. Parse YAML: `title` (prefix), `body` array (fields with `type`, `id`, `attributes`)

4. Field types: `textarea`/`input` → section header; `dropdown` → select option; `checkboxes` → check only attestations
   verified from repository/user evidence; `markdown` → skip. If a required checkbox cannot be verified, stop for the
   missing fact rather than auto-acknowledging it.

### If No Templates Found

Use default structure (see below).

## Generate Title and Body

See `writing.md > Informal Tone` for tone guidance.

**Title**: If template has `title` field, prepend it. Otherwise create clear summary (5-10 words).

**Body with template**: Generate `### {field.attributes.label}` sections matching template fields.

**Body without template**:

```markdown
## Context

[What is this discussion about?]

## Discussion Points

[Key points or questions]

## Additional Context

[Background information, if applicable]
```

Use ordinary GitHub Markdown only where it improves this discussion. See `context.md > Platform String Normalization` if
OS details are required.

If images were requested, complete `context.md > Image Uploads` before creating the discussion.

## Create the Discussion

```bash
gh api graphql -f query='
  mutation($repositoryId: ID!, $categoryId: ID!, $title: String!, $body: String!) {
    createDiscussion(input: {
      repositoryId: $repositoryId
      categoryId: $categoryId
      title: $title
      body: $body
    }) {
      discussion { url }
    }
  }
' -f repositoryId="$REPO_ID" -f categoryId="$CAT_ID" -f title="$TITLE" -f body="$BODY"
```

Display the verified URL with the `### 🚀 Discussion created` receipt from `SKILL.md`.

## Examples

```bash
# Simple discussion in current repository
"Proposal for adding dark mode support"

# Explicit repository
PaulRBerg/dotfiles "Ideas for improving the zsh setup"

# Another repository
vercel/next.js "Question about server components caching"

# With --check flag
--check "How to configure custom routes"

# Explicit repository with --check
facebook/react --check "Proposal for new hook API"
```

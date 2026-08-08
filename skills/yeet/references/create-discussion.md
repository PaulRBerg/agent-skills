# Discussion Creation Workflow

Create a GitHub discussion with the native `gh discussion` command. Load [posting.md](posting.md) before the write.

## Validate Prerequisites

Requires authenticated GitHub CLI >= 2.97.0. See `context.md > Auth Validation`; the repository-context read is the auth
check. Handle image options through `context.md > Image Uploads`, then run `posting.md > External-disclosure Review` on
the final title, body, labels, and attachments.

## Parse Repository Argument

- If the first token matches `owner/repo`, use it as the repository.
- Otherwise infer the repository from the local `origin` remote via `<skill-dir>/scripts/yeet-context.sh repo`.
- Error if no repository can be inferred and none was supplied.

Parse `--check`; resolve `<skill-dir>` as the absolute directory containing the owning `SKILL.md`.

## Collect Repository Context

Fetch repository identity, live categories, and discussion-template entries once:

```bash
<skill-dir>/scripts/yeet-context.sh repo "<owner>/<repo>" --discussion-categories --discussion-templates
```

Cache `repository.discussionCategories.nodes` and `repository.discussionTemplateTree.entries`. Treat every live
category's `name`, `slug`, and `description` as authoritative. Do not use a static category table.

## Check for Similar Discussions (Optional)

If `--check` is present, search title/body terms in every state and show matches without adding a confirmation gate:

```bash
gh discussion list --repo "<owner>/<repo>" --state all --search "<key terms>" \
  --limit 10 --json number,title,url,closed
```

Display matches under `### 🔎 Similar discussions`, say `Creation is continuing`, and continue. If the create command
later fails, follow `posting.md > Error Handling and Idempotency` before retrying; never recreate a possible match.

## Select Discussion Category

Infer a category from the user's title and description by matching the live `name`, `slug`, and `description`. A
supplied category may match its exact name or slug. If no category matches, or more than one category is plausible, stop
and ask the user to choose from the live candidates with their descriptions. Never default to a guessed or invented
category.

## Check for Discussion Templates

Keep live template-tree entries ending in `.yml` or `.yaml`. A discussion form filename normally matches the category
slug. If multiple templates match, stop and ask the user to choose; if none matches, use the selected category without a
form.

For a selected form, fetch it from the default branch:

```bash
gh api repos/<owner>/<repo>/contents/.github/DISCUSSION_TEMPLATE/<template-name> --jq '.content' | base64 -d
```

Parse `title`, `labels`, and `body`. If labels are declared, fetch the live label set with
`<skill-dir>/scripts/yeet-context.sh labels "<owner>/<repo>"`; preserve declared labels whose exact live names match,
deduplicate, and pass them with `--label`. Stop for a declared label that is not live rather than inventing it. For
`textarea`/`input`, render a section header; for `dropdown`, select only a live option; for `checkboxes`, check an
attestation only when repository or user evidence verifies it. Stop for any required checkbox whose attestation cannot
be verified. Skip `markdown` fields.

## Generate Title and Body

See `writing.md > Informal Tone`.

If the form supplies a title prefix, prepend it. Otherwise write a clear 5–10-word title. Render form fields as
`### <field label>` sections. Without a form, use:

```markdown
## Context

[What is this discussion about?]

## Discussion Points

[Key points or questions]

## Additional Context

[Background information, if applicable]
```

If images were requested, complete `context.md > Image Uploads` before the disclosure review.

## Create and Verify the Discussion

Use the live category name or slug and the native command:

```bash
gh discussion create --repo "<owner>/<repo>" \
  --category "<category-name-or-slug>" \
  --title "<title>" \
  --body-file "<body-file>" \
  --label "<template-labels>"
```

Omit `--label` when no labels apply. Read the returned URL (or list the distinctive title in `--state all`) to verify
the discussion, then display the `### 🚀 Discussion created` receipt from `SKILL.md`. On any ambiguous result, follow
`posting.md` and do not rerun creation.

## Examples

```text
# Simple discussion in current repository
"Proposal for adding dark mode support"

# Explicit repository
PaulRBerg/dotfiles "Ideas for improving the zsh setup"

# With a non-blocking duplicate search
--check "How to configure custom routes"
```

# Discussion Update Workflow

Update an existing GitHub discussion's title, body, category, labels, or images with `gh discussion edit`. Discussion
state changes and comment editing are outside this workflow.

## Validate Prerequisites

The installed GitHub CLI must provide `gh discussion view` and `gh discussion edit`. These commands are in preview; if
either is unavailable, stop and tell the user to upgrade GitHub CLI rather than inventing a GraphQL fallback. The
discussion read below is the authentication check.

## Parse Arguments

Accept these forms:

- `{owner}/{repo}#{number} {update instructions}`
- `{owner}/{repo} {number} {update instructions}`
- `#{number} {update instructions}` (infer the repository from the current directory)
- `{number} {update instructions}` (infer the repository from the current directory)
- `https://github.com/{owner}/{repo}/discussions/{number} {update instructions}`

For a URL, parse the owner, repository, and number. For `owner/repo#number`, split on `#`. For `owner/repo number`, use
the first two tokens. For a local number, resolve the repository with `{skill-dir}/scripts/yeet-context.sh repo`. Reject
other targets with: `Couldn't figure out the discussion. Pass owner/repo#123 or a GitHub discussion URL.` Everything
after the target is the natural-language update instruction; parse repeated `--image <path>` and optional
`--image-release` through `context.md > Image Uploads`.

## Fetch Discussion Context

Always read the discussion before editing:

```sh
gh discussion view <number> --repo "<owner>/<repo>" \
  --json number,url,title,body,category,labels
```

Keep the existing body verbatim unless the user asks to rewrite or append to it.

## Interpret and Validate Updates

Multiple intents may apply at once:

| Intent          | Cue or input                       | `gh discussion edit` flag              |
| --------------- | ---------------------------------- | -------------------------------------- |
| Update title    | "title", "rename"                  | `--title`                              |
| Regenerate body | "description", "body", "rewrite"   | `--body-file`                          |
| Append to body  | "add to body", "append"            | `--body-file` with existing + appended |
| Change category | "category"                         | `--category`                           |
| Add labels      | "label X", "tag as X", "add label" | `--add-label`                          |
| Remove labels   | "unlabel", "remove label"          | `--remove-label`                       |
| Add images      | `--image <path>`                   | `--body-file` after shared upload      |

If no update instruction remains, stop with: `Tell me what to update — title, body, category, labels, or images.`

For a body rewrite, follow `writing.md > Informal Tone` and preserve recognizable template structure. For an append,
retain the current body byte-for-byte, add a blank line, then add the requested content. If images were requested,
complete `context.md > Image Uploads` and treat its result as the body update; when combined with a rewrite, place the
images in the regenerated body.

For a category change, fetch live categories with:

```sh
{skill-dir}/scripts/yeet-context.sh repo "<owner>/<repo>" --discussion-categories
```

Match the requested category against the live name or slug and reject an unknown category. For label additions, follow
`context.md > Fetch Repo Labels`; reject unknown labels and do not create them. Removal may name only labels currently
present on the discussion.

## Execute and Verify

Build one non-interactive command containing every requested edit:

```sh
gh discussion edit <number> --repo "<owner>/<repo>" \
  [--title "<title>"] \
  [--body-file "<body-file>"] \
  [--category "<category>"] \
  [--add-label "<label1,label2>"] \
  [--remove-label "<label3>"]
```

After the edit, repeat the context read and verify every requested field. Display its URL with the
`### ✅ Discussion updated` receipt from `SKILL.md` and one line naming the changed fields.

On failure, read the discussion again before any retry to determine whether GitHub applied the update. Do not retry
automatically; report the concrete error, observed state, and next action.

## Examples

```text
# Rename and recategorize
owner/repo#42 "rename to 'CLI roadmap', category Ideas"

# Append images without rewriting the existing body
#42 --image ./before.png --image ./after.png "append these comparisons"

# Replace the body and add a label
https://github.com/owner/repo/discussions/42 "rewrite body and add label documentation"
```

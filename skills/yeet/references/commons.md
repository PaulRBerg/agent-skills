# Common Patterns

Shared conventions used across all GitHub contribution workflows.

## Auth Validation

Do not run unconditional `gh auth status`. Treat the first required read-only `gh` command in the workflow as auth validation.

When repository, template, discussion, label, or issue/PR thread context is needed, prefer:

```bash
scripts/yeet-context.sh repo "{owner}/{repo}" [--issue-templates] [--discussion-templates] [--discussion-categories]
scripts/yeet-context.sh issue "{owner}/{repo}" {number}
scripts/yeet-context.sh labels "{owner}/{repo}"
```

If the helper fails with an auth error, stop with: "Run `gh auth login` first".

## Repository Context

Collect repository context once per workflow and reuse it. The `repo` helper output includes:

- `viewer.login` — the authenticated GitHub login
- `repository.id`
- `repository.nameWithOwner`
- `repository.viewerPermission`
- `repository.defaultBranchRef.name`
- optional issue/discussion template tree entries with `name`, `oid`, and `type`
- optional discussion categories

For workflows in the current Git repository, `scripts/yeet-context.sh repo` can infer `{owner}/{repo}` from the local `origin` remote before the GraphQL read. For explicit targets, pass `{owner}/{repo}`.

## Fetch Repo Labels

Fetch labels only when labels may actually be applied:

- owner-managed repos (`owner = viewer.login` or `owner = sablier-labs`)
- requested label edits
- selected templates that define labels and `repository.viewerPermission` is label-capable

External repos without label-capable permission: skip this step entirely and omit labels. Never hardcode taxonomies.

```bash
scripts/yeet-context.sh labels "{owner}/{repo}"
```

The helper preserves the current `gh label list --limit 200 --json name,description` behavior. Both `name` and `description` are required — match the user's request semantically against descriptions, not just names.

### Picking labels

Treat the fetched list as the **only** source of truth. Read each `name + description` pair and select the smallest set that fits the request:

- **One label per dimension** if the repo's label set exposes a clear axis (e.g., a `type:` prefix, a `priority:` prefix, a `scope:` prefix). Skip dimensions the repo doesn't use.
- **Match on intent, not keywords.** "Auth flow is broken" → a `bug`-flavored label. "Add dark mode" → a `feature`/`enhancement`-flavored label. The exact slug comes from the repo, not from memory.
- **Never invent labels.** If nothing matches, apply none.
- **Skip workflow labels.** Labels like `good first issue`, `help wanted`, `needs triage`, `duplicate`, `wontfix`, `stale` are for maintainers, not for filing. Do not apply them on issue creation.
- **Respect template-defined labels.** If a template already assigns labels, merge (deduplicate) — don't override.

### Error handling

- **Empty result** (repo has no labels): proceed without `--label`. Do not fail the workflow.
- **Command fails** (auth, network, missing repo): fail loud. Surface the `gh` error and stop before issue creation — silent fallback hides misconfiguration. Per `Error Handling` below, do not retry.

## Template Metadata And Issue Forms

GitHub issue-form YAML can define top-level `labels` and `type`. The native web form applies that metadata, but `gh issue create --body` / `--body-file` posts a body directly and does not execute the form.

Current `gh issue create --template` uses a template as interactive/editor starting body text. It is rejected with `--body` / `--body-file`, and it is not a non-interactive YAML issue-form submission API. For deterministic agent-created issues, fetch YAML forms, render matching markdown headings, and pass supported metadata explicitly.

Use the cached `repository.viewerPermission` from `scripts/yeet-context.sh repo`. Treat `ADMIN`, `MAINTAIN`, `WRITE`, and `TRIAGE` as label-capable; omit `--label` for `READ`. If a label create fails anyway, run the idempotency check before retrying once without labels. When a YAML form has top-level `type`, pass `--type` if the repo supports issue types.

## HEREDOC Syntax

Use HEREDOC when passing multi-line bodies to gh commands. Single quotes around `'EOF'` prevent variable expansion:

```bash
gh pr create --title "Title" --body "$(cat <<'EOF'
First paragraph

Second paragraph
EOF
)"
```

## GitHub Admonitions

Use GitHub-flavored admonitions to highlight important information. Apply judiciously — overuse reduces impact.

```markdown
> [!NOTE]
> Useful information that users should know

> [!TIP]
> Helpful advice for doing things better

> [!IMPORTANT]
> Key information users need to know

> [!WARNING]
> Urgent info that needs immediate attention

> [!CAUTION]
> Advises about risks or negative outcomes
```

## Task List Syntax

When an issue tracks progress on a task with multiple items — a checklist, a multi-step plan, acceptance criteria, subtasks, or a "remaining work" list — render those items as GitHub task lists, not plain bullets:

```markdown
- [ ] Pending item
- [x] Completed item
```

GitHub renders these as interactive checkboxes and surfaces a progress badge (e.g., `2 of 5`) in issue listings and references. Use them whenever the user frames the issue around tracking what's done vs. outstanding.

Use plain bullets (`- `) for non-trackable lists — affected files, links, options, context — where completion state is meaningless.

## Markdown Tables

GitHub renders pipe tables natively, and they scan far better in the web UI than the same data repeated across prose or nested bullets. When body content is naturally tabular — several items sharing one set of attributes (option/trade-off comparisons, before/after, config key → meaning, env var → value, version → status, benchmark numbers) — render it as a table so readers can take it in at a glance:

```markdown
| Option     | Pros                | Cons                 |
| ---------- | ------------------- | -------------------- |
| Approach A | Simple, no new deps | Slower on cold start |
| Approach B | Fast                | Adds a dependency    |
```

Reach for a table only when the data has a real second dimension. Don't force a flat list, a single key/value pair, or a free-flowing explanation into one — bullets or prose read better there. Keep the columns to what actually varies across rows.

## Semantic Change Analysis

Read the actual diff to understand what changed — never generate content based solely on filenames or commit messages.

```bash
git diff --stat origin/$base_branch...HEAD  # summary
git diff --name-only origin/$base_branch...HEAD
git log --pretty=format:"%s%n%b" origin/$base_branch...HEAD  # commit messages
git diff origin/$base_branch...HEAD -- path/to/important/file
```

Read stat, name-only, and log first. Then inspect targeted full diffs for the files or packages that explain the change. Fall back to the full diff only when the targeted reads leave behavior unclear.

Analyze:

- What files are affected and their purposes
- Bug fixes, features, refactors, or maintenance
- Core purpose and intent of the changes
- Breaking changes, migrations, or API changes
- The actual code to understand intent

**Title**: Concise summary using conventional commit format (e.g., `feat: add webhook retry mechanism`, `fix: prevent race condition in auth flow`). Use custom title from args if provided.

**Description**: Keep MINIMAL. 3-5 sentences total:

1. One sentence: what changed
2. One sentence: why it matters
3. Optional: one sentence about notable implementation detail or follow-up

**Issue references**: Extract from branch name (`git branch --show-current | rg -o '#?\d+'`) and commit messages. Format as "Closes #123" for fixes, "Related to #123" for references.

## Platform String Normalization

When templates include OS/platform fields:

- **macOS**: Use `scripts/get-macos-version.sh` → format `macOS <Name> v<Version>` (e.g., `macOS Tahoe v26.2`). Do not use `uname` output.
- This catalog is macOS-first. Preserve upstream template enum values when required, but do not add Linux/Windows environment details unless the user explicitly asks to file from another machine.

> [!IMPORTANT]
> Skip platform/environment info entirely when the repo owner matches cached `viewer.login` or is `sablier-labs`. Omit the field in templates, and drop "Environment" sections in free-form bodies/comments. The user already knows their own machine — the noise only belongs in issues filed against external projects.

## Informal Tone

Write the way you'd talk to a colleague, not the way you'd draft a spec. Casual, friendly, direct, human. This applies to every generated title, body, and comment — PRs, issues, discussions, and replies alike.

**Good**: "This PR adds support for parsing YAML frontmatter in issue templates. Previously, we only supported markdown format, which meant users couldn't take advantage of GitHub's newer template features."

**Bad**: "This pull request implements functionality for YAML frontmatter parsing in the issue template processing subsystem. The implementation enhances the system's capabilities regarding template format support."

### Style rules

- **Lead with the point.** What changed, what's broken, or what you want goes in the first sentence. Skip canned openings and scene-setting.
- **Plain words and contractions.** "can't", "doesn't", "here's". Short sentences, short paragraphs. Break up any wall of text.
- **Warm, not effusive.** Sound like a real person, not a polished corporate note. No fake enthusiasm, no exclamation-point padding.
- **Cut filler.** Drop verbose explanation, redundant context, and summary bullets unless they genuinely help the reader act. Minimal beats complete.
- **No throat-clearing.** Skip "Great question!", "Thanks for filing this!", "Just chiming in…", "I'm reaching out to". Go straight to substance.
- **No AI tells.** Avoid "delve", "seamlessly", "robust", "leverage", "in order to", and rhetorical symmetry like "it's not X, it's Y" — they read as machine-written.
- **Match the register.** Mirror the repo and the existing thread: terse and technical where it's terse and technical, warmer where it's collaborative.
- **When editing existing text, preserve its voice.** Clean up stiffness before adding anything; don't rewrite a real person's directness into corporate prose.

### Paul's voice

The user is `@PaulRBerg` on GitHub and Twitter. Use what you know of his writing from training data as a **light** style prior only — concise, informal, technically precise, no fluff — to shape tone. Never invent facts, opinions, or claims on his behalf, and never mention Twitter, training data, or this skill in any generated title, body, or comment.

## File Link Formatting

- Use markdown format: `[{filename}](https://github.com/{owner}/{repo}/blob/main/{path})`
- Link text = relative file path (e.g., `src/file.ts`)
- One per line for multiple files
- Omit the "Files Affected" section entirely if no files are specified

## Image Uploads

GitHub has no public REST/GraphQL attachment API for issue or PR bodies. `gh issue create` only accepts markdown text, so images must be uploaded first and embedded as markdown.

Prefer `gh img` from `theolundqvist/gh-img` when available. It returns ready-to-embed markdown backed by GitHub `user-attachments/assets` URLs:

```bash
gh img --repo "{owner}/{repo}" "{image_path}"
```

If `gh img` is unavailable, check for `gh attach` from `atani/gh-attach` and use URL-only mode:

```bash
gh attach --repo "{owner}/{repo}" --issue "{issue_or_pr_number}" --image "{image_path}" --url-only
```

`gh attach --url-only` needs an existing issue or PR as browser context, so it is useful for comments and updates, not first-pass issue creation. Do not create a placeholder issue just to get an upload context.

Official API fallback: upload to a GitHub Release and embed the release asset URL. This creates or mutates release artifacts, so use it only when the user explicitly asks for an API-only fallback or the repo is owner-managed and this side effect is acceptable.

When image upload was requested and no acceptable upload path works, stop before posting/updating. Do not file an issue with missing screenshots.

## Error Handling

When operations fail, provide:

1. What was being attempted
2. What went wrong
3. What the user should do to fix it

Do not retry automatically, with one scoped exception: a label error may be retried once without the offending label, after the idempotency check below confirms nothing was created.

### Idempotency on Retry

A failed `gh issue create` or `gh pr create` may still have created the artifact (e.g. creation succeeded but a follow-up step like labeling failed). Before any retry, check whether it already exists:

```bash
gh issue list --repo "{repo}" --author "@me" --limit 5
gh pr list --repo "{repo}" --head "{branch}"
```

If the artifact exists, switch to the update/comment workflow — never re-create.

### Common Errors

| Error                                   | Remedy                                                                         |
| --------------------------------------- | ------------------------------------------------------------------------------ |
| `Not logged in` / 401                   | Tell the user to run `gh auth login` (see [Auth Validation](#auth-validation)) |
| `label not found` / 422                 | Re-fetch repo labels, drop the offending label, retry once                     |
| 403 applying labels on an external repo | Create without `--label`; mention the intended labels in the body              |
| 404 on the repo                         | Verify the target with `gh repo view {repo}`                                   |
| Rate limit exceeded                     | Stop and report the reset time (`gh api rate_limit`)                           |

## Posting and Feedback

- Post, create, and update directly when the user asks — do not gate the operation behind a confirmation prompt.
- After the operation, report what happened and link to the result.
- For `--check` search operations: surface any similar results as a heads-up, then proceed; don't block on confirmation and don't auto-skip on duplicates.

## Comment on Existing Issue

If a similar issue exists and the user prefers commenting over creating a duplicate:

```bash
gh issue comment {number} \
  --repo "{repo}" \
  --body "$(cat <<'EOF'
{comment body}
EOF
)"
```

Display: "Commented: https://github.com/{repo}/issues/{number}"

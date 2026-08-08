# Issue Creation Workflow

Create a GitHub issue from repository evidence and the selected live template.

## Context and Selection

Parse an optional leading `owner/repo`; otherwise infer the current repository. Repo-specific workflows supply their
fixed target and never infer it. Fetch authenticated context once:

```sh
<skill-dir>/scripts/yeet-context.sh repo "<owner>/<repo>" --issue-templates
```

Resolve `<skill-dir>` to the absolute directory containing the owning `SKILL.md`. Cache viewer login, permission,
default branch, and template entries. Parse `--check`; handle image options through `context.md > Image Uploads`. With
`--check`, search similar issues in all states and show results without adding a confirmation gate.

Select the best template from the user's intent. This is an agent decision. Prefer YAML when a suitable YAML and
Markdown template coexist. Exclude `config.yml`.

## YAML Issue Forms

Inspect the selected live form; do not mirror its schema in prose:

```sh
uv run "<skill-dir>/scripts/issue-form.py" inspect \
  --repo "<owner>/<repo>" --template "<name>.yml" > <form.json>
```

The form JSON reports title prefix, assignees, labels, projects, issue type, field IDs, descriptions, render modes,
defaults, dropdown options, upload constraints, multi-select behavior, required flags, and checkbox attestations.
Compose answers in a separate JSON object keyed by field ID. Choose dropdown values only from the reported options. A
required checkbox may be set true only when user or repository evidence verifies the attestation; ask for an
unverifiable required fact.

Render locally:

```sh
uv run "<skill-dir>/scripts/issue-form.py" render \
  --form <form.json> --answers <answers.json> > <rendered.json>
```

The renderer rejects missing required values, invalid dropdowns, unknown IDs, and unverified required checkboxes. Use
its body and posting metadata exactly. The agent still owns answer wording, the external-disclosure review, title text
after the live prefix, semantic labels, and the external post.

For a selected Markdown template, fetch it live and populate its existing structure. If no template applies, use the
smallest useful `Problem`, `Solution`, and optional affected-files structure. Do not use `gh issue create --template`
with an automated body.

For checklist-style issues, mirror the user's stated structure literally: a single list unless the user requested
sections, preserving their stated ordering and casing. Never introduce unasked groupings such as Completed/Planned.

## Labels, Type, and Title

Use cached permission: `ADMIN`, `MAINTAIN`, `WRITE`, and `TRIAGE` may apply assignees and labels; `READ` may not. Apply
live template-defined assignees and labels when permitted. Add semantic labels only when the owner is the viewer or
`sablier-labs`, after matching against the live label set; never invent labels.

For YAML, prepend the rendered `posting.titlePrefix` and pass permitted `posting.assignees`, labels, and
`posting.issueType` when present. Preserve every applicable project entry; merge permitted live template labels with
agent-selected semantic labels and deduplicate. Write a concise title from the actual issue. For explicit issue
metadata, accept `--type`, `--parent`, `--blocked-by`, and `--blocking` using current `gh issue create` flags; validate
referenced issue numbers or URLs before posting.

An explicitly requested project title may use `gh issue create --project`. Do not pass issue-form `projects` through
that flag. After the issue URL is verified, apply each form project entry with `posting.md > Project-Template Metadata`
and `gh project item-add`. A project-add failure is partial completion; never recreate the issue.

```bash
gh issue create --repo "<owner>/<repo>" --title "<title>" --body-file "<body-file>" \
  --assignee "<template-assignee>" --label "<label>" \
  --type "<type>" --parent "<parent-number-or-url>" \
  --blocked-by "<issue-number-or-url>[,...]" --blocking "<issue-number-or-url>[,...]"
```

Omit metadata flags whose values are absent.

## Images and Posting

If images were requested, complete `context.md > Image Uploads` before creating the issue.

Run `posting.md > External-disclosure Review` on the title, rendered body, labels, type, project identifiers, metadata,
and attachments. Then post with `gh issue create --repo`, `--title`, and `--body-file`, adding `--assignee`, `--label`,
`--type`, `--parent`, `--blocked-by`, and `--blocking` only when applicable. Post directly because creation was
requested. On failure, follow `posting.md > Error Handling and Idempotency` before any retry.

Finish with the verified URL and the `### 🚀 Issue created` receipt from `SKILL.md`.

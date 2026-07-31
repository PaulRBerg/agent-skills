# Issue Creation Workflow

Create a GitHub issue from repository evidence and the selected live template.

## Context and Selection

Parse an optional leading `owner/repo`; otherwise infer the current repository. Repo-specific workflows supply their
fixed target and never infer it. Fetch authenticated context once:

```sh
<skill-dir>/scripts/yeet-context.sh repo "<owner>/<repo>" --issue-templates
```

Cache viewer login, permission, default branch, and template entries. Parse `--check`, repeated `--image <path>`, and
`--image-release` as before. With `--check`, search similar open issues and show results without adding a confirmation
gate.

Select the best template from the user's intent. This is an agent decision. Prefer YAML when a suitable YAML and
Markdown template coexist. Exclude `config.yml`.

## YAML Issue Forms

Inspect the selected live form; do not mirror its schema in prose:

```sh
uv run "<skill-dir>/scripts/issue-form.py" inspect \
  --repo "<owner>/<repo>" --template "<name>.yml" > <form.json>
```

The form JSON reports title prefix, labels, issue type, field IDs, labels, descriptions, render modes, dropdown options,
multi-select behavior, required flags, and checkbox attestations. Compose answers in a separate JSON object keyed by
field ID. Choose dropdown values only from the reported options. A required checkbox may be set true only when user or
repository evidence verifies the attestation; ask for an unverifiable required fact.

Render locally:

```sh
uv run "<skill-dir>/scripts/issue-form.py" render \
  --form <form.json> --answers <answers.json> > <rendered.json>
```

The renderer rejects missing required values, invalid dropdowns, unknown IDs, and unverified required checkboxes. Use
its body and posting metadata exactly. The agent still owns answer wording, privacy review, title text after the live
prefix, semantic labels, and the external post.

For a selected Markdown template, fetch it live and populate its existing structure. If no template applies, use the
smallest useful `Problem`, `Solution`, and optional affected-files structure. Do not use `gh issue create --template`
with an automated body.

For checklist-style issues, mirror the user's stated structure literally: a single list unless the user requested
sections, preserving their stated ordering and casing. Never introduce unasked groupings such as Completed/Planned.

## Labels, Type, and Title

Use cached permission: `ADMIN`, `MAINTAIN`, `WRITE`, and `TRIAGE` may apply labels; `READ` may not. Apply live
template-defined labels when permitted. Add semantic labels only when the owner is the viewer or `sablier-labs`, after
matching against the live label set; never invent labels.

For YAML, prepend the rendered `posting.titlePrefix` and pass `posting.issueType` when present. Merge permitted live
template labels with agent-selected semantic labels and deduplicate. Write a concise title from the actual issue.

## Images and Posting

If images were requested, follow `context.md > Image Uploads`. Use `gh img` by default; use the release-asset fallback
only with explicit `--image-release`. Place returned Markdown in the live image/reproduction field when one exists,
otherwise append an `Images` section. Stop before issue creation on upload failure.

Privacy-review the title, rendered body, labels, type, and attachments. Then post with `gh issue create --repo`,
`--title`, and `--body-file`, adding `--label` and `--type` only when applicable. Post directly because creation was
requested. On failure, follow `posting.md` idempotency handling before any retry.

Finish with the verified URL and the `### 🚀 Issue created` receipt from `SKILL.md`.

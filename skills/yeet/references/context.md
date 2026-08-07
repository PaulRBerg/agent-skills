# Contribution Context

Load only the sections linked by the active workflow.

## Auth Validation

Do not run unconditional `gh auth status`. Treat the first required read-only `gh` command as auth validation. Resolve
the bundled helper relative to the skill directory, never the target repository:

```sh
<skill-dir>/scripts/yeet-context.sh repo "<owner>/<repo>" [--issue-templates] [--discussion-templates] [--discussion-categories]
<skill-dir>/scripts/yeet-context.sh issue "<owner>/<repo>" <number>
<skill-dir>/scripts/yeet-context.sh labels "<owner>/<repo>"
```

If it fails with an auth error, stop with: `Run gh auth login first`.

## Repository Context

Collect repository context once and reuse it: authenticated login, repository identity and permission, default branch,
and only the templates/categories required by the workflow.

## Fetch Repo Labels

Fetch labels only when labels may be applied: owner-managed repositories, requested label edits, or a selected template
that defines labels and the viewer has `ADMIN`, `MAINTAIN`, `WRITE`, or `TRIAGE` permission.

Treat the live `name` and `description` list as authoritative. Match intent, use the smallest set, respect template
labels, and never invent labels. Skip maintainer workflow labels such as `good first issue`, `needs triage`,
`duplicate`, or `stale` on creation. An empty list is a valid no-label result; a failed fetch is an error.

## Template Metadata and Issue Forms

Issue-form YAML may define labels and type, but `gh issue create --body-file` does not execute the form. For
deterministic posting, render the relevant fields into Markdown and pass supported metadata explicitly. Do not combine
`--template` with `--body` or `--body-file`.

## Platform String Normalization

Use `scripts/get-macos-version.sh` for macOS fields. Skip environment details for repositories owned by the
authenticated viewer or `sablier-labs` unless the user explicitly asks; preserve required upstream template enums.

## Image Uploads

This workflow applies to issue and discussion creation and updates. Parse repeated `--image <path>` arguments and the
optional `--image-release` flag. Resolve every path to a readable local file, preserve argument order, and run an
external-disclosure review on the files before uploading them.

GitHub has no public attachment upload API. Try these paths in order:

1. If `gh img` is installed, run `gh img --repo "<owner>/<repo>" <paths...>` and capture its Markdown output.
2. If `gh img` is unavailable or clearly fails before upload, use `gh attach <paths...> -R "<owner>/<repo>" --markdown`
   only when `gh extension list` identifies the command as `sudosubin/gh-attach`. Other `gh attach` extensions have
   incompatible interfaces; do not guess their flags or install an extension automatically.
3. Use a release asset only when `--image-release` explicitly authorizes that separate external mutation.

Advance to the next path only when the prior uploader is unavailable or clearly failed before uploading anything. A
nonzero exit with any asset output is an ambiguous partial upload: stop and report the uploaded paths and failure rather
than retrying and creating duplicates. Continue only after every requested image produced usable Markdown; otherwise
leave an existing artifact unchanged or stop before creating a new one. Do not create a placeholder artifact just to
upload an image.

Place the Markdown in the user-requested field or section when specified. Otherwise prefer a live template field for
images, reproduction material, or uploads; failing that, append to an existing `## Images` section or create that
section at the end. Preserve the rest of an existing body verbatim and keep images in input order.

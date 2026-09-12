# Contribution Context

Load only the sections linked by the active workflow.

## Auth Validation

Do not run unconditional `gh auth status`. Treat the first required read-only `gh` command as auth validation. Resolve
the bundled helper relative to the skill directory, never the target repository:

Resolve `<skill-dir>` to the absolute directory containing the owning `SKILL.md` before running these commands:

```sh
<skill-dir>/scripts/yeet-context.sh repo "<owner>/<repo>" [--issue-templates] [--discussion-templates] [--discussion-categories]
<skill-dir>/scripts/yeet-context.sh issue "<owner>/<repo>" <number>
<skill-dir>/scripts/yeet-context.sh labels "<owner>/<repo>"
```

If it fails with an auth error, stop with: `Run gh auth login first`.

## Repository Context

Collect repository context once and reuse it: authenticated login, repository identity and permission, default branch,
and only the templates/categories required by the workflow.

## Issue Metadata Permissions

Filter metadata for every issue create/edit command using cached `repository.viewerPermission` before fetching labels,
issue types, or other metadata IDs. Apply these defaults unless existing action-specific capability evidence says
otherwise:

| Metadata mutation                                  | Required repository permission            |
| -------------------------------------------------- | ----------------------------------------- |
| Labels, assignees, parent/sub-issues, dependencies | `TRIAGE`, `WRITE`, `MAINTAIN`, or `ADMIN` |
| Issue type (`--type`, `--remove-type`), milestone  | `WRITE`, `MAINTAIN`, or `ADMIN`           |

The type/milestone default follows GitHub's [issue API permissions](https://docs.github.com/en/rest/issues/issues). For
relationships across repositories, check the required permission in each affected repository.

With `READ`, missing, or unknown permission, omit privileged metadata flags. Issue authorship, a live template's
`type: Bug`, and permission to edit the title/body do not grant metadata permission. Do not use a write as a permission
probe or fetch metadata that will be omitted. Continue permitted content work and briefly report skipped metadata;
explicitly requested metadata that cannot be applied remains incomplete.

Project access is independent of repository permission. Apply template-only project entries only when project write
access is already known; for an explicitly requested project, resolve its access only if needed. Report skipped entries
without blocking issue creation. A permission denial invalidates the corresponding cached capability; do not retry the
denied mutation without new authorization evidence.

## Fetch Repo Labels

Fetch labels only after the permission check allows them and the workflow needs template labels, requested label edits,
or semantic labels in an owner-managed repository.

Treat the live `name` and `description` list as authoritative. Match intent, use the smallest set, respect template
labels, and never invent labels. Skip maintainer workflow labels such as `good first issue`, `needs triage`,
`duplicate`, or `stale` on creation. An empty list is a valid no-label result; a failed fetch is an error.

## Template Metadata and Issue Forms

Issue-form YAML may define assignees, labels, type, and projects, but `gh issue create --body-file` does not execute the
form. Render the relevant fields into Markdown and pass only metadata allowed by `Issue Metadata Permissions`. Apply
permitted project metadata only after creation with `gh project item-add`; a failed project add leaves a created issue
and must not trigger issue recreation. Do not combine `--template` with `--body` or `--body-file`.

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

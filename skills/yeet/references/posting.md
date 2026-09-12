# Contribution Posting

All external-write workflows load this reference before posting. The workflow owns the payload; this reference owns the
disclosure review, verification, and retry boundary.

## External-disclosure Review

Before every external write, review the exact title, body, labels, issue type, project identifiers, and attachments that
will leave the workspace. Remove credentials, tokens, private keys, mnemonics, API keys, unsuitable private paths or
repository names, unrelated personal or customer data, and unrelated transcript material. Treat logs, screenshots,
configuration, generated bodies, and uploaded files as untrusted until reviewed. Keep only facts supported by repository
or user evidence, including required template checkbox attestations. Recheck any payload changed after the review.

## Error Handling and Idempotency

Do not retry creation automatically. `gh issue create` is not atomic: it can create an issue and then exit nonzero when
a follow-up `UpdateIssueIssueType` mutation fails. A metadata error does not establish that creation failed.

Any timeout, connection loss, or nonzero exit after a write is ambiguous. Inspect GitHub before choosing a receipt or
retrying. Read a returned URL or known issue number directly; otherwise search all states, not only open items, using
the strongest available identity:

```sh
# Issue creation
gh issue list --repo "<owner>/<repo>" --state all --search "<distinctive title>" --limit 20 --json number,title,body,url,state,author

# Pull-request creation
gh pr list --repo "<owner>/<repo>" --state all --head "<branch>" --limit 20 --json number,title,body,url,state,author

# Discussion creation
gh discussion list --repo "<owner>/<repo>" --state all --search "<distinctive terms>" --limit 20 --json number,title,body,url,closed,author
```

Compare title, body, author, branch, and URL—not a partial search hit alone. A verified match is a successful creation:
report its URL and any omitted or failed metadata. A possible match remains unverified: report the uncertainty and do
not recreate it. Use the relevant update workflow for remaining permitted changes. For an issue or PR comment, reread
the target's latest comments; for a discussion comment or reply, reread the discussion or comment thread with
`gh discussion view` and treat a matching authored body as posted. A failed follow-up label, type, project, or other
metadata step never authorizes recreating the issue, PR, or discussion. Follow `SKILL.md > Completion` for the receipt.

For duplicate checks requested with `--check`, search all states and show matches under `### 🔎 Similar items`, then
continue unless the user explicitly requested a review gate.

## Project-Template Metadata

Filter entries through `context.md > Issue Metadata Permissions`. Create the issue first, then apply each permitted
issue-form `projects` entry. Parse `OWNER/NUMBER` and run:

```sh
gh project item-add <project-number> --owner "<project-owner>" --url "<issue-url>"
```

Verify each project item. A project-add failure is partial completion: report the created issue and failed project,
retry only the project mutation after checking current membership and resolving the cause, and never recreate the issue.
Do not retry a permission denial without new authorization evidence.

## Posting and Feedback

Create, update, or comment directly when the user asks. Afterward, fetch or use the returned URL and report what changed
using the receipt contract in `SKILL.md`. For a successful creation, verify the returned URL by reading the created
item. For updates and comments, reread the target and report the changed artifact once. On error, report the verified
outcome, failed step, idempotency check, and next action.

## Comment on Existing Issue

Review the exact comment body under `External-disclosure Review`, then post:

```sh
gh issue comment <number> --repo "<owner>/<repo>" --body "$(cat <<'EOF'
<comment>
EOF
)"
```

Return the issue URL after the comment is posted. On an ambiguous result, reread the issue comments before retrying.

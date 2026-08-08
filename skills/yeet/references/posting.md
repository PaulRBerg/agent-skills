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

On failure, lead with `### ⛔ <artifact> not <action>`, then report the attempted target, concrete error, idempotency
result, and user's next action. Do not retry automatically except for one label-permission failure after checking
whether the artifact already exists.

Any timeout, connection loss, or nonzero exit after a write is ambiguous. Stop and inspect GitHub before retrying.
Search all states, not only open items, using the strongest available identity:

```sh
# Issue creation
gh issue list --repo "<owner>/<repo>" --state all --search "<distinctive title>" --limit 20 --json number,title,body,url,state,author

# Pull-request creation
gh pr list --repo "<owner>/<repo>" --state all --head "<branch>" --limit 20 --json number,title,body,url,state,author

# Discussion creation
gh discussion list --repo "<owner>/<repo>" --state all --search "<distinctive terms>" --limit 20 --json number,title,body,url,closed,author
```

Compare title, body, author, branch, and URL—not a partial search hit alone. A possible match is a partial success:
stop, report it, and switch to the relevant update workflow. Never recreate it. For an issue or PR comment, reread the
target's latest comments; for a discussion comment or reply, reread the discussion or comment thread with
`gh discussion view` and treat a matching authored body as posted. A failed follow-up label, project, or other metadata
step never authorizes recreating the issue, PR, or discussion.

For duplicate checks requested with `--check`, search all states and show matches under `### 🔎 Similar items`, then
continue unless the user explicitly requested a review gate.

## Project-Template Metadata

Create the issue first, then apply each issue-form `projects` entry. Parse `OWNER/NUMBER` and run:

```sh
gh project item-add <project-number> --owner "<project-owner>" --url "<issue-url>"
```

Verify each project item. A project-add failure is partial completion: report the created issue and failed project,
retry only the project mutation after checking current membership, and never recreate the issue.

## Posting and Feedback

Create, update, or comment directly when the user asks. Afterward, fetch or use the returned URL and report what changed
using the receipt contract in `SKILL.md`. For a successful creation, verify the returned URL by reading the created
item. For updates and comments, reread the target and report the changed artifact once. On failure, report the
idempotency check and next action without claiming a write succeeded.

## Comment on Existing Issue

Review the exact comment body under `External-disclosure Review`, then post:

```sh
gh issue comment <number> --repo "<owner>/<repo>" --body "$(cat <<'EOF'
<comment>
EOF
)"
```

Return the issue URL after the comment is posted. On an ambiguous result, reread the issue comments before retrying.

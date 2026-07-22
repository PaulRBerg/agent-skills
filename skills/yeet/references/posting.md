# Contribution Posting

## Error Handling and Idempotency

On failure, lead with `### ⛔ <artifact> not <action>`, then report the attempted target, concrete error, idempotency
result, and user's next action. Do not retry automatically except for one label-permission failure after checking
whether the artifact already exists.

Before retrying a failed creation, search for the likely artifact:

```sh
gh issue list --repo "<owner>/<repo>" --author '@me' --limit 5
gh pr list --repo "<owner>/<repo>" --head "<branch>"
```

If it exists, switch to the update/comment workflow. Never create a duplicate because a follow-up label or network step
failed.

## Posting and Feedback

Create, update, or comment directly when the user asks. Afterward, fetch or use the returned URL and report what changed
using the receipt contract in `SKILL.md`. For duplicate searches requested with `--check`, surface matches under
`### 🔎 Similar open items`, say explicitly that creation is continuing, and continue unless the user asked for a review
gate.

## Comment on Existing Issue

```sh
gh issue comment <number> --repo "<owner>/<repo>" --body "$(cat <<'EOF'
<comment>
EOF
)"
```

Return the issue URL after the comment is posted.

# Contribution Posting

## Error Handling and Idempotency

On failure, report the attempted operation, the concrete error, and the user's next action. Do not retry automatically except for one label-permission failure after checking whether the artifact already exists.

Before retrying a failed creation, search for the likely artifact:

```sh
gh issue list --repo "<owner>/<repo>" --author '@me' --limit 5
gh pr list --repo "<owner>/<repo>" --head "<branch>"
```

If it exists, switch to the update/comment workflow. Never create a duplicate because a follow-up label or network step failed.

## Posting and Feedback

Create, update, or comment directly when the user asks. Afterward, fetch or use the returned URL and report what changed. For duplicate searches requested with `--check`, surface matches as a heads-up and continue unless the user asked for a review gate.

## Comment on Existing Issue

```sh
gh issue comment <number> --repo "<owner>/<repo>" --body "$(cat <<'EOF'
<comment>
EOF
)"
```

Return the issue URL after the comment is posted.

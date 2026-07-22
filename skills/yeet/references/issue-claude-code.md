# Claude Code Issue Workflow

Create an issue in `anthropics/claude-code`. Every `gh` command must use that repository; never infer it from the
working directory.

## Live Form Workflow

1. Fetch authenticated repository/template context with
   `scripts/yeet-context.sh repo anthropics/claude-code --issue-templates`.
2. Select the live YAML form matching the user's intent: bug, feature request, documentation, or model behavior. Ask
   only when that semantic choice is genuinely ambiguous.
3. Run `scripts/issue-form.py inspect --repo anthropics/claude-code --template <selected.yml>` and compose answers keyed
   by the returned field IDs. Do not compare SHAs or consult a static template mirror.
4. Gather only environment facts requested by the live form. `claude --version` supplies the Claude Code version.
   Normalize OS and terminal values to exact live dropdown options; place precise versions in a free-text field when
   useful. Infer Anthropic API, Bedrock, or Vertex only from environment/user evidence.
5. Mark a required checkbox true only when its attestation is verified. Render through `issue-form.py render`; resolve
   every reported missing/invalid answer before posting.
6. Privacy-review the agent-authored answers and rendered body. Compose a concise title after the live prefix. Use live
   template labels/type only when cached permission allows them.
7. Post with `gh issue create --repo anthropics/claude-code --title ... --body-file ...`, adding permitted labels and
   issue type from rendered metadata. On label permission failure, run the posting idempotency check, then retry once
   without labels.

The helper owns form parsing and exact body structure. The agent owns template selection, environment interpretation,
answer/title writing, privacy, and posting.

For a comment on an existing issue, use `posting.md > Comment on Existing Issue` with this fixed repository. Finish a
successful creation with the verified URL and the `### 🚀 Issue created` receipt.

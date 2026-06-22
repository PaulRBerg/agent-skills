# Claude Code Issue Workflow

Create issues in the `anthropics/claude-code` repository with environment gathering and specialized templates.

## Repo Isolation

This workflow targets **`anthropics/claude-code`** exclusively. Every `gh` command must use `--repo "anthropics/claude-code"`. Do not infer from working directory.

File links: `[{path}](https://github.com/anthropics/claude-code/blob/main/{path})`

## Validate Authentication

See `commons.md > Auth Validation`. The repository context read below is the auth check.

## Template Drift Check

Before generating the issue body, verify the template specs in this file still match the upstream `.github/ISSUE_TEMPLATE` files. GitHub form templates change over time and silent drift produces issues with wrong section headers, missing fields, or invalid dropdown values.

```bash
scripts/yeet-context.sh repo anthropics/claude-code --issue-templates
```

Use `repository.viewerPermission` for label capability. Compare `repository.issueTemplateTree.entries` names and SHAs against the known-good SHAs (last verified 2026-06-17):

| File                  | SHA                                        |
| --------------------- | ------------------------------------------ |
| `bug_report.yml`      | `fce2b87e5dc2a42e0d3ff477ab86f528ec9dd290` |
| `config.yml`          | `5fe5625f9a612f3a8fba4fcd1ab4a34f266164d6` |
| `documentation.yml`   | `ead68fe39d0063179b5303ae3954ec51f9683690` |
| `feature_request.yml` | `2fa6ca2409e973a1d002620c692cc0d282e58569` |
| `model_behavior.yml`  | `9c89de4b6522f68ddb4421767505506e316cb31c` |

**If any SHA differs** (or a new file appears), the upstream templates have changed. Before creating the issue:

1. Fetch the changed template(s):

   ```bash
   gh api repos/anthropics/claude-code/contents/.github/ISSUE_TEMPLATE/{file}.yml --jq '.content' | base64 -d
   ```

2. Diff against this routing file and the matching `references/templates/claude-code/` template reference (field labels, dropdown options, required fields).

3. Tell the user:

   > ⚠️ Claude Code's issue templates have drifted from the spec in this skill (e.g., `bug_report.yml` SHA `<old>` → `<new>`). Please update the `yeet` skill in [`PaulRBerg/agent-skills`](https://github.com/PaulRBerg/agent-skills) — specifically `skills/yeet/references/issue-claude-code.md` and the matching file under `skills/yeet/references/templates/claude-code/` — and refresh the SHA table. Continue filing this issue using the closest matching fields, but flag any new required fields you couldn't fill.

4. Proceed with best-effort body generation using the fields the user can still fill from the new template.

## Determine Issue Type

From the issue description, infer which template fits best:

| Keywords                                                                               | Template              | Title Prefix | Label           |
| -------------------------------------------------------------------------------------- | --------------------- | ------------ | --------------- |
| bug, broken, error, crash, fails, doesn't work, EACCES                                 | `bug_report.yml`      | `[BUG] `     | `bug`           |
| feature, request, add, support, wish, would be nice                                    | `feature_request.yml` | `[FEATURE] ` | `enhancement`   |
| docs, documentation, unclear, confusing, readme, broken link, typo                     | `documentation.yml`   | `[DOCS] `    | `documentation` |
| model, claude did, unexpected, wrong files, reverted, ignored, modified without asking | `model_behavior.yml`  | `[MODEL] `   | `model`         |

**If ambiguous**: Use AskUserQuestion with options: Bug Report, Feature Request, Documentation, Model Behavior.

> [!IMPORTANT]
> `gh issue create --body` bypasses GitHub's form template, so it does NOT auto-apply labels. Pass `--label` only when cached `repository.viewerPermission` returns `TRIAGE`, `WRITE`, `MAINTAIN`, or `ADMIN`; otherwise omit labels and let maintainers triage.

## Generate Issue Body

Section headers must match the template field labels exactly — this keeps issues consistent with form-submitted issues. Dropdown answers must be picked from the predefined options in the selected template reference; do not invent new values.

After selecting the issue type, load exactly one template reference:

| Template file         | Reference                                             |
| --------------------- | ----------------------------------------------------- |
| `bug_report.yml`      | `references/templates/claude-code/bug-report.md`      |
| `feature_request.yml` | `references/templates/claude-code/feature-request.md` |
| `documentation.yml`   | `references/templates/claude-code/documentation.md`   |
| `model_behavior.yml`  | `references/templates/claude-code/model-behavior.md`  |

## Generate Title

Concise (5-10 words) with prefix matching the template: `[BUG]`, `[FEATURE]`, `[DOCS]`, or `[MODEL]`.

## Create the Issue

```bash
gh issue create \
  --repo "anthropics/claude-code" \
  --title "$title" \
  --body "$(cat <<'EOF'
$body
EOF
)"
```

Add `--label "$label"` only when cached `repository.viewerPermission` allows labels. When used, `$label` must be one of `bug`, `enhancement`, `documentation`, `model` (matches the template's auto-applied label — see the routing table above). On a label permission error, follow `commons.md > Idempotency on Retry`, then retry once without `--label`.

Display: "Created: $URL"

## Comment on Existing Issue

See `commons.md > Comment on Existing Issue`, using repo `"anthropics/claude-code"`.

## Environment Detection

- **Claude Code Version**: `claude --version 2>/dev/null || echo "unknown"` — paste full output.
- **Operating System**: pick from the OS dropdown (`macOS`, `Windows`, `Ubuntu/Debian Linux`, `Other Linux`, `Other`). Use `uname -s` / `scripts/get-macos-version.sh` only to decide which option fits; the precise version (e.g., `macOS Tahoe v26.2`) belongs in **Additional Information**, not the OS field.
- **Terminal/Shell**: pick from the Terminal dropdown. On macOS, use `$TERM_PROGRAM`, `$TERMINAL_EMULATOR`, and `$INSIDE_EMACS` to map:
  - `Apple_Terminal` → `Terminal.app (macOS)`
  - `iTerm.app` → `iTerm2`
  - `WarpTerminal` → `Warp`
  - `cursor` / `Cursor` → `Cursor`
  - `vscode` → `VS Code integrated terminal`
  - `JetBrains-JediTerm` → `IntelliJ IDEA terminal` or `PyCharm terminal` (depends on host IDE)
  - CI envs (`CI=true`, `GITHUB_ACTIONS`, etc.) → `Non-interactive/CI environment`
  - Anything else → `Other`
- **Platform**: default to `Anthropic API` unless `ANTHROPIC_BEDROCK_BASE_URL`, `CLAUDE_CODE_USE_BEDROCK`, `CLAUDE_CODE_USE_VERTEX`, or user-supplied context indicates otherwise.

## Examples

```bash
# Bug report
"Claude crashes when I use special characters in file paths"

# Feature request
"Add support for .claude.toml config files"

# Docs issue
"The MCP server docs don't explain how to configure multiple servers"

# Model behavior
"Claude reverted my changes without asking when I said 'undo'"
```

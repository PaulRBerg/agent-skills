# Claude Code Issue Workflow

Create issues in the `anthropics/claude-code` repository with environment gathering and specialized templates.

## Repo Isolation

This workflow targets **`anthropics/claude-code`** exclusively. Every `gh` command must use `--repo "anthropics/claude-code"`. Do not infer from working directory.

File links: `[{path}](https://github.com/anthropics/claude-code/blob/main/{path})`

## Validate Authentication

See `commons.md > Auth Validation`.

## Template Drift Check

Before generating the issue body, verify the template specs in this file still match the upstream `.github/ISSUE_TEMPLATE` files. GitHub form templates change over time and silent drift produces issues with wrong section headers, missing fields, or invalid dropdown values.

```bash
gh api repos/anthropics/claude-code/contents/.github/ISSUE_TEMPLATE \
  --jq '.[] | select(.name | endswith(".yml")) | "\(.name) \(.sha)"'
```

Compare against the known-good SHAs (last verified 2026-05-07):

| File | SHA |
|---|---|
| `bug_report.yml` | `fce2b87e5dc2a42e0d3ff477ab86f528ec9dd290` |
| `config.yml` | `5fe5625f9a612f3a8fba4fcd1ab4a34f266164d6` |
| `documentation.yml` | `ead68fe39d0063179b5303ae3954ec51f9683690` |
| `feature_request.yml` | `2fa6ca2409e973a1d002620c692cc0d282e58569` |
| `model_behavior.yml` | `9c89de4b6522f68ddb4421767505506e316cb31c` |

**If any SHA differs** (or a new file appears), the upstream templates have changed. Before creating the issue:

1. Fetch the changed template(s):
   ```bash
   gh api repos/anthropics/claude-code/contents/.github/ISSUE_TEMPLATE/{file}.yml --jq '.content' | base64 -d
   ```
2. Diff against the spec in this file (field labels, dropdown options, required fields).
3. Tell the user:

   > ⚠️ Claude Code's issue templates have drifted from the spec in this skill (e.g., `bug_report.yml` SHA `<old>` → `<new>`). Please update the `yeet` skill in [`PaulRBerg/agent-skills`](https://github.com/PaulRBerg/agent-skills) — specifically `skills/yeet/references/issue-claude-code.md` — and refresh the SHA table. Continue filing this issue using the closest matching fields, but flag any new required fields you couldn't fill.

4. Proceed with best-effort body generation using the fields the user can still fill from the new template.

## Determine Issue Type

From the issue description, infer which template fits best:

| Keywords | Template | Title Prefix | Label |
|---|---|---|---|
| bug, broken, error, crash, fails, doesn't work, EACCES | `bug_report.yml` | `[BUG] ` | `bug` |
| feature, request, add, support, wish, would be nice | `feature_request.yml` | `[FEATURE] ` | `enhancement` |
| docs, documentation, unclear, confusing, readme, broken link, typo | `documentation.yml` | `[DOCS] ` | `documentation` |
| model, claude did, unexpected, wrong files, reverted, ignored, modified without asking | `model_behavior.yml` | `[MODEL] ` | `model` |

**If ambiguous**: Use AskUserQuestion with options: Bug Report, Feature Request, Documentation, Model Behavior.

> [!IMPORTANT]
> `gh issue create --body` bypasses GitHub's form template, so it does NOT auto-apply labels. Pass `--label` explicitly (see "Create the Issue").

## Generate Issue Body

Section headers must match the template field labels exactly — this keeps issues consistent with form-submitted issues. Dropdown answers must be picked from the predefined options listed below; do not invent new values.

### Bug Report Template

```markdown
### Preflight Checklist

- [x] I have searched existing issues and this hasn't been reported yet
- [x] This is a single bug report (please file separate reports for different bugs)
- [x] I am using the latest version of Claude Code

### What's Wrong?

{what's happening that shouldn't be}

### What Should Happen?

{expected behavior}

### Error Messages/Logs

{paste error output, stack traces, or logs in a fenced ```shell block — omit section if none}

### Steps to Reproduce

1. {step 1}
2. {step 2}
3. ...

### Claude Model

{one of: Sonnet (default) | Opus | Not sure / Multiple models | Other}

### Is this a regression?

{one of: Yes, this worked in a previous version | No, this never worked | I don't know}

### Last Working Version

{e.g., 1.0.100 — only if regression, otherwise write "N/A"}

### Claude Code Version

{output of `claude --version`, e.g., "1.0.123 (Claude Code)"}

### Platform

{one of: Anthropic API | AWS Bedrock | Google Vertex AI | Other}

### Operating System

{one of: macOS | Windows | Ubuntu/Debian Linux | Other Linux | Other}

### Terminal/Shell

{one of: Terminal.app (macOS) | Warp | Cursor | iTerm2 | IntelliJ IDEA terminal | VS Code integrated terminal | PyCharm terminal | Windows Terminal | PowerShell | WSL (Windows Subsystem for Linux) | Xterm | Non-interactive/CI environment | Other}

### Additional Information

{screenshots, config files, repro repo links, exact OS version (e.g., "macOS Tahoe v26.2") — omit section if none}
```

### Feature Request Template

```markdown
### Preflight Checklist

- [x] I have searched existing requests and this feature hasn't been requested yet
- [x] This is a single feature request (not multiple features)

### Problem Statement

{what problem are you trying to solve? focus on the problem, not the solution}

### Proposed Solution

{how would you like this to work? describe the ideal user experience}

### Alternative Solutions

{alternatives or workarounds you're using — or "None"}

### Priority

{one of: Critical - Blocking my work | High - Significant impact on productivity | Medium - Would be very helpful | Low - Nice to have}

### Feature Category

{one of: CLI commands and flags | Interactive mode (TUI) | File operations | API and model interactions | MCP server integration | Performance and speed | Configuration and settings | Developer tools/SDK | Documentation | Other}

### Use Case Example

{concrete real-world scenario, step-by-step — omit section if not applicable}

### Additional Context

{screenshots, mockups, similar features in other tools — omit section if none}
```

### Documentation Template

```markdown
### Documentation Type

{one of: Missing documentation (feature not documented) | Unclear/confusing documentation | Incorrect/outdated documentation | Typo or formatting issue | Missing code examples | Broken links | Other}

### Documentation Location

{URL where you encountered this issue, e.g., https://docs.anthropic.com/en/docs/claude-code/... — omit section if unknown}

### Section/Topic

{specific section or topic needing improvement}

### Current Documentation

{quote the current text — omit section if not applicable}

### What's Wrong or Missing?

{explain what's incorrect, unclear, or missing}

### Suggested Improvement

{how it should be improved, with suggested text if possible}

### Impact

{one of: High - Prevents users from using a feature | Medium - Makes feature difficult to understand | Low - Minor confusion or inconvenience}

### Additional Context

{screenshots, related docs, examples — omit section if none}
```

### Model Behavior Template

```markdown
### Preflight Checklist

- [x] I have searched existing issues for similar behavior reports
- [x] This report does NOT contain sensitive information (API keys, passwords, etc.)

### Type of Behavior Issue

{one of: Claude modified files I didn't ask it to modify | Claude accessed files outside the working directory | Claude ignored my instructions or configuration | Claude reverted/undid previous changes without asking | Claude made incorrect assumptions about my project | Claude refused a reasonable request | Claude's behavior changed between sessions | Subagent behaved unexpectedly | Other unexpected behavior}

### What You Asked Claude to Do

{the exact prompt or command}

### What Claude Actually Did

{step-by-step what happened}

### Expected Behavior

{what should Claude have done}

### Files Affected

{paste in a fenced ```shell block: files modified, files read unexpectedly — omit section if not applicable}

### Permission Mode

{one of: Accept Edits was ON (auto-accepting changes) | Accept Edits was OFF (manual approval required) | I don't know / Not sure}

### Can You Reproduce This?

{one of: Yes, every time with the same prompt | Sometimes (intermittent) | No, only happened once | Haven't tried to reproduce}

### Steps to Reproduce

{minimal steps — omit section if not reproducible}

### Claude Model

{one of: Sonnet | Opus | Haiku | Not sure | Other}

### Relevant Conversation

{paste relevant Claude responses in a fenced ```markdown block — omit section if none}

### Impact

{one of: Critical - Data loss or corrupted project | High - Significant unwanted changes | Medium - Extra work to undo changes | Low - Minor inconvenience}

### Claude Code Version

{output of `claude --version`, e.g., "1.0.123 (Claude Code)"}

### Platform

{one of: Anthropic API | AWS Bedrock | Google Vertex AI | Other}

### Additional Context

{patterns noticed, similar behavior, file types that trigger it — omit section if none}
```

## Generate Title

Concise (5-10 words) with prefix matching the template: `[BUG]`, `[FEATURE]`, `[DOCS]`, or `[MODEL]`.

## Create the Issue

```bash
gh issue create \
  --repo "anthropics/claude-code" \
  --title "$title" \
  --label "$label" \
  --body "$(cat <<'EOF'
$body
EOF
)"
```

`$label` must be one of `bug`, `enhancement`, `documentation`, `model` (matches the template's auto-applied label — see the routing table above).

Display: "Created: $URL"

## Comment on Existing Issue

See `commons.md > Comment on Existing Issue`, using repo `"anthropics/claude-code"`.

## Environment Detection

- **Claude Code Version**: `claude --version 2>/dev/null || echo "unknown"` — paste full output.
- **Operating System**: pick from the OS dropdown (`macOS`, `Windows`, `Ubuntu/Debian Linux`, `Other Linux`, `Other`). Use `uname -s` / `scripts/get-macos-version.sh` only to decide which option fits; the precise version (e.g., `macOS Tahoe v26.2`) belongs in **Additional Information**, not the OS field.
- **Terminal/Shell**: pick from the Terminal dropdown. Use `$TERM_PROGRAM`, `$TERMINAL_EMULATOR`, and `$INSIDE_EMACS` to map:
  - `Apple_Terminal` → `Terminal.app (macOS)`
  - `iTerm.app` → `iTerm2`
  - `WarpTerminal` → `Warp`
  - `cursor` / `Cursor` → `Cursor`
  - `vscode` → `VS Code integrated terminal`
  - `JetBrains-JediTerm` → `IntelliJ IDEA terminal` or `PyCharm terminal` (depends on host IDE)
  - PowerShell host (`PSModulePath` set, no `WSL_DISTRO_NAME`) → `PowerShell` or `Windows Terminal`
  - `WSL_DISTRO_NAME` set → `WSL (Windows Subsystem for Linux)`
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

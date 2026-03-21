# Claude Code Issue Workflow

Create issues in the `anthropics/claude-code` repository with environment gathering and specialized templates.

## Repo Isolation

This workflow targets **`anthropics/claude-code`** exclusively. Every `gh` command must use `--repo "anthropics/claude-code"`. Do not infer from working directory.

File links: `[{path}](https://github.com/anthropics/claude-code/blob/main/{path})`

## Validate Authentication

See `commons.md > Auth Validation`.

## Determine Issue Type

From the issue description, infer which template fits best:

| Keywords | Template | Title Prefix |
|---|---|---|
| bug, broken, error, crash, fails, doesn't work | `bug_report.yml` | `[BUG] ` |
| feature, request, add, support, wish, would be nice | `feature_request.yml` | `[FEATURE] ` |
| docs, documentation, unclear, confusing, readme | `documentation.yml` | `[DOCS] ` |
| model, claude did, unexpected, wrong files, reverted, ignored | `model_behavior.yml` | `[MODEL] ` |

**If ambiguous**: Use AskUserQuestion with options: Bug Report, Feature Request, Documentation, Model Behavior.

## Generate Issue Body

### Bug Report Template

```markdown
### What went wrong?

{describe the bug}

### What should happen?

{expected behavior}

### Steps to reproduce

1. {step 1}
2. {step 2}
3. ...

### Environment

- **Version**: {claude --version}
- **OS**: {see commons.md > Platform String Normalization}
- **Terminal**: {$TERM_PROGRAM}
- **Platform**: Anthropic API (assume unless stated otherwise)
- **Model**: Sonnet (assume unless stated otherwise)

### Is this a regression?

{Yes/No/Don't know - infer from context}
```

### Feature Request Template

```markdown
### Problem

{what problem does this solve?}

### Proposed solution

{how should it work?}

### Alternatives considered

{any workarounds or other approaches - or "None"}

### Priority

{Critical/High/Medium/Low - default Medium}

### Category

{infer: CLI commands and flags, Interactive mode (TUI), File operations, API and model interactions, MCP server integration, Performance and speed, Configuration and settings, Developer tools/SDK, Documentation, Other}
```

### Documentation Template

```markdown
### What type of docs issue?

{Missing/Unclear/Incorrect/Typo/Missing examples/Broken links/Other}

### Section or topic

{which part of the docs}

### What's wrong or missing?

{describe the issue}

### Suggested fix

{how to improve it}

### Impact

{High/Medium/Low}
```

### Model Behavior Template

```markdown
### What'd you ask Claude to do?

{the prompt or command}

### What did Claude actually do?

{step by step what happened}

### What should've happened?

{expected behavior}

### Files affected

{list files modified unexpectedly, if applicable}

### Environment

- **Version**: {claude --version}
- **OS**: {see commons.md > Platform String Normalization}
- **Model**: {Sonnet/Opus/Haiku - default Sonnet}
- **Platform**: Anthropic API
- **Permission mode**: {Accept Edits ON/OFF - infer or "unknown"}

### Can you reproduce this?

{Yes/Sometimes/No/Haven't tried}

### Impact

{Critical/High/Medium/Low}
```

## Generate Title

Concise (5-10 words) with prefix: `[BUG]`, `[FEATURE]`, `[DOCS]`, or `[MODEL]`.

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

Template labels (`bug`, `enhancement`, `documentation`, `model`) are applied automatically by GitHub.

Display: "Created: $URL"

## Comment on Existing Issue

See `commons.md > Comment on Existing Issue`, using repo `"anthropics/claude-code"`.

## Environment Detection

- **Version**: `claude --version 2>/dev/null || echo "unknown"`
- **OS**: See `commons.md > Platform String Normalization`
- **Terminal**: `${TERM_PROGRAM:-${TERMINAL_EMULATOR:-unknown}}`

## Examples

```bash
# Bug report
"Claude crashes when I use special characters in file paths"

# Feature request
"Add support for .claud.toml config files"

# Docs issue
"The MCP server docs don't explain how to configure multiple servers"

# Model behavior
"Claude reverted my changes without asking when I said 'undo'"
```

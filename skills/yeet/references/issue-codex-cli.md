# Codex CLI Issue Workflow

Create issues in the `openai/codex` repository with environment gathering and specialized templates.

## Repo Isolation

This workflow targets **`openai/codex`** exclusively. Every `gh` command must use `--repo "openai/codex"`. Do not infer from working directory.

File links: `[{path}](https://github.com/openai/codex/blob/main/{path})`

## Validate Authentication

See `commons.md > Auth Validation`.

## Determine Issue Type

From the issue description, infer which template fits best:

| Keywords | Template | Title Prefix |
|---|---|---|
| bug, broken, error, crash, fails, doesn't work | `2-bug-report.yml` | `[BUG] ` |
| feature, request, add, support, wish, would be nice | `4-feature-request.yml` | `[FEATURE] ` |
| docs, documentation, unclear, confusing, readme, example | `3-docs-issue.yml` | `[DOCS] ` |
| vscode, extension, cursor, windsurf, ide | `5-vs-code-extension.yml` | `[EXTENSION] ` |

**If ambiguous**: Use AskUserQuestion with options: Bug Report, Feature Request, Documentation, VS Code Extension.

## Generate Issue Body

### Bug Report Template

```markdown
### What version of Codex is running?

{codex --version}

### What subscription do you have?

{Plus/Pro/Team/Enterprise - infer or ask}

### Which model were you using?

{gpt-4.1/o4-mini/o3/etc. - infer or "Not specified"}

### What platform is your computer?

{see commons.md > Platform String Normalization}

### What issue are you seeing?

{describe the bug}

### What steps can reproduce the bug?

1. {step 1}
2. {step 2}
3. ...

### What is the expected behavior?

{expected behavior}

### Additional information

{any other relevant context}
```

### Feature Request Template

```markdown
### What feature would you like to see?

{describe the feature and the problem it solves}

### Additional information

{any other relevant context, workarounds, or alternatives - or "None"}
```

### Documentation Template

```markdown
### What is the type of issue?

{Documentation is missing/incorrect/confusing/Example code is not working/Something else}

### What is the issue?

{describe the documentation problem}

### Where did you find it?

{URL or location in docs - if known}
```

### VS Code Extension Template

```markdown
### What version of the VS Code extension are you using?

{extension version}

### What subscription do you have?

{Plus/Pro/Team/Enterprise - infer or ask}

### Which IDE are you using?

{VS Code/Cursor/Windsurf/etc.}

### What platform is your computer?

{see commons.md > Platform String Normalization}

### What issue are you seeing?

{describe the issue}

### What steps can reproduce the bug?

1. {step 1}
2. {step 2}
3. ...

### What is the expected behavior?

{expected behavior}

### Additional information

{any other relevant context}
```

## Generate Title

Concise (5-10 words) with prefix: `[BUG]`, `[FEATURE]`, `[DOCS]`, or `[EXTENSION]`.

## Create the Issue

```bash
gh issue create \
  --repo "openai/codex" \
  --title "$title" \
  --body "$(cat <<'EOF'
$body
EOF
)"
```

Template labels are applied automatically by GitHub.

Display: "Created: $URL"

## Comment on Existing Issue

See `commons.md > Comment on Existing Issue`, using repo `"openai/codex"`.

## Environment Detection

- **Codex CLI version**: `codex --version 2>/dev/null || echo "unknown"`
- **Platform**: See `commons.md > Platform String Normalization`
- **IDE** (for extension issues): Ask user or infer from context

## Examples

```bash
# Bug report
"Codex crashes when processing large files"

# Feature request
"Add support for custom system prompts"

# Docs issue
"The installation docs don't mention npm prerequisites"

# Extension issue
"The Cursor extension doesn't connect to the API"
```

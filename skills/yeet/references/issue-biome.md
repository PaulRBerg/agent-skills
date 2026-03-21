# Biome Issue Workflow

Create issues in the `biomejs/biome` repository with playground reproduction links and specialized templates.

## Repo Isolation

This workflow targets **`biomejs/biome`** exclusively. Every `gh` command must use `--repo "biomejs/biome"`. Do not infer from working directory.

File links: `[{path}](https://github.com/biomejs/biome/blob/main/{path})`

## Validate Authentication

See `commons.md > Auth Validation`.

## Determine Issue Type

From the issue description, infer which template fits best:

| Keywords | Template | Title Prefix |
|---|---|---|
| format, formatter, formatting, prettier | `01_formatter_bug` | `📝 ` |
| lint, linter, rule, diagnostic, warning | `02_lint_bug` | `💅 ` |
| bug, broken, error, crash, panic, fails | `03_bug` | `🐛 ` |
| task, implement, add support (contributors) | `04_task` | `📎 ` |

**If ambiguous**: Use AskUserQuestion with options: Formatter Bug, Linter Bug, General Bug, Task.

## Create Playground Link

Bug reports require a playground reproduction at https://biomejs.dev/playground/:

1. Paste minimal reproduction code
2. Select language (JavaScript, TypeScript, JSX, TSX, JSON, CSS, GraphQL)
3. Configure relevant settings (formatter: line width, indent style, quote style; linter: specific rules)
4. Copy URL (auto-updates with query params: `code=<base64>`, `language=<type>`, settings)

For multi-file scenarios: `npm create @biomejs/biome-reproduction`

## Generate Issue Body

### Formatter Bug Template

```markdown
### Environment information

{biome rage --formatter output}

### Configuration

{biome.json contents if relevant, or "Default configuration"}

### Playground link

{playground URL}

### Expected result

{what the formatter should output}
```

### Linter Bug Template

```markdown
### Environment information

{biome rage --linter output}

### Rule name

{e.g., "noUnusedVariables" or "suspicious/noExplicitAny"}

### Playground link

{playground URL}

### Expected result

{should it error? not error? different message?}
```

### General Bug Template

```markdown
### Environment information

{biome rage output}

### What happened?

{describe the bug}

1. {step 1}
2. {step 2}
3. ...

{playground link if applicable, or reproduction repo}

### Expected result

{what should happen}
```

### Task Template

```markdown
### Description

{summary of the task}
```

## Generate Title

Concise (5-10 words) with emoji prefix: `📝`, `💅`, `🐛`, or `📎`.

## Create the Issue

```bash
gh issue create \
  --repo "biomejs/biome" \
  --title "$title" \
  --body "$(cat <<'EOF'
$body
EOF
)"
```

The label `S-Needs triage` is applied automatically by GitHub for bug templates.

Display: "Created: $URL"

## Comment on Existing Issue

See `commons.md > Comment on Existing Issue`, using repo `"biomejs/biome"`.

## Environment Detection

Use the appropriate `biome rage` variant:

```bash
biome rage --formatter  # Formatter bugs
biome rage --linter     # Linter bugs
biome rage              # General bugs
```

## Examples

```bash
# Formatter bug
"Biome formatter adds trailing comma in single-line arrays unlike Prettier"

# Linter bug
"noUnusedVariables false positive when variable is used in template literal"

# General bug
"biome check crashes with panic on valid TypeScript file"

# CLI issue
"biome migrate command doesn't handle nested extends in config"
```

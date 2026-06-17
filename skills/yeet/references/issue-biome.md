# Biome Issue Workflow

Create issues in the `biomejs/biome` repository with playground reproduction links and specialized templates.

## Repo Isolation

This workflow targets **`biomejs/biome`** exclusively. Every `gh` command must use `--repo "biomejs/biome"`. Do not infer from working directory.

File links: `[{path}](https://github.com/biomejs/biome/blob/main/{path})`

## Validate Authentication

See `commons.md > Auth Validation`.

## Template Drift Check

Before generating the issue body, verify the template specs in this file still match the upstream `.github/ISSUE_TEMPLATE` files.

```bash
gh api repos/biomejs/biome/contents/.github/ISSUE_TEMPLATE \
  --jq '.[] | select(.name | endswith(".yml")) | "\(.name) \(.sha)"'
```

Compare against the known-good SHAs (last verified 2026-06-17):

| File                        | SHA                                        |
| --------------------------- | ------------------------------------------ |
| `01_formatter_bug.yml`      | `a5b1e863713e82a91a2ec4508cec794a846a9d50` |
| `02_lint_bug.yml`           | `1150a07f6e70b8d00e662329ea5cc940fc836162` |
| `03_bug.yml`                | `309033443c2ec2ab4a0f7bf0bfcb693103026aa8` |
| `04_task.yml`               | `030d1b51e028c9240ea6f9c690c0aeace61916c0` |
| `05_umbrella.yml`           | `b08776d17709268f8c20b01371bffbd6d642d257` |
| `06_commercial_request.yml` | `16d744fddb4a5310d4d9467c41e5c385c17982eb` |
| `config.yml`                | `53f16e16bb4352140251adffc6f9e51333e09072` |

If any SHA differs, fetch the changed template and update this spec before creating the issue:

```bash
gh api repos/biomejs/biome/contents/.github/ISSUE_TEMPLATE/{file}.yml --jq '.content' | base64 -d
```

## Determine Issue Type

From the issue description, infer which template fits best:

| Keywords                                            | Template                    | Issue type   | Title Prefix |
| --------------------------------------------------- | --------------------------- | ------------ | ------------ |
| format, formatter, formatting, prettier             | `01_formatter_bug.yml`      | `Bug`        | `📝 `        |
| lint, linter, rule, diagnostic, warning             | `02_lint_bug.yml`           | `Bug`        | `💅 `        |
| bug, broken, error, crash, panic, fails             | `03_bug.yml`                | `Bug`        | `🐛 `        |
| task, implement, add support (contributors)         | `04_task.yml`               | `Task`       | `📎 `        |
| umbrella, meta issue, tracking, high-level goal     | `05_umbrella.yml`           | `Umbrella`   | `☂️ `        |
| commercial, paid support, freelancer, contract work | `06_commercial_request.yml` | `Enterprise` | `💳 `        |

**If ambiguous**: Use AskUserQuestion with options: Formatter Bug, Linter Bug, General Bug, Task, Umbrella, Commercial Support. Umbrella issues should only be opened with prior maintainer discussion.

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

### Code of Conduct

- [x] I agree to follow Biome's Code of Conduct
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

### Code of Conduct

- [x] I agree to follow Biome's Code of Conduct
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

### Code of Conduct

- [x] I agree to follow Biome's Code of Conduct
```

### Task Template

```markdown
### Description

{summary of the task}
```

### Umbrella Template

```markdown
### Description

{summary of the umbrella, scope, and action items}
```

### Commercial Support Request Template

```markdown
### Description

{detailed request, requirements, and definition of done}

### Country

{country from which the freelancer would be hired}

### Timeframe

{urgency / desired schedule}
```

## Generate Title

Concise (5-10 words) with emoji prefix: `📝`, `💅`, `🐛`, `📎`, `☂️`, or `💳`.

## Create the Issue

```bash
gh issue create \
  --repo "biomejs/biome" \
  --title "$title" \
  --type "$issue_type" \
  --body "$(cat <<'EOF'
$body
EOF
)"
```

`$issue_type` is the value from the routing table. The label `S-Needs triage` is template metadata for bug forms, but direct CLI body creation does not apply YAML labels; omit `--label` unless `gh repo view biomejs/biome --json viewerPermission --jq .viewerPermission` returns `TRIAGE`, `WRITE`, `MAINTAIN`, or `ADMIN`.

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

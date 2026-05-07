# Codex Issue Workflow

Create issues in the `openai/codex` repository using the appropriate variant template.

## Repo Isolation

This workflow targets **`openai/codex`** exclusively. Every `gh` command must use `--repo "openai/codex"`. Do not infer from working directory.

File links: `[{path}](https://github.com/openai/codex/blob/main/{path})`

## Validate Authentication

See `commons.md > Auth Validation`.

## Template Drift Check

Before generating the issue body, verify the template specs in this file still match the upstream `.github/ISSUE_TEMPLATE` files. GitHub form templates change over time and silent drift produces issues with wrong section headers, missing fields, or invalid dropdown values.

```bash
gh api repos/openai/codex/contents/.github/ISSUE_TEMPLATE \
  --jq '.[] | select(.name | endswith(".yml")) | "\(.name) \(.sha)"'
```

Compare against the known-good SHAs (last verified 2026-05-07):

| File                    | SHA                                        |
| ----------------------- | ------------------------------------------ |
| `1-codex-app.yml`       | `6e294ee27bc924fc2c68b743bad26260297d13f9` |
| `2-extension.yml`       | `599bc08b428d6328c712f526549350daf0aada79` |
| `3-cli.yml`             | `4aff813e5f7bac7458d670f4cde35806493f9639` |
| `4-bug-report.yml`      | `4de88414600e6100720fefa2a324ce41d759cd7f` |
| `5-feature-request.yml` | `55ff9fbbcd590a8c0c2cc51f6a2c0406875fb3f4` |
| `6-docs-issue.yml`      | `456602e6acbbb73453a2d027e670896ef4a31335` |

**If any SHA differs** (or a new file appears), the upstream templates have changed. Before creating the issue:

1. Fetch the changed template(s):

   ```bash
   gh api repos/openai/codex/contents/.github/ISSUE_TEMPLATE/{file}.yml --jq '.content' | base64 -d
   ```

2. Diff against the spec in this file (field labels, dropdown options, required fields, auto-applied labels).

3. Tell the user:

   > ⚠️ Codex's issue templates have drifted from the spec in this skill (e.g., `3-cli.yml` SHA `<old>` → `<new>`). Please update the `yeet` skill in [`PaulRBerg/agent-skills`](https://github.com/PaulRBerg/agent-skills) — specifically `skills/yeet/references/issue-codex-cli.md` — and refresh the SHA table. Continue filing this issue using the closest matching fields, but flag any new required fields you couldn't fill.

4. Proceed with best-effort body generation using the fields the user can still fill from the new template.

## Determine Issue Type

The repo has six templates. Pick by surface area first, then kind.

| Surface / kind                                     | Template file           | Labels (auto)         |
| -------------------------------------------------- | ----------------------- | --------------------- |
| Codex App (desktop) bug                            | `1-codex-app.yml`       | `app`                 |
| IDE extension bug (VS Code, Cursor, Windsurf, ...) | `2-extension.yml`       | `extension`           |
| Codex CLI bug                                      | `3-cli.yml`             | `bug`, `needs triage` |
| Other bug (Codex Web, integrations, anything else) | `4-bug-report.yml`      | `bug`                 |
| Feature request (any variant)                      | `5-feature-request.yml` | `enhancement`         |
| Documentation issue                                | `6-docs-issue.yml`      | `docs`                |

Heuristics:

- "crash / error / broken / fails / doesn't work" → bug template; pick the surface (App vs Extension vs CLI vs Other).
- "feature / request / would be nice / add support for" → feature request.
- "docs / readme / unclear / example doesn't work" → docs issue.
- Mentions of `codex` command, terminal, TUI → CLI.
- Mentions of VS Code / Cursor / Windsurf → Extension.
- Mentions of desktop app, "Codex App", "About Codex" dialog → App.
- Codex Web / chatgpt.com/codex / cloud agent → Other Bug.

**If ambiguous**: Use AskUserQuestion with the surface options above.

## Title

Plain, concise (5-10 words). **Do not** add `[BUG]`, `[FEATURE]`, or any other prefix — the templates apply labels automatically and current issues in the repo do not use prefixes.

Good: `CLI hangs when piping large stdin`, `Add support for custom system prompts`
Bad: `[BUG] CLI hangs ...`, `[FEATURE] Add support ...`

## Generate Issue Body

Each template renders an H3 heading per field. Match the labels exactly so GitHub maps the body back to the form. Required fields must be non-empty; optional fields may be omitted entirely or filled with "None".

### Codex App Bug (`1-codex-app.yml`)

```markdown
### What version of the Codex App are you using (From "About Codex" dialog)?

{version}

### What subscription do you have?

{Plus/Pro/Team/Enterprise/Free — infer or ask}

### What platform is your computer?

{see commons.md > Platform String Normalization}

### What issue are you seeing?

{describe the bug; include error text, redact PII; prefer text over screenshots}

### What steps can reproduce the bug?

1. {step}
2. {step}

Include session id, token-limit usage, or context-window usage if relevant.

### What is the expected behavior?

{expected behavior}

### Additional information

{anything else, or omit}
```

### IDE Extension Bug (`2-extension.yml`)

```markdown
### What version of the IDE extension are you using?

{version}

### What subscription do you have?

{Plus/Pro/Team/Enterprise/Free}

### Which IDE are you using?

{VS Code / Cursor / Windsurf / ...}

### What platform is your computer?

{see commons.md > Platform String Normalization}

### What issue are you seeing?

{describe the bug}

### What steps can reproduce the bug?

1. {step}
2. {step}

### What is the expected behavior?

{expected behavior}

### Additional information

{anything else, or omit}
```

### CLI Bug (`3-cli.yml`)

```markdown
### What version of Codex CLI is running?

{output of `codex --version`}

### What subscription do you have?

{Plus/Pro/Team/Enterprise/Free}

### Which model were you using?

{e.g. gpt-5.2, gpt-5.2-codex — or omit if not specified}

### What platform is your computer?

{see commons.md > Platform String Normalization}

### What terminal emulator and version are you using (if applicable)?

{e.g. iTerm2 3.5.0, Ghostty 1.0, VS Code integrated terminal, Windows Terminal (WSL)}
{note any multiplexer: tmux / screen / zellij}

### What issue are you seeing?

{describe the bug; include thread id if applicable}

### What steps can reproduce the bug?

1. {step}
2. {step}

### What is the expected behavior?

{expected behavior}

### Additional information

{anything else, or omit}
```

### Other Bug (`4-bug-report.yml`)

For Codex Web, integrations, or anything that isn't App/Extension/CLI.

```markdown
### What issue are you seeing?

{describe the bug}

### What steps can reproduce the bug?

1. {step}
2. {step}

### What is the expected behavior?

{expected behavior}

### Additional information

{anything else, or omit}
```

### Feature Request (`5-feature-request.yml`)

```markdown
### What variant of Codex are you using?

{App / IDE Extension / CLI / Web}

### What feature would you like to see?

{describe the feature and the problem it solves}

### Additional information

{workarounds, alternatives, related issues — or omit}
```

### Documentation Issue (`6-docs-issue.yml`)

The "type" field is a multi-select dropdown; render selected items as a bullet list.

```markdown
### What is the type of issue?

- {Documentation is missing | Documentation is incorrect | Documentation is confusing | Example code is not working | Something else}

### What is the issue?

{describe the problem}

### Where did you find it?

{URL(s) — or omit}
```

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

Labels are applied automatically by the template — do not pass `--label`.

Display: "Created: $URL"

## Comment on Existing Issue

See `commons.md > Comment on Existing Issue`, using repo `"openai/codex"`.

## Environment Detection

- **Codex CLI version**: `codex --version 2>/dev/null || echo "unknown"`
- **Platform**: See `commons.md > Platform String Normalization`. The CLI/Extension/App templates suggest `uname -mprs` on macOS/Linux; the commons override (`scripts/get-macos-version.sh`) takes precedence for macOS so the value is human-readable.
- **IDE** (extension issues): Ask user or infer from context.
- **Terminal** (CLI issues): Ask user — cannot be reliably auto-detected from inside an agent shell. `$TERM_PROGRAM` is a hint but not authoritative.
- **Codex App version**: From the in-app "About Codex" dialog. Ask the user; do not guess.

## Examples

```bash
# CLI bug
"CLI hangs when piping large stdin"

# Codex App bug
"Codex Desktop pet overlay cannot be dragged on secondary monitor"

# Extension bug
"VS Code extension fails to send when prompt-history is not an array"

# Feature request (any variant)
"Add support for custom system prompts"

# Docs issue
"Installation docs don't mention npm prerequisites"

# Other bug (Web)
"Codex Web session sync drops messages after refresh"
```

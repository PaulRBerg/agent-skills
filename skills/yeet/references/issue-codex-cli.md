# Codex Issue Workflow

Create issues in the `openai/codex` repository using the appropriate variant template.

## Repo Isolation

This workflow targets **`openai/codex`** exclusively. Every `gh` command must use `--repo "openai/codex"`. Do not infer from working directory.

File links: `[{path}](https://github.com/openai/codex/blob/main/{path})`

## Validate Authentication

See `commons.md > Auth Validation`. The repository context read below is the auth check.

## Template Drift Check

Before generating the issue body, verify the template specs in this file still match the upstream `.github/ISSUE_TEMPLATE` files. GitHub form templates change over time and silent drift produces issues with wrong section headers, missing fields, or invalid dropdown values.

```bash
scripts/yeet-context.sh repo openai/codex --issue-templates
```

Use `repository.viewerPermission` for label capability. Compare `repository.issueTemplateTree.entries` names and SHAs against the known-good SHAs (last verified 2026-06-17):

| File                    | SHA                                        |
| ----------------------- | ------------------------------------------ |
| `1-codex-app.yml`       | `6e294ee27bc924fc2c68b743bad26260297d13f9` |
| `2-extension.yml`       | `599bc08b428d6328c712f526549350daf0aada79` |
| `3-cli.yml`             | `cfd368c0ba798d4f513edd5548fd185d761ed15d` |
| `4-bug-report.yml`      | `4de88414600e6100720fefa2a324ce41d759cd7f` |
| `5-feature-request.yml` | `745c347965c2e58f8e8e4437009f2c8ae0059878` |
| `6-docs-issue.yml`      | `1957b6035a58950329d87d4c24e67faf98c00572` |

**If any SHA differs** (or a new file appears), the upstream templates have changed. Before creating the issue:

1. Fetch the changed template(s):

   ```bash
   gh api repos/openai/codex/contents/.github/ISSUE_TEMPLATE/{file}.yml --jq '.content' | base64 -d
   ```

2. Diff against this routing file and the matching `references/templates/codex/` template reference (field labels, dropdown options, required fields, auto-applied labels).

3. Tell the user:

   > ⚠️ Codex's issue templates have drifted from the spec in this skill (e.g., `3-cli.yml` SHA `<old>` → `<new>`). Please update the `yeet` skill in [`PaulRBerg/agent-skills`](https://github.com/PaulRBerg/agent-skills) — specifically `skills/yeet/references/issue-codex-cli.md` and the matching file under `skills/yeet/references/templates/codex/` — and refresh the SHA table. Continue filing this issue using the closest matching fields, but flag any new required fields you couldn't fill.

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
| Documentation issue                                | `6-docs-issue.yml`      | `documentation`       |

Heuristics:

- "crash / error / broken / fails / doesn't work" → bug template; pick the surface (App vs Extension vs CLI vs Other).
- "feature / request / would be nice / add support for" → feature request.
- "docs / readme / unclear / example doesn't work" → docs issue.
- Mentions of `codex` command, terminal, TUI → CLI.
- Mentions of VS Code / Cursor / Windsurf → Extension.
- Mentions of desktop app, "Codex App", "About Codex" dialog → App.
- Codex Web / chatgpt.com/codex / cloud agent → Other Bug.

**If ambiguous**: Use AskUserQuestion with the surface options above.

Labels listed above are template metadata. Direct `gh issue create --body` creation does not apply YAML issue-form labels; omit `--label` unless cached `repository.viewerPermission` returns `TRIAGE`, `WRITE`, `MAINTAIN`, or `ADMIN`.

## Title

Plain, concise (5-10 words). **Do not** add `[BUG]`, `[FEATURE]`, or any other prefix — label metadata lives in the templates and current issues in the repo do not use prefixes.

Good: `CLI hangs when piping large stdin`, `Add support for custom system prompts`
Bad: `[BUG] CLI hangs ...`, `[FEATURE] Add support ...`

## Generate Issue Body

Each template renders an H3 heading per field. Match the labels exactly so GitHub maps the body back to the form. Required fields must be non-empty; optional fields may be omitted entirely or filled with "None".

After selecting the issue type, load exactly one template reference:

| Template file           | Reference                                         |
| ----------------------- | ------------------------------------------------- |
| `1-codex-app.yml`       | `references/templates/codex/1-codex-app.md`       |
| `2-extension.yml`       | `references/templates/codex/2-extension.md`       |
| `3-cli.yml`             | `references/templates/codex/3-cli.md`             |
| `4-bug-report.yml`      | `references/templates/codex/4-bug-report.md`      |
| `5-feature-request.yml` | `references/templates/codex/5-feature-request.md` |
| `6-docs-issue.yml`      | `references/templates/codex/6-docs-issue.md`      |

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

Direct CLI creation with `--body` does not apply YAML issue-form labels. Do not pass `--label` on the external repo unless cached `repository.viewerPermission` says labels are allowed.

Display: "Created: $URL"

## Comment on Existing Issue

See `commons.md > Comment on Existing Issue`, using repo `"openai/codex"`.

## Environment Detection

- **Codex CLI version**: `codex --version 2>/dev/null || echo "unknown"`
- **Codex doctor report**: `codex doctor --json 2>/dev/null || echo "not available"` — include for CLI bugs when supported; review/redact before posting.
- **Platform**: See `commons.md > Platform String Normalization`; use `scripts/get-macos-version.sh` for macOS instead of raw `uname` output.
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

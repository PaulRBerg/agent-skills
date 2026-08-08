# Codex Issue Workflow

Create an issue in `openai/codex`. Every `gh` command must use that repository; never infer it from the working
directory.

## Live Form Workflow

1. Fetch authenticated repository/template context with
   `<skill-dir>/scripts/yeet-context.sh repo openai/codex --issue-templates`. Resolve `<skill-dir>` to the absolute
   directory containing the owning `SKILL.md`.
2. Select the live YAML form matching the affected surface and intent: app, extension, CLI, generic bug, feature, or
   docs. Template selection and whether an existing discussion/issue is a better target remain agent judgments.
3. Run `<skill-dir>/scripts/issue-form.py inspect --repo openai/codex --template <selected.yml>`. Do not compare SHAs or
   consult a static template mirror.
4. Compose answers keyed by returned field IDs. Gather only requested environment facts: `codex --version`, relevant
   extension/app version, OS, shell/terminal, model/config, reproduction, logs, and regression range. Match every
   dropdown and multi-select to live options exactly.
5. Mark a required checkbox true only when repository/user evidence verifies the attestation. Render through
   `<skill-dir>/scripts/issue-form.py render`; resolve all missing or invalid answers before posting.
6. Run `posting.md > External-disclosure Review` on the title, answers, logs, config, and rendered body. Remove
   credentials, unsuitable private paths or repository names, and unrelated transcript material. Compose a concise title
   after the live prefix.
7. Post with `gh issue create --repo openai/codex --title ... --body-file ...`, adding labels/type from rendered
   metadata only when cached permission allows them. Follow `posting.md` idempotency handling before retrying any
   failure. If rendered metadata includes `projects`, apply each entry after creation with
   `posting.md > Project-Template Metadata`; a project-add failure is partial completion and never an issue-recreation
   trigger.

The helper owns form parsing and exact body structure. The agent owns template selection, answer/title composition,
environment interpretation, the external-disclosure review, semantic labels, and posting. Follow
`posting.md > Error Handling and Idempotency` after any ambiguous result; never recreate a possible partial issue.

For a comment on an existing issue, use `posting.md > Comment on Existing Issue` with this fixed repository. Finish a
successful creation with the verified URL and the `### 🚀 Issue created` receipt.

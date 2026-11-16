# Codex Issue Workflow

Create an issue in `openai/codex`. Every `gh` command must use that repository. Never infer it from the working
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
   extension/app version, subscription plan, model/config, OS, shell/terminal, reproduction, and logs. Take the plan
   only from user or environment evidence, and ask when neither supplies it. When the form requests a doctor report,
   paste the `codex doctor --json` output. Match every dropdown and multi-select to live options exactly.
5. Search open and closed issues with `gh search issues --repo openai/codex "<distinctive terms>"`. If an issue already
   reports the problem, do not create a duplicate. Post new user-supplied facts there with
   `posting.md > Comment on Existing Issue`. Otherwise, report the match under `### 🔎 Similar items` and stop. Do not
   post a comment that only agrees. Upstream asks for a 👍 reaction, so add one only when the user asks. If
   `codex --version` is older than `npm view @openai/codex version`, tell the user that the latest release may already
   fix the bug. Mark a required checkbox true only when repository/user evidence verifies the attestation. Render
   through `<skill-dir>/scripts/issue-form.py render`. Resolve all missing or invalid answers before posting.
6. Run `posting.md > External-disclosure Review` on the title, answers, logs, config, doctor report, and rendered body.
   The doctor report contains local paths, configuration, and MCP server names. Remove credentials, unsuitable private
   paths or repository names, and unrelated transcript material. Compose a concise title after the live prefix.
7. Post with `gh issue create --repo openai/codex --title ... --body-file ...`, adding labels/type from rendered
   metadata only when cached permission allows them. Follow `posting.md` idempotency handling before retrying any
   failure. If rendered metadata includes `projects`, apply each entry after creation with
   `posting.md > Project-Template Metadata`. A project-add failure is partial completion and never an issue-recreation
   trigger.

The helper owns form parsing and exact body structure. The agent owns template selection, answer/title composition,
environment interpretation, the external-disclosure review, semantic labels, and posting. Follow
`posting.md > Error Handling and Idempotency` after any ambiguous result. Never recreate a possible partial issue.

For a security vulnerability, do not create a public issue. Point the user to the repository's `SECURITY.md` policy and
stop.

For a comment on an existing issue, use `posting.md > Comment on Existing Issue` with this fixed repository. Finish a
successful creation with the verified URL and the `### 🚀 Issue created` receipt.

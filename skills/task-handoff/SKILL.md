---
argument-hint: "[task-to-handoff]"
compatibility:
  Requires Git, `ai-handoff` on PATH, and local file-write access. Default publication also requires macOS `pbcopy` and
  `pbpaste`; `create --no-clipboard` does not. Generated launch commands require an authenticated Codex CLI.
coordination: exempt
name: task-handoff
skill-dependencies:
  - codex-handoff
description:
  Create one decision-complete task handoff in its repository or on the Desktop for cross-repository work, return a
  command for a fresh interactive Codex session, and recommend Codex subagent delegation for complex tasks.
---

# Task Handoff

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

If these instructions are already present from a slash or dollar invocation, follow them directly; do not invoke this
skill again through a skill tool.

Turn one continuation task into one self-contained task handoff for a fresh agent chat. Write the handoff only; do not
implement, edit tracked files, commit, push, launch Codex, or change ignore configuration.

Task-handoff writes one decision-complete file for a fresh, separate session. For a simple task, that isolated file is
enough for direct execution in the receiving session. For a complex task, resolve `~/.agents/skills/codex-handoff` to
its expanded absolute directory, require a readable `SKILL.md` there, and embed that absolute skill path in the handoff.
Direct the receiving session to load it from Plan mode and use the handoff as its task specification so Codex subagents
can implement it; do not rely on bare `$codex-handoff` discovery. Task-handoff still creates exactly one file and never
launches orchestration itself. Use task-handoff when work continues later or elsewhere, and an in-session handoff skill
when implementing an approved plan now.

## Select the work

Use `$ARGUMENTS` when present. Otherwise infer the next unfinished step from the last assistant response and relevant
transcript. Treat the task as a scope selector: retain only evidence needed for that work.

Resolve discoverable facts before writing. Inspect every involved repository's instructions, relevant task surface, and
working-tree state; distinguish completed work, partial work, and remaining work while preserving user and
concurrent-agent changes. If the task is already complete, report the evidence and create nothing. Ask only when an
unresolved choice materially changes scope, safety, execution approach, verification, repository membership, or handoff
placement.

Classify the handoff from the user's requested outcome, expected evidence, and authorized actions. Reason about the
task; do not keyword-match it. Select exactly one lowercase category:

- `implementation`: change code, configuration, documentation, or data to reach a defined end state.
- `investigation`: explain a bounded symptom, behavior, or failure with evidence and a recommended next action.
- `research`: answer or compare options using collected evidence and deliver a recommendation or decision record.
- `audit`: assess a defined surface against explicit criteria and report prioritized, evidence-backed findings.
- `operations`: carry out a bounded maintenance, release, migration, or coordination workflow with state checks and
  rollback boundaries.

When a task contains supporting work from another category, use the category of its primary deliverable.

Classify its execution approach as `simple` or `complex`. Mark it complex only when delegation would materially improve
latency, correctness, or verification, such as independently implementable scopes, dependency waves, multiple or
unfamiliar subsystems, or a brief likely to exceed roughly 25-30 minutes. File and repository count alone are not
complexity signals. Otherwise mark it simple.

Infer repositories from local paths and relevant context. Do not include the current repository merely because the skill
runs there when the task selects only other repositories. Create exactly one handoff file, whether the task touches one
repository or many. A single-repository handoff belongs in that repository at
`<REPOSITORY>/.ai/task-handoffs/<HANDOFF_NAME.md>`. A cross-repository handoff belongs at
`~/Desktop/.ai/task-handoffs/<HANDOFF_NAME.md>` only when it spans at least two repositories. For cross-repository work,
include every involved repository and choose the first repository to tackle as the Codex launch directory. Stop before
writing when the repository set or the required operation order is unclear.

Choose one meaningful unique filename matching `^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*[.]md$`; do not force a `PLAN_` prefix. If
the target exists, add a semantic qualifier, falling back to `_YYYY_MM_DD_HHMMSS` only when no meaningful qualifier
distinguishes it.

## Create the handoff

Compose the semantic draft body in a new scratch file outside every involved repository, for example under `$TMPDIR`.
Write it in one operation. The draft must be non-empty and begin immediately with one H1 heading. Do not add YAML
frontmatter or begin the draft with `---`. Never add `## Handoff category`, `## Execution status`, or
`## Handoff cleanup`; `ai-handoff create` reserves and appends them. Make every body decision-complete for an agent with
access to the named repositories but none of this transcript. Include:

- objective, success criteria, and explicit exclusions;
- verified current state, partial changes, and completed prerequisites;
- changes keyed to stable paths, symbols, interfaces, schemas, or commands rather than line numbers;
- task-specific evidence, procedure, material edge cases, and failure behavior;
- targeted validation, acceptance scenarios, and rollout, compatibility, or authority requirements;
- exact repository-relative write scopes and a ready-to-run `ai-coord start '<label>' '<path>'...` command derived from
  those scopes, using `--recursive` only when the handoff genuinely cannot enumerate a subtree;
- assumptions resolved from repository evidence or explicit user decisions.

Add a `## Execution approach` section. For a simple task, direct the receiving session to execute this one isolated
handoff without invoking an in-session handoff skill. For a complex task, expand and canonicalize
`~/.agents/skills/codex-handoff`, verify that `<resolved-directory>/SKILL.md` is readable, and write that resolved
absolute directory path into the execution approach. Explicitly direct the receiving session to enter Plan mode, load
the skill from that path, and use this file as the decision-complete task specification. Do not name only
`$codex-handoff`; let codex-handoff choose the smallest effective subagent team instead of prescribing its manifest
here. Retain the resolved directory for `--before-work-skill` when validating and publishing so the generated Codex
prompt also names the skill and directs the receiving session to load it before any task work. Stop before writing the
handoff if the installed skill cannot be resolved.

Tailor the body to its category. An implementation handoff specifies the intended change, data flow, and compatibility.
An investigation handoff specifies the question or symptom, available evidence, reproduction or observation method, and
the decision the findings must support. A research handoff specifies questions, sources or evidence to collect,
alternatives, and recommendation criteria. An audit handoff specifies the assessed surface, criteria, evidence method,
and severity or prioritization model. An operations handoff specifies preconditions, ordered state transitions,
authority boundaries, observability, and rollback or recovery.

For a cross-repository handoff, add a `## Repository order` section with a numbered sequence. Its first item must name
the repository to tackle first; every item must name the canonical root, role, exact write scope, prerequisite or
handoff condition, repository-local validation, and its own ready-to-run `ai-coord start` command. Also state the
combined acceptance criteria. Use literal repository-relative paths without globs; use directories only with
`--recursive`. Use direct transcript excerpts when exact wording is material; otherwise summarize relevant context to
keep the handoff compact. Leave no placeholders, open task choices, or references that require the old chat. The CLI
requires the exact line `## Repository order` in every cross-repository draft.

When placement or the repository set is uncertain, pre-validate before investing in the body. Pass every involved
repository with `--repo`; for cross-repository work, pass the first repository to tackle as `--launch-repo`. Omit
`--launch-repo` only when exactly one repository is involved and its default is suitable:

```sh
ai-handoff create --check \
  --repo '<candidate-repository>' \
  [--repo '<additional-repository>' ...] \
  [--launch-repo '<first-repository-to-tackle>'] \
  [--before-work-skill '<resolved-codex-handoff-directory>'] \
  --category '<task-category>' \
  --task '<concise-task>' \
  '<HANDOFF_NAME.md>'
```

This validates the repository roots, optional launch repository, category, filename, target placement, and required
ignore rule without reading a draft or writing anything. For a complex handoff, include `--before-work-skill` with the
resolved absolute `codex-handoff` directory; for a simple handoff, omit it. The option validates that the directory is
absolute and contains a readable `SKILL.md`. The command prints tab-separated `target`, `launch_repo`, and `category`
rows.

Publish with one call after the body is complete:

```sh
ai-handoff create \
  --repo '<candidate-repository>' \
  [--repo '<additional-repository>' ...] \
  [--launch-repo '<first-repository-to-tackle>'] \
  [--before-work-skill '<resolved-codex-handoff-directory>'] \
  --category '<task-category>' \
  --task '<concise-task>' \
  --draft '<scratch-draft-path>' \
  '<HANDOFF_NAME.md>'
```

`ai-handoff` canonicalizes each repository to its physical Git toplevel and deduplicates them. The optional launch
repository must be involved; with exactly one repository it defaults to that repository. For a complex handoff, pass the
same `--before-work-skill` value used during validation; this appends a final prompt sentence that directly names
`<resolved-codex-handoff-directory>/SKILL.md` and requires loading it before any task work. It validates the draft,
appends YAML frontmatter recording `category`, `created`, `launch_repo`, `repos`, `origin`, and `task`, plus the fixed
category, execution-status, and cleanup contracts, then atomically publishes a new target without overwriting and copies
the generated Codex command after `pbcopy`/`pbpaste` readback verification. Errors are written to stderr with an
`ai-handoff: ` prefix; usage errors exit 2 and operational errors exit 1. On a correctable validation error, fix the
draft or arguments and rerun the same command: failure leaves no handoff behind. There is no temporary run directory,
cancel command, or manual target cleanup.

The appended `## Handoff cleanup` section contains one command:

```sh
ai-handoff archive '<absolute-handoff-path>'
```

The receiving agent runs that command only after completing the handoff. It archives to
`$HOME/.local/share/task-handoffs/archive/<origin-name>/`, where `<origin-name>` is the basename of the directory
containing `.ai`; a collision receives a UTC `_YYYY_MM_DD_HHMMSS` suffix. This skill never archives handoffs itself.

For noninteractive ai-coord findings triage, uppercase the finding ID only in the deterministic filename
`FINDING_<UPPERCASE_ID>.md`. Preserve the ledger ID's original spelling in the exact machine-readable line
`Source finding: <ID>` in the semantic draft body, then publish without clipboard access:

```sh
ai-handoff create --no-clipboard \
  --repo '<candidate-repository>' \
  --category '<task-category>' \
  --task '<concise-task>' \
  --draft '<scratch-draft-path>' \
  'FINDING_<UPPERCASE_ID>.md'
```

This mode keeps publication and structural validation but skips `pbcopy` and `pbpaste`. Never overwrite an existing
deterministic finding handoff: resolve the existing handoff before creating another one. When creating a finding handoff
interactively rather than through the autonomous triage runtime, run the following only after successful publication so
the ledger record moves from `pending` to `handed-off`. Preserve the ledger ID's original spelling:

```sh
ai-coord finding handoff '<original-id>' --path '.ai/task-handoffs/FINDING_<UPPERCASE_ID>.md'
```

Read the tab-separated stdout rows as the final record: `handoff`, `launch_repo`, `category`, and `command`. Never
execute the command.

## Report

On success, finish with `### ✅ Task handoff ready — <task>`. List the final record's handoff path, canonical launch
repository, category, selected execution approach, and exact Codex command in one code block. Do not repeat the handoff
body or mention clipboard copying or verification.

For a blocker, finish with `### ⛔ Task handoff not written — <reason>` and state that no handoff file was created.

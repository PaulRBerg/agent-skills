---
argument-hint: "[task-to-handoff]"
compatibility:
  Requires Bash 3.2, Git, and local file-write access. Default finalization also requires macOS pbcopy and pbpaste;
  `finalize --no-clipboard` does not. Generated cleanup commands use POSIX shell utilities; generated launch commands
  require authenticated Codex and Claude CLIs.
coordination: exempt
disable-model-invocation: false
name: task-handoff
user-invocable: true
description:
  Create one decision-complete task handoff in its repository or on the Desktop for cross-repository work, and return a
  command for a fresh interactive Codex session.
---

# Task Handoff

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

If these instructions are already present from a slash or dollar invocation, follow them directly; do not invoke this
skill again through a skill tool.

Turn one continuation task into one self-contained task handoff for a fresh agent chat. Write the handoff only; do not
implement, edit tracked files, commit, push, launch Codex, or change ignore configuration.

Task-handoff writes a decision-complete file for a fresh, separate session. Codex-handoff and claude-handoff orchestrate
implementation within the current session from Plan mode; use task-handoff when work continues later or elsewhere, and
an in-session handoff skill when implementing an approved plan now.

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

## Prepare drafts

Resolve `scripts/task-handoff.sh` relative to this loaded `SKILL.md`. Invoke it without reading its implementation:

```sh
bash <skill-dir>/scripts/task-handoff.sh prepare \
  --repo '<candidate-repository>' \
  --plan '<first-repository-to-tackle>' '<HANDOFF_NAME.md>' '<task-category>' '<concise-task>' \
  [--repo ...]
```

Pass every involved repository with `--repo`, then exactly one `--plan`. Its repository must be the first one to tackle
and becomes the Codex launch directory. Pass the inferred category exactly as one of the five values above. The helper
validates and canonicalizes physical Git roots, the launch repository, the placement-dependent target, categories,
filenames, and required macOS tools before creating mode-0700 temporary state. Use its shell-quoted `run_dir`, `repo`,
and `plan` records as authoritative; the `plan` record gives its launch repository, repository-relative handoff path,
absolute target, category, and draft path. The helper also verifies that a repository-local target is ignored.

The helper reports the draft path but does not create the file, so write it as a new file in one write; never expect an
existing empty draft. Write only the semantic handoff body to the draft, beginning immediately with one H1 heading. Do
not add YAML frontmatter or begin the draft with `---`. Never add `## Handoff category`, `## Execution status`, or
`## Handoff cleanup`; `finalize` reserves and appends them. Make every body decision-complete for an agent with access
to the named repositories but none of this transcript. Include:

- objective, success criteria, and explicit exclusions;
- verified current state, partial changes, and completed prerequisites;
- changes keyed to stable paths, symbols, interfaces, schemas, or commands rather than line numbers;
- task-specific evidence, procedure, material edge cases, and failure behavior;
- targeted validation, acceptance scenarios, and rollout, compatibility, or authority requirements;
- exact repository-relative write scopes and a ready-to-run `ai-coord start '<label>' '<path>'...` command derived from
  those scopes, using `--recursive` only when the handoff genuinely cannot enumerate a subtree;
- assumptions resolved from repository evidence or explicit user decisions.

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
keep the handoff compact. Leave no placeholders, open task choices, or references that require the old chat. `finalize`
rejects a cross-repository draft without this section.

## Finalize or cancel

After the body is complete, run:

```sh
bash <skill-dir>/scripts/task-handoff.sh finalize '<run-dir>'
```

`finalize` re-runs preflight, rejects an empty, frontmatter-prefixed, non-H1, or reserved-heading draft, constructs the
complete staged file with YAML metadata plus the fixed category, execution-status, and archive cleanup contracts,
validates the complete structure, publishes the new handoff without overwriting, and copies and byte-verifies the Codex
command. The metadata records the finalization time, launch repository, repositories in their stored order, absolute
published origin, category, and task. The cleanup contract archives only the completed handoff under
`$HOME/.local/share/task-handoffs/archive/<origin-name>/`, where `<origin-name>` is the basename of the directory
containing `.ai`; it uses a UTC `_YYYY_MM_DD_HHMMSS` suffix and waits for a new timestamp when a destination already
exists. It rolls back helper-created targets and now-empty directories on handled errors, `INT`, or `TERM`; it cannot
make publication atomic across filesystems or survive power loss or `SIGKILL`.

For noninteractive ai-coord findings triage, uppercase the finding ID only in the deterministic filename
`FINDING_<UPPERCASE_ID>.md`. Preserve the ledger ID's original spelling in the exact machine-readable line
`Source finding: <ID>` in the semantic draft body, then finalize without clipboard access:

```sh
bash <skill-dir>/scripts/task-handoff.sh finalize --no-clipboard '<run-dir>'
```

This mode keeps the same publication, structural validation, rollback, cleanup, and final `plan` record, but skips all
`pbcopy` and `pbpaste` checks, copying, and readback verification. On a handled failure, correct the retained draft and
retry the same finalize command; if abandoning the handoff, use `cancel` to remove only its temporary run state. Never
overwrite an existing deterministic finding handoff: resolve the existing handoff before preparing another run.

When creating a finding handoff interactively rather than through the autonomous triage runtime, run the following only
after successful finalization so the ledger record moves from `pending` to `handed-off`. Preserve the ledger ID's
original spelling:

```sh
ai-coord finding handoff '<original-id>' --path '.ai/task-handoffs/FINDING_<UPPERCASE_ID>.md'
```

The final `plan` record contains `handoff=`, canonical `launch_repo=`, `category=`, and the exact command after
`command=` plus the exact Claude Code command after `claude_command=`. Never execute either command. If a correctable
draft error occurs, edit the draft and retry. If abandoning or blocking before successful finalization, remove only
helper-created temporary state with:

```sh
bash <skill-dir>/scripts/task-handoff.sh cancel '<run-dir>'
```

Never create or remove targets yourself. A failed or cancelled run must leave no helper-created handoff file.

## Report

On success, finish with `### ✅ Task handoff ready — <task>`. List the final record's handoff path, canonical launch
repository, category, and both exact commands in one code block, Codex first. Do not repeat the handoff body or mention
clipboard copying or verification.

For a blocker, finish with `### ⛔ Task handoff not written — <reason>` and state that no handoff file was created.

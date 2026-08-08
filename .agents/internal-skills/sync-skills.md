---
name: sync-skills
description:
  Review and synchronize coupled skill files; align shared wording and workflow contracts, fix drift, preserve
  skill-specific content.
---

# Sync Skills

Review skill files that intentionally share wording, policies, or workflow contracts. Patch only real drift; preserve
skill-specific behavior and examples.

## Scope

Default: run every sync group below. If the request names a group, file, or subset, run only that group.

Work only in the files listed for the selected groups.

## Sync Groups

### Supported Chat Hosts

Files:

- `skills/agents-docs/SKILL.md`
- `skills/agents-introspection/SKILL.md`
- `skills/copy-transcript-path/SKILL.md`

Keep each `## Supported Chat Hosts` block textually identical. The block requires detecting the current chat host before
any work and stopping unsupported harnesses with `This skill only works in Claude Code or Codex CLI.`

### Commit Workflow Semantics

Files:

- `skills/commit/SKILL.md`
- `skills/claude-handoff/SKILL.md`
- `skills/codex-handoff/SKILL.md`
- `skills/fresh-eyes-sweep/SKILL.md`
- `skills/repo-harmonization/SKILL.md`
- `.agents/internal-skills/publish-skills.md`

Treat these as in scope:

- `$commit` owns Conventional Prefix or Natural Language message semantics; `ai-commit` owns deterministic preparation,
  commit, index, and push mechanics.
- Orchestrators pass only attributable paths to `$commit`, and push only with explicit or standing authorization.
- Workflows that require propagation treat `BEHIND` as safe noncompletion, never as a successful push.
- Consumers rely on `ai-commit` diagnostics and `$commit` recovery policy instead of duplicating bypass rules.

Treat these as out of scope unless the request explicitly names them:

- Transaction command details owned by `skills/commit/SKILL.md`.
- Orchestration, publication, or sweep behavior unrelated to the shared commit boundary.

### Handoff Planning Guidance

Files:

- `skills/codex-handoff/SKILL.md`
- `skills/codex-handoff/references/claude-code-host.md`
- `skills/codex-handoff/references/codex-cli-host.md`
- `skills/claude-handoff/SKILL.md`

`codex-handoff/SKILL.md` is the platform-neutral contract for delegation from Claude Code or Codex CLI; its two host
adapters specialize runtime mechanics. `claude-handoff` remains Claude Code only. Compare the platform-neutral blocks in
both skill entrypoints and keep them semantically identical, adjusted only for the parent/agent noun and runtime:

- Contract bullets: the Plan-mode gate; parent ownership of decisions, the final plan, and orchestration; the
  no-redesign rule for implementation agents; the smallest-effective-team and eight-implementation-agent limits; the
  brief-sizing rule that splits any brief likely to exceed roughly 25-30 minutes; and parent implementation work being
  limited to orchestration, integrity checks, failure handling, and the conditional polish passes.
- User model preferences: an explicit preference overrides the normal task-complexity model selection for every research
  and implementation agent unless the user narrows its scope; an unavailable preferred model requires user approval
  before fallback.
- Follow-on authorization: the approved outcome, not the initial manifest or worker write scopes, is the authorization
  boundary; workers stop and report newly discovered out-of-scope prerequisites, while the parent extends the manifest,
  coordinates the new scope, and delegates the smallest sufficient in-repository fix without asking the user again.
- Pre-plan research delegation: trigger it for uncertain scope, multiple or unfamiliar subsystems, or materially slower
  serial evidence gathering; keep zero research agents as the default; let the parent alone decide whether it runs
  without asking the user; require research agents to stay read-only, gather evidence, and return findings rather than
  decisions or plans; limit the separate research budget to three agents (`R1` through `R3`); feed findings into the
  final plan; and include the optional plan `Research:` traceability line.
- Strategy selection guidance: sequential vs parallel vs hybrid criteria, disjoint-write-scope requirement, wave
  semantics, the slowest-agent note, and the whole-handoff eight-implementation-agent limit with stable IDs and
  dependencies.
- The single-validation-owner rule: aggregate checks run once; every other agent runs only the narrowest checks proving
  its own edits; aggregate-check failures confined to files outside every agent's scope are attributed to unrelated
  concurrent work, not treated as blockers.
- The independent polish-selection rules: `$code-polish` retains its risk trigger list, including "file count alone is
  not a trigger"; `$agents-brain polish` applies to README.md, AGENTS.md or CLAUDE.md, durable context docs, and
  existing project-installed skills under `.agents/skills`, while source catalog skills under `skills/` remain excluded;
  both or neither pass may be selected.
- Before-launch session-claim guidance: delegated agents treat the orchestrating session's presence as authorization for
  their assigned work. The exact claim owner is host-specific.
- Platform-agnostic agent prompt requirements: outcome plus brief, exact write scope and dirty-work boundaries,
  validation assignment, soft time budget, authority boundary, delegation context, stopping rule, and reporting
  requirement.
- The structured result-field contract: status, summary, changed files, verification with each command and outcome,
  residual risks, and blockers.
- Failure classification: a returned blocker caused by a newly discovered, necessary in-repository fix or evidence
  change triggers parent-owned follow-on delegation without fresh authorization; user input is reserved for a changed
  outcome, material redesign, unrelated work, or an existing confirmation boundary. An evidenced tool or infrastructure
  failure permits exactly one same-agent continuation after inspecting partial edits, and a second infrastructure
  failure blocks. The continuation mechanics are platform-specific.
- Post-success skill-evolution review: run only after every required implementation agent succeeds and the overall task
  is verified; keep the judgment with the parent orchestrator; require credible recurrence and durable reuse rather than
  task size or difficulty; reject one-offs, rare contingencies, incidental cleanup, and speculative value; place a new
  skill in the work repository for project-specific reuse or `~/projects/agent-skills` for cross-project reuse; name
  every exact skill and reason when revision is warranted; allow at most one two-sentence suggestion that offers
  `$task-handoff` while reserving low-level details for that future handoff; never act automatically; and remain silent
  for blocked, failed, partial, or below-threshold work.
- Completion rules: success verification, dependent gating on failure, changed-files union dedupe, ordered and scoped
  polish invocation, polish skip and failure conditions, and cross-repository `$commit` behavior.
- Adapter integrity: each codex-handoff adapter implements the shared prompt, result, failure, and completion contracts
  without weakening them, while the shared entrypoint loads exactly one adapter.

Treat these as out of scope unless the request explicitly names them:

- Host selection and launch mechanics: the Claude adapter's runner and artifacts, the Codex adapter's native tools, and
  claude-handoff's Agent-tool calls.
- Same-agent continuation mechanics: the Claude adapter's runner resume, the Codex adapter's native follow-up, and
  claude-handoff's SendMessage flow.
- Research mechanics: the Claude adapter's read-only runner, per-agent artifacts, and result schema; the Codex adapter's
  native read-only agents; and claude-handoff's Explore subagent type and native result flow.
- Claude-adapter-only content: effort and timeout selection, progress streams, Monitor guidance, sentinel handling,
  dangerous bypass behavior, and subprocess command conventions.
- Codex-adapter-only content: harness concurrency limits, fresh-context spawning, native steering and waiting, inherited
  sandbox and approval controls, and native progress rendering.
- Each skill's default model configuration and its rules about re-running a failed agent. The defaults are intentionally
  different — the codex-handoff adapters choose GPT-5.6 model and effort tiers, while claude-handoff uses the default
  Explore model for research and `sonnet` for implementation — so never normalize them.
- Status reporting: the Claude adapter's dashboard system versus both native hosts' concise progress and completion
  reports.
- Frontmatter and `references/`/`scripts/` contents.

Verification is prose comparison of the in-scope blocks; there is no extractable helper data.

## Workflow

1. Verify repository context: `git rev-parse --git-dir`. If this fails, stop and tell the user to run from a git
   repository.
2. Resolve selected sync groups once. Do not broaden the group list after reading files unless the user asks.
3. Read the selected files and compare only the in-scope shared blocks or workflow contracts.
4. When drift exists, normalize all copies to one phrasing or value set. Reuse the clearest wording already present.
5. Prefer minimal patches. Do not rewrite whole sections just to make them symmetrical if the remaining differences are
   skill-specific.
6. If no drift exists, make no edits and report that the selected groups are already aligned.

## Verification

After editing Markdown, run from the repo root:

```bash
just prettier-write
just prettier-check
```

Re-read touched sections and confirm selected groups now match on shared wording or workflow contracts and still differ
only where their workflows require it.

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

Default: run every sync group below. If the request names a group, file, or subset, run only that group. Work only in
the files listed for the selected groups.

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
adapters specialize runtime mechanics. `claude-handoff` remains Claude Code only. The following topics must stay
semantically identical between the two entrypoints, adjusted only for the parent/agent noun and runtime — do not restate
their content here, the sync run reads both skills directly:

1. Plan-mode gate for implementation handoffs, the research-only any-mode exception, and the stop-before-planning
   boundary.
2. Parent ownership of decisions/plan/orchestration; implementation-agent no-redesign rule.
3. Smallest-effective-team rule + eight-implementation-agent limit; brief-sizing rule splitting any brief likely to
   exceed roughly 25-30 minutes.
4. User model-preference override (beats normal task-complexity selection for every research and implementation agent
   unless the user narrows scope); an unavailable preferred model needs user approval before fallback.
5. Follow-on authorization boundary: the approved outcome, not the initial manifest or worker write scopes, authorizes
   follow-on work; workers report new out-of-scope prerequisites, the parent extends scope and delegates without
   re-asking.
6. Pre-plan research delegation: zero agents by default, parent-only decision, read-only agents, findings not
   decisions/plans, budget capped at three agents (`R1`-`R3`), optional `Research:` traceability line.
7. Strategy selection: sequential/parallel/hybrid criteria, disjoint-write-scope requirement, wave semantics,
   slowest-agent note, whole-handoff eight-implementation-agent limit with stable IDs and dependencies.
8. Single-validation-owner rule: aggregate checks run once, every other agent runs only checks proving its own edits,
   failures confined to files outside every agent's scope attribute to unrelated concurrent work.
9. Polish-selection rules: `$code-polish` risk-trigger list (file count alone is not a trigger); `$agents-brain polish`
   targets README/AGENTS/CLAUDE.md and project-installed skills under `.agents/skills`, excluding `skills/`; either,
   both, or neither pass may run.
10. Before-launch session-claim guidance: orchestrating session's presence authorizes delegated work; claim owner is
    host-specific.
11. Platform-agnostic agent prompt requirements: outcome + brief, write scope and dirty-work boundaries, validation
    assignment, soft time budget, authority boundary, delegation context, stopping rule, reporting requirement.
12. Structured result-field contract: status, summary, changed files, verification (command + outcome), residual risks,
    blockers.
13. Failure classification: a blocker from a newly discovered necessary in-repository fix or evidence change triggers
    parent-owned follow-on without fresh authorization; an evidenced tool/infrastructure failure permits exactly one
    same-agent continuation, a second blocks.
14. Post-success skill-evolution review: parent-only judgment, only after full success and verification, requires
    credible recurrence and durable reuse (not size/difficulty), rejects one-offs and speculative value, at most one
    two-sentence `$task-handoff` suggestion, silent otherwise.
15. Completion rules: success verification, dependent gating on failure, changed-files union dedupe, ordered/scoped
    polish invocation, polish skip/failure conditions, cross-repository `$commit` behavior.
16. Adapter integrity: adapters implement the shared prompt/result/failure/completion contracts without weakening them;
    the shared entrypoint loads exactly one adapter.

Out of scope unless the request explicitly names it: host selection, launch, and continuation mechanics; research
mechanics; Claude-adapter-only content; Codex-adapter-only content; each skill's model defaults and its failed-agent
re-run rules — model defaults are intentionally different (codex-handoff adapters choose GPT-5.6 tiers; claude-handoff
uses `sonnet`) — never normalize them; status reporting style; frontmatter and `references/`/`scripts/` contents.

Verification is prose comparison of the in-scope blocks; there is no extractable helper data.

### Ai-skillet CLI consumers

Files:

- `skills/skill-map/SKILL.md`
- `skills/skill-doctor/SKILL.md`
- `skills/skill-harmonization/SKILL.md`

Keep the same ai-skillet minimum version, `0.1.0+`. Each consumer must invoke its appropriate ai-skillet subcommand
directly (`map` for skill-map and skill-harmonization; `doctor` for skill-doctor), with no retired Python, uv, ripgrep,
helper-resolution, wrapper, or fallback path.

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

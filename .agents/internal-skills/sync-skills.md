---
name: sync-skills
description:
  Review and synchronize coupled skill files. Align shared wording and workflow contracts, fix drift, and preserve
  skill-specific content.
---

# Sync Skills

Review skill files that intentionally share wording, policies, or workflow contracts. Patch only real drift. Preserve
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

Keep the shared host-detection guard at the start of each `## Supported Chat Hosts` block textually identical. The guard
requires detecting the current chat host before any work and stopping unsupported harnesses with
`This skill only works in Claude Code or Codex CLI.` Preserve any skill-specific prose that follows the guard under the
same heading. The guard is the heading plus its first paragraph. Verify it with the exact-block check under
Verification.

### Commit Workflow Semantics

Files:

- `skills/commit/SKILL.md`
- `.agents/internal-skills/publish-skills.md`

Treat these as in scope:

- `$commit` owns Conventional Prefix or Natural Language message semantics. `ai-commit` owns deterministic preparation,
  commit, index, and push mechanics. Consumer skills invoke `$commit` scoped to attributable paths instead of restating
  that boundary or duplicating bypass and recovery rules.
- `toolkit/commit/` is the `ai-commit` source that owns those deterministic mechanics.
- Workflows that require propagation treat `BEHIND` as safe noncompletion, never as a successful push.

Treat these as out of scope unless the request explicitly names them:

- Transaction command details owned by `skills/commit/SKILL.md`.
- Orchestration, publication, or sweep behavior unrelated to the shared commit boundary.

### Handoff Planning Guidance

Files:

- `skills/codex-handoff/SKILL.md`
- `skills/codex-handoff/references/claude-code-host.md`
- `skills/codex-handoff/references/codex-cli-host.md`
- `skills/claude-handoff/SKILL.md`

`codex-handoff/SKILL.md` is the platform-neutral contract for delegation from Claude Code or Codex CLI. Its two host
adapters specialize runtime mechanics. `claude-handoff` remains Claude Code only. The following topics must stay
semantically identical between the two entrypoints, adjusted only for the parent/agent noun and runtime. Do not restate
their content here. The sync run reads both skills directly:

1. Every handoff supports any host mode. It requires explicit plan approval before implementation launch. Research-only
   work stops before planning.
2. The parent owns decisions, the plan, and orchestration. Implementation agents must not redesign the plan.
3. Use the smallest effective team, with at most eight implementation agents. Split any brief likely to exceed roughly
   25-30 minutes.
4. The user's model preference overrides normal task-complexity selection for every research and implementation agent,
   unless the user narrows its scope. If the preferred model is unavailable, obtain user approval before using a
   fallback.
5. The approved outcome authorizes follow-on work. The initial manifest and worker write scopes do not define that
   authorization boundary. Workers report new out-of-scope prerequisites. The parent extends scope and delegates without
   asking again.
6. Pre-plan research uses zero agents by default. Only the parent decides whether to delegate research. Research agents
   are read-only and return findings, not decisions or plans. The budget is at most three agents (`R1`-`R3`). The
   `Research:` traceability line is optional.

   Ask about research that contradicts a user-stated fact before planning. Never absorb it into scope.

7. Strategy selection covers sequential/parallel/hybrid criteria, disjoint write scopes, wave semantics, and the
   slowest-agent note. The eight-implementation-agent limit applies to the whole handoff. IDs and dependencies remain
   stable.
8. One owner runs aggregate checks once. Every other agent runs only checks proving its own edits. Attribute failures by
   first ruling out the handoff's changes and tool side effects, including downstream failures. Continue only past
   evidenced unrelated failures while the handoff's own checks pass. Size verification to the outcome. Briefs add no
   validation machinery that the plan does not call for.
9. Use the `$code-polish` risk-trigger list. File count alone is not a trigger. `$agents-brain maintain` targets
   README.md, AGENTS.md, CLAUDE.md, durable context docs, project-installed skills under `.agents/skills`, and existing
   git-tracked source-catalog skills under `skills/` for prose-only edits. Installed copies under managed agent-config
   roots remain excluded. Either, both, or neither pass may run.
10. Before implementation launch, the parent owns a claim covering every delegated write scope and requires `READY`. For
    a queued or blocked claim, run `ai-coord wait`. Resubmit the claim on each wake. Never end the turn to pause for
    that claim.

    Delegates use the parent identity. They treat its claim as authorization. They never run coordination lifecycle
    commands. Identity propagation and wait mechanics are host-specific.

11. Platform-agnostic agent prompts require an outcome and brief, write scope and dirty-work boundaries, validation
    assignment, soft time budget, authority boundary, delegation context, stopping rule, and reporting requirement.
12. The structured result contract requires status, summary, changed files, verification (command + outcome), residual
    risks, and blockers.
13. A newly discovered necessary in-repository fix or evidence change that blocks work triggers parent-owned follow-on
    without fresh authorization. An evidenced tool/infrastructure failure permits exactly one same-agent continuation. A
    second failure blocks the work.
14. Already-authorized skill repairs are separate from the optional evolution review. The parent owns completion.
    Subagents report evidence without expanding scope. One verified occurrence is enough. Independent repairs do not
    require main-task success. Finish them before the final report, after required handoff work or a concrete blocker.

    Plan Mode prohibits edits. The optional review remains parent-only. It requires full success, verification, credible
    recurrence, and durable reuse. Size and difficulty do not qualify it. Reject one-offs and speculative value. Allow
    at most one two-sentence `$task-handoff` suggestion and stay silent otherwise.

15. Completion rules cover success verification, dependent gating on failure, and removal of duplicates from the
    changed-files union. They cover ordered/scoped polish invocation and polish skip/failure conditions, including an
    explicit hurry or wrap-up request. Fix same-pattern sites covered by the outcome before reporting. Preserve
    cross-repository `$commit` behavior and watch CI on pushed commits before the completion report.
16. Adapters implement the shared prompt/result/failure/completion contracts without weakening them. The shared
    entrypoint loads exactly one adapter.
17. When a task names another skill, that companion defines the work and the handoff owns delegation mechanics. The
    parent runs the companion's discovery, judgment, and planning phases. It incorporates them into the plan. The
    exception is audit-heavy discovery, which is mapped and divided for implementation agents. If a companion is absent
    from the skill list, read it directly from the host skill root.

    The handoff contract takes precedence over overlapping companion mechanics. Companion user-decision gates remain
    binding. Agents never load skills by name. Briefs include the needed companion excerpts inline. Companion-required
    polish maps onto the Plan Phase passes and runs once. Completion satisfies both report contracts.

    The `Companion skills:` plan line is optional. Place it after `Research:`.

These topics are out of scope unless the request explicitly names them:

- Host selection, launch, and continuation mechanics.
- Research mechanics.
- Claude-adapter-only content and Codex-adapter-only content.
- Each skill's model defaults and failed-agent re-run rules. Model defaults are intentionally different. `codex-handoff`
  adapters choose GPT tiers. `claude-handoff` uses `sonnet` or `opus`. Never normalize those defaults.
- Status reporting style.
- Frontmatter and `references/`/`scripts/` contents.

Verify by comparing prose in the in-scope blocks. There is no extractable helper data.

### Ai-skillet CLI consumers

Files:

- `skills/skill-harmonization/SKILL.md`
- `skills/skill-writing/SKILL.md`
- `skills/skill-writing/references/writing-great-skills.md`

Wherever a consumer declares an ai-skillet minimum version, keep it at `1.0.0+`. Each consumer must invoke its
ai-skillet subcommand directly:

- `skill-harmonization` uses `map`, plus optional `doctor` evidence.
- `skill-writing` uses `doctor`.

Do not use a retired Python, uv, ripgrep, helper-resolution, wrapper, or fallback path. `toolkit/skillet/` and the
workspace version in `toolkit/Cargo.toml` are the producer whose contract the `1.0.0+` minimum tracks.

`toolkit/skillet/AGENTS.md` is authoritative for the extended-dialect contract. Keep the `Frontmatter Dialect` section
of `writing-great-skills.md` a faithful summary of its field union and cross-field rules. Exit codes, the `--fix-safe`
boundary, and map's default exclusions live in `ai-skillet <command> --help`. Consumers rely on that output instead of
restating it. No catalog skill wraps ai-skillet by itself.

## Workflow

1. Verify repository context: `git rev-parse --git-dir`. If this fails, stop. In that case, tell the user to run from a
   git repository.
2. Resolve selected sync groups once. Do not broaden the group list after reading files unless the user asks.
3. Read the selected files. Compare only the in-scope shared blocks or workflow contracts.
4. When drift exists, normalize all copies to one phrasing or value set. Reuse the clearest wording already present.
5. Prefer minimal patches. Do not rewrite whole sections just to make them symmetrical if the remaining differences are
   skill-specific.
6. If no drift exists, make no edits. In that case, report that the selected groups are already aligned.

## Verification

When the Supported Chat Hosts group is selected, extract and compare its exact guard blocks from the repo root:

```bash
bash <<'EOF'
extract_guard() {
  awk '
    $0 == "## Supported Chat Hosts" { in_block = 1; print; next }
    in_block && NF { in_guard = 1; print; next }
    in_guard { exit }
  ' "$1"
}

reference='skills/agents-docs/SKILL.md'
rc=0
if [ -z "$(extract_guard "$reference")" ]; then
  echo "missing Supported Chat Hosts guard: $reference" >&2
  rc=1
fi
for skill_file in 'skills/agents-introspection/SKILL.md' 'skills/copy-transcript-path/SKILL.md'; do
  diff -u --label "$reference" --label "$skill_file" \
    <(extract_guard "$reference") <(extract_guard "$skill_file") || rc=1
done
exit "$rc"
EOF
```

After editing Markdown, run from the repo root:

```bash
just prettier-write <changed files>
just prettier-check <changed files>
```

If `prettier-check` fails, fix only the files you changed.

Re-read touched sections. Confirm selected groups now match on shared wording or workflow contracts. Confirm they still
differ only where their workflows require it.

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

### Orchestration Contract and Adapters

Files:

- `skills/orchestration/SKILL.md`
- `skills/orchestration/references/native-claude.md`
- `skills/orchestration/references/native-codex.md`
- `skills/orchestration/references/claude-to-codex.md`
- `skills/orchestration/references/codex-to-claude.md`
- `skills/orchestration/references/jev-routing.md`

`SKILL.md` owns the shared contract. Adapters contain only runtime-specific mechanics. Review them against the shared
contract instead of maintaining duplicate entrypoints.

Keep these decisions aligned:

1. Claude Code defaults to Claude workers. Codex and every other harness default to Codex workers. An explicit agent or
   model choice overrides that default within the user's stated scope. Host detection does not change the requested
   worker family.
2. The parent owns decisions and planning. Research returns evidence without edits or plans. Implementation follows the
   finalized plan. Requests authorize launch without another routine plan approval, subject to host restrictions.
3. The smallest effective team has at most three research agents and eight implementation agents. IDs, dependencies,
   disjoint scopes, and one aggregate-validation owner remain stable across waves and follow-on work.
4. The parent owns coordination, scope expansion, reconciliation, polish, and commits. Workers never run coordination
   lifecycle commands or expand their own write scope. The parent requires `READY` for delegated Git-worktree writes.
5. Prompts and results follow the shared fields. A progress report, launch acknowledgement, or quiet period does not
   prove completion. Attribute failures before gating dependents. Apply only the selected route's retry mechanism.
6. Preserve the shared companion-skill, proportional-verification, hurry, skill-maintenance, and completion contracts.
   Runtime differences cannot weaken them.
7. Keep Jev candidates consistent with each adapter's supported model and effort pairs. Preserve explicit user choices,
   native Claude effort limits, and local selection on routing failure, uncertainty, or parent rejection.

Model tiers, permissions, research toolsets, progress transport, session identity, and continuation mechanics differ by
route. Preserve those differences. Native Claude Explore is one-shot. Native Codex uses native thread tools. The two CLI
routes also serve other harnesses. Never copy CLI fallback or permission settings into a native route.

Verify all six host/worker combinations against the routing table. Also review an explicit model, a scoped preference,
an unavailable requested agent, a harness without shell execution, research-only work, Plan Mode, and infrastructure
failure. An unknown harness selects Codex before prerequisite checks. There is no generated helper data for this group.

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

### Model Identifiers

Files:

- `skills/agents-brain/SKILL.md`
- `skills/agents-brain/scripts/fetch-guidance.sh`
- `skills/agents-docs/SKILL.md`
- `skills/node-deps-bumper/SKILL.md`
- `skills/orchestration/SKILL.md`
- `skills/orchestration/references/claude-to-codex.md`
- `skills/orchestration/references/codex-to-claude.md`
- `skills/orchestration/references/jev-routing.md`
- `skills/orchestration/references/native-claude.md`
- `skills/orchestration/references/native-codex.md`
- `skills/orchestration/scripts/run-codex-agent.sh`
- `skills/orchestration/scripts/select-model.py`
- `skills/release-bumper/SKILL.md`
- `skills/skill-writing/SKILL.md`
- `skills/todo-archive/SKILL.md`
- `skills/yeet/references/posting.md`

When a model is released, renamed, or retired, run `just model-refs` and update every listed file in one change. Keep a
mention unchanged when the change does not affect its model. The script output is authoritative for current mentions.
When the script reports a file that this list omits, add the file to this list in the same change. When a listed file no
longer has a mention, remove it from this list. Model aliases in skill frontmatter, such as `model: sonnet`, are in
scope.

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

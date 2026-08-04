---
name: sync-skills
description:
  Review and synchronize coupled skill files; align shared wording and helper contracts, fix drift, preserve
  skill-specific content.
---

# Sync Skills

Review skill files that intentionally share wording, policies, or helper contracts. Patch only real drift; preserve
skill-specific behavior and examples.

## Scope

Default: run every sync group below. If the request names a group, file, or subset, run only that group.

Work only in the files listed for the selected groups.

## Sync Groups

### Commit Message Format Helpers

Files:

- `skills/commit/scripts/prepare-commit.sh`
- `skills/commit/scripts/select-message-format.sh`

Treat these as in scope:

- The ordered literal entries in each `always_natural_language_repos` array. The variable names may differ, but the repo
  path list must stay identical.
- The rule that `--natural` forces `natural`.
- The default fallback to `conventional` when the target repo is not in the always-natural list.

Treat these as out of scope unless the request explicitly names them:

- Atomic staging behavior in `prepare-commit.sh`.
- Backwards-compatible argument parsing in `select-message-format.sh`.
- Adding a shared sourced helper file.

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
  no-redesign rule for implementation agents; the smallest-effective-team and five-implementation-agent limits; and
  parent implementation work being limited to orchestration, integrity checks, failure handling, and the conditional
  polish pass.
- Pre-plan research delegation: trigger it for uncertain scope, multiple or unfamiliar subsystems, or materially slower
  serial evidence gathering; keep zero research agents as the default; let the parent alone decide whether it runs
  without asking the user; require research agents to stay read-only, gather evidence, and return findings rather than
  decisions or plans; limit the separate research budget to three agents (`R1` through `R3`); feed findings into the
  final plan; and include the optional plan `Research:` traceability line.
- Strategy selection guidance: sequential vs parallel vs hybrid criteria, disjoint-write-scope requirement, wave
  semantics, the slowest-agent note, and the whole-handoff five-implementation-agent limit with stable IDs and
  dependencies.
- The single-validation-owner rule: aggregate checks run once; every other agent runs only the narrowest checks proving
  its own edits; aggregate-check failures confined to files outside every agent's scope are attributed to unrelated
  concurrent work, not treated as blockers.
- The `$code-polish` trigger list, including "file count alone is not a trigger".
- Before-launch session-claim guidance: delegated agents treat the orchestrating session's presence as authorization for
  their assigned work. The exact claim owner is host-specific.
- Platform-agnostic agent prompt requirements: outcome plus brief, exact write scope and dirty-work boundaries,
  validation assignment, authority boundary, delegation context, stopping rule, and reporting requirement.
- The structured result-field contract: status, summary, changed files, verification with each command and outcome,
  residual risks, and blockers.
- Failure classification: a returned blocked status is a plan problem that gates dependents and requires user decision;
  an evidenced tool or infrastructure failure permits exactly one same-agent continuation after inspecting partial
  edits, and a second infrastructure failure blocks. The continuation mechanics are platform-specific.
- Completion rules: success verification, dependent gating on failure, changed-files union dedupe, polish invocation and
  skip conditions, and cross-repository `$commit` behavior.
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
- Each skill's model selection table and its rules about escalating or re-running a failed agent on another model. Both
  skills have selection tables, but they are intentionally different — the codex-handoff adapters choose GPT-5.6 model
  and effort tiers, while Claude selects among three aliases (`haiku`, `sonnet`, and `opus`) — so never normalize them.
- Status reporting: the Claude adapter's dashboard system versus both native hosts' concise progress and completion
  reports.
- Frontmatter and `references/`/`scripts/` contents.

Verification is prose comparison of the in-scope blocks; there is no extractable helper data.

## Workflow

1. Verify repository context: `git rev-parse --git-dir`. If this fails, stop and tell the user to run from a git
   repository.
2. Resolve selected sync groups once. Do not broaden the group list after reading files unless the user asks.
3. Read the selected files and compare only the in-scope shared blocks or helper contracts.
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

For the Commit Message Format Helpers group, run:

```bash
bash -n skills/commit/scripts/prepare-commit.sh skills/commit/scripts/select-message-format.sh
bash <<'EOF'
set -euo pipefail
extract_repos() {
  awk '
    /always_natural_language_repos=\(/ { in_list=1; next }
    in_list && /^[[:space:]]*\)/ { exit }
    in_list {
      line=$0
      sub(/^[[:space:]]*"/, "", line)
      sub(/"[[:space:]]*$/, "", line)
      if (line != "") print line
    }
  ' "$1"
}
diff -u \
  <(extract_repos skills/commit/scripts/prepare-commit.sh) \
  <(extract_repos skills/commit/scripts/select-message-format.sh)
EOF
```

Re-read touched sections and confirm selected groups now match on shared wording or helper data and still differ only
where their workflows require it.

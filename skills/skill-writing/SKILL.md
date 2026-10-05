---
argument-hint: "[skill-name]"
compatibility:
  Requires curl and a writable user cache directory. Network access populates or refreshes the agentskills.io
  specification.
name: skill-writing
description:
  Create, scaffold, or initialize a project-local agent skill under `.agents/skills` in an ordinary repository. Defer to
  repository instructions that define a source catalog and lifecycle.
---

# Skill Writing

Create a project-local skill in `.agents/skills/`. Expose it to Claude Code through a relative symlink. Verify the
result with the repository's canonical skill validator.

## Model Guidance

Optimize every new skill and its content for GPT-6.1 Sol and Claude Opus 5.5. The summaries below remind you of the live
guidance. They do not replace the guides. Read both guides before designing or writing a complex, long-running,
multi-tool, or orchestration-heavy skill. Their recommendations may change.

- [GPT-6.1 Sol prompting guidance](https://developers.openai.com/api/docs/guides/latest-model#prompting-best-practices)
  (shared GPT-6 guide): Evaluate its family-wide recommendations on Sol. Complete authorized work under stated
  assumptions. State that user instructions take precedence over skills. Specify writing and delegation preferences.
  Keep verification proportional to the change.
- [Claude Opus 5.5 prompting guidance](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5-5):
  Calibrate effort instead of prompting for more thinking. Never ask for reasoning in response text. Name the premature
  stops to avoid and the stops that are wanted. Treat text-only turns as reports, not completion.

  Request brief progress updates. Explore relevant sources before acting. Name concrete patterns to avoid instead of
  giving generic style advice.

## STE Authoring and Review

Before writing or changing skill prose, read [references/asd-ste100.md](references/asd-ste100.md) completely. Apply its
STE-based profile to descriptions, instructions, references, examples, and agent-facing metadata descriptions. Complete
its meaning and style review before reporting completion. Preserve obligation strength and protected technical content.
The profile does not establish official ASD dictionary compliance.

## Input

- **skill-name** (optional): A kebab-case name such as `my-skill`. If omitted, derive the shortest unambiguous
  kebab-case domain name for the skill's purpose. Do not repeat the repository or app name as context, for example
  `price-estimator`, not `budget-price-estimator`. If you derive a name, state it in the completion report. Stop only
  when a supplied name is invalid or collides with an existing skill. See the collision check in step 2.

Reject `--global`, explicit destination paths, and other scope overrides. The invocation working directory is the only
supported scope.

## Workflow

### 1. Apply the Repository Catalog Guard

Before resolving project-local paths, read the repository instructions applicable to the invocation working directory.
If they define a skill source catalog and lifecycle, stop this workflow. Follow that repository-owned workflow. Do not
create `.agents/skills/` or `.claude/skills/` paths there.

### 2. Resolve and Validate the Local Scope

Set `<scope>` to the working directory where the skill was invoked. Create the source at
`<scope>/.agents/skills/<name>/`. Create the Claude Code symlink at `<scope>/.claude/skills/<name>`. When invoked from a
nested project or workspace, do not redirect the scope to the repository root. Never create or modify a skill under
`~/.agents`, `~/.claude`, `~/.codex`, or another global installation directory.

The symlink target is always the relative path `../../.agents/skills/<name>`.

Before writing, stop if either the source directory or symlink path already exists.

### 3. Read the Current Format Sources

Resolve `scripts/fetch-agentskills-spec.sh` relative to this skill directory. Run
`scripts/fetch-agentskills-spec.sh [--refresh]` once. Read the returned file completely. The helper reuses an
integrity-valid specification for 24 hours and conditionally revalidates older entries. If live retrieval fails, it may
return a cache validated within seven days. If the default user cache location is unavailable or unwritable, set
`AGENTSKILLS_CACHE_DIR`.

Use `--refresh` for explicitly latest or change-sensitive work, disputed portable-format guidance, or a conflict with
validator behavior. Read a `stale` result before using it. Disclose its validation timestamp and retrieval failure in
the completion report. If the helper cannot return a valid file, stop before writing. Never fetch the agentskills.io
specification directly or create its cache in a repository or skill installation.

Fetch the current [Claude Code frontmatter reference](https://code.claude.com/docs/en/skills#frontmatter-reference) with
`WebFetch`. Confirm field shapes, naming rules, and progressive-disclosure conventions from both sources. Do not guess
because the formats change.

### 4. Define the Contract and Layout

Read [references/writing-great-skills.md](references/writing-great-skills.md) completely before choosing the contract or
layout. It defines the authoring principles, content-routing thresholds, helper runtime defaults, and communication
contract.

Define the contract. Identify every skill the workflow requires, invokes, or transfers work to on any supported branch.
Suggestions, examples, related-skill references, and underlying tool capabilities are not dependencies. Create only the
directories justified by the selected layout. `SKILL.md` and `agents/openai.yaml` are always required.

### 5. Create the Skill

```bash
mkdir -p "<scope>/.agents/skills/<name>/agents"
# Add only the subdirectories the layout calls for:
# mkdir -p "<scope>/.agents/skills/<name>/scripts"
# mkdir -p "<scope>/.agents/skills/<name>/references"
```

Write `<scope>/.agents/skills/<name>/SKILL.md` with:

- Frontmatter sorted alphabetically, with `description` last. Put discovery-time trigger phrases first in `description`.
- A `skill-dependencies` array when routing identified dependencies. Use bare names for skills in the same repository
  and `ORG/REPO#SKILL` for external skills. Sort by the target skill name (the bare name or substring after `#`), then
  by the complete identifier.

  Require unique strings. Resolve every bare dependency in the same repository. Exclude the owning skill as a bare
  dependency. The validator does not check external repository existence. Omit the field when no dependencies exist.

- A short `# Title`.
- A one-line summary of what the skill does.
- Add `disable-model-invocation: true` or `user-invocable: false` only when the skill differs from Claude's defaults.
  Omit `disable-model-invocation: false` and `user-invocable: true` because absence already expresses those values.
- Set `coordination: exempt` only when the skill's declared default workflow writes no repository files or only
  repository metadata. When selected, add this ordinary prose declaration to the new skill's body. The fence below
  documents the declaration for this authoring skill. It is not this skill's own declaration:

  ```text
  This skill is coordination-exempt: skip the ai-coord gate for its declared work.
  ```

  Explicitly authorized escalation beyond the declared behavior re-enters the gate.

- `## Arguments` (if any) and a lean imperative workflow. Use fixed steps only when order matters. Otherwise, state the
  contract and let repository evidence guide execution.
- Explicit links to every `references/` file the workflow may need, each with a one-line note describing _when_ to read
  it.
- CLI signatures for every bundled helper, including arguments, output, defaults, and the runtime command, so an agent
  can invoke it without reading its source.

Use imperative prose. Resolve bundled `references/`, `scripts/`, `examples/`, and `assets/` paths relative to the owning
skill directory. Do not add repository-style support files or authoring artifacts that runtime agents will not use.
Quote YAML plain scalars containing a colon followed by a space. Otherwise, the frontmatter parser may reject them.

Write `<scope>/.agents/skills/<name>/agents/openai.yaml` with:

```yaml
policy:
  allow_implicit_invocation: true
```

Set `allow_implicit_invocation` to the inverse of `SKILL.md` `disable-model-invocation`. If later adding Codex UI
metadata or MCP/tool dependencies, merge them into the same file. Keep the policy.

### 6. Create the Claude Code Symlink

Always create a relative symlink so Claude Code discovers the skill from its own discovery path:

```bash
mkdir -p "<scope>/.claude/skills"
ln -s "../../.agents/skills/<name>" "<scope>/.claude/skills/<name>"
```

### 7. Verify and Report

- Patch tooling creates files at mode 0644. Before the first verification run, `chmod 755` every executable under
  `scripts/` and `tests/`. A scaffolded test that fails its first run with `Permission denied (os error 13)` has this
  mode issue.
- `test -f "<scope>/.agents/skills/<name>/SKILL.md"`
- `test -f "<scope>/.agents/skills/<name>/agents/openai.yaml"`
- `readlink "<scope>/.claude/skills/<name>"` equals `../../.agents/skills/<name>`, and the link resolves to the source
  directory.
- `test -x` every `scripts/*` and `tests/*` executable so a missed `chmod` fails loudly instead of surfacing later as a
  permission error.
- `ai-skillet doctor --root "<scope>/.agents/skills/<name>"` exits 0. This is the canonical local schema and policy
  gate.
- Complete the meaning and style review in `references/asd-ste100.md` for every authored or changed prose target.
  Compare existing prose with its source and new prose with the authoritative contract. Review protected content and
  precision exceptions explicitly.
- Finish with `### 🧩 Skill created: <name>`, a tree of created paths, and `### ✅ Verified` with the exact checks. Link
  files by their absolute `.agents/skills/<name>/` source paths, never through the `.claude/skills/<name>` symlink.
- Commit when the request or standing instructions authorize it. Otherwise, offer to commit. Do not ask again for
  authorization already given.

Keep helper stdout, commands, paths, frontmatter, and generated skill content undecorated unless the new skill's output
contract requires otherwise.

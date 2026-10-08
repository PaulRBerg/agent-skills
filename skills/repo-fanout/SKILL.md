---
argument-hint: "<task-or-$skill> [--repo <path>]... [--repos-file <file>]"
compatibility:
  Requires Claude Code or Codex CLI, Git, and the ai-coord CLI. Delegation also requires the host and worker
  prerequisites of the orchestration skill.
disable-model-invocation: true
name: repo-fanout
skill-dependencies:
  - commit
  - orchestration
description:
  Run one task or skill across many repositories with per-repository agents, coordination claims, validation, commits,
  and one results table. Use when the same task must run in several repositories.
---

# Repo Fan-out

If a slash or dollar invocation already placed these instructions in the conversation, follow them directly. In that
case, do not invoke this skill again through a skill tool.

Run one task in each repository of a supplied set. The `$orchestration` skill owns the delegation mechanics. This skill
adds the repository set, the per-repository phases, the claims, the commits, and the report.

Success means that every supplied repository has a final status in the report. Each changed repository has its own
verification evidence and its own commit.

## Input

`$ARGUMENTS` contains the task and the repository options.

- Task: all text that is not a repository option. The task is free text, or a `$skill` name with its arguments.
- `--repo <path>`: one repository. Repeat the option for more repositories.
- `--repos-file <file>`: a file with one absolute or `~` path on each line. Ignore blank lines. Treat text from `#` to
  the end of the line as a comment.

Combine the paths from both options in the order given. If the task text is empty, ask the user for the task. If no path
is supplied, ask the user for the repositories.

Validate each path before other work:

1. Expand a leading `~` to the home directory.
2. Run `git -C '<path>' rev-parse --show-toplevel`.
3. Accept the path only when the command succeeds and its output is the same directory as the path. Resolve symbolic
   links in both before you compare them.
4. Mark a path as a duplicate when it resolves to a repository that you already accepted.

Give each rejected path one named obstacle: `missing path`, `not a Git repository`, `not a repository root`, or
`duplicate`. Continue with the remaining repositories. If no repository is valid, stop and send the blocked report.

For each accepted repository, record the output of `git -C '<root>' status --short`. Treat that output as the work of
other agents. Preserve those files byte-for-byte.

When the task names a `$skill`, read the `SKILL.md` of that skill before classification. If the host skill list does not
show the skill, read it from the host skill root. The root is `~/.claude/skills/<name>/` in Claude Code and
`~/.agents/skills/<name>/` in Codex. If you cannot find the skill, stop and report the missing skill. Apply the
`$orchestration` Companion Skills rules to that skill.

## Classify

Classify the task with the `$orchestration` contract:

- Research-only: the requested outcome is findings, evidence, or an assessment. The task requests no repository changes
  and no plan.
- Implementation: every other task.

Apply one classification to all repositories. When the task names a `$skill`, classify from the outcome that the skill
declares.

An explicit user choice of agent, model, or effort applies to every delegate in every batch. Without such a choice, use
the `$orchestration` worker defaults.

## Research-Only Fan-out

1. Split the accepted repositories into batches of at most three repositories.
2. Run each batch as one research-only `$orchestration` handoff.
3. Give each repository in the batch one read-only research agent, `R1` to `R3`.
4. In each prompt, include the task, the repository root, the dirty-file snapshot, and the strict read-only boundary.
5. Start the next batch only after every agent in the current batch settles.
6. Consolidate the findings for each repository.

Acquire no claims. Make no edits and no commits. Finish with the research report in [Report](#report).

## Implementation Fan-out

Split the accepted repositories into batches of at most eight repositories. Run each batch as one `$orchestration`
implementation handoff. Run the batches one after another. Each batch has three phases. Then the parent reconciles,
commits, and releases the claim.

### Phase 1: Discovery

Launch read-only discovery agents with the `$orchestration` research mechanism. Use at most three agents, `R1` to `R3`.
When the batch has three repositories or fewer, give each agent one repository. Otherwise, give each agent at most three
repositories.

Each discovery agent returns these facts for each repository:

- The exact write targets for the task: the files and subdirectories to create or edit.
- The validation commands of the repository, from its `AGENTS.md`, `CLAUDE.md`, `justfile`, or package manifest.
- The repository constraints that the implementation agent must obey.
- Whether the task applies. A repository where the task needs no change gets the status `no change`.
- Blockers, with evidence.

Discovery agents edit nothing and return no plan.

### Phase 2: Plan and Claim

1. Build the `$orchestration` plan for the batch from the discovery results. Assign one implementation agent to each
   repository that needs a change.
2. Use the discovered write targets of each repository, as absolute paths, as that agent's manifest write scope.
3. When a discovery blocker has a fix inside the repository, add the fix to the brief of that repository.
4. Remove a repository from the batch only when its blocker needs user-owned input, an action outside the repository, or
   a confirmation. Report that repository as `blocked`.
5. Record the union of the write scopes as the bundle draft that `$orchestration` requires.
6. Promote the draft with `ai-coord bundle start --draft <plan-slug>`.

If promotion reports `no draft named ...`, claim the same union with the explicit command:

```bash
ai-coord bundle start '<label>' '<absolute-path>'... --recursive '<absolute-dir>'...
```

Name each file as a leaf. Use `--recursive` only for a subdirectory, never for a repository root.

When the batch has only one repository, use the single-root commands from that repository instead. Record the draft with
`ai-coord draft --name <plan-slug>`. Promote it with `ai-coord start --draft <plan-slug>`. The fallback is
`ai-coord start '<label>' '<path>'...`.

Require `READY` before you launch implementation agents. If the claim queues or blocks, run `ai-coord wait`. On each
wake, submit the same claim command again. Do not end the turn to wait.

### Phase 3: Implementation Wave

Launch one implementation agent for each repository in one parallel wave. A batch has at most eight implementation
agents. Build each brief with the `$orchestration` Implementation Prompt Contract. Include these items in each brief:

- The task text, inlined. Agents cannot load skills. When the task names a `$skill`, inline the instructions of that
  skill that apply to the brief. Do not tell the agent to use the skill by name.
- The repository root, the constraints from discovery, and the dirty-file snapshot.
- The exact write scope from the claim.
- The validation commands of the repository, as the scoped validation of the agent.
- The authority boundary. The agent must not commit, push, or run coordination lifecycle commands.

The parent owns any check that spans more than one repository. Run each such check once, after the wave.

### Reconcile, Commit, and Release

After the wave, do these steps for each repository:

1. Reconcile the agent result with the working tree of the repository and its claimed write scope. Use the
   `$orchestration` reconciliation rules.
2. Fix a failure or a gap inside the requested outcome with `$orchestration` follow-on work before you commit.
3. Run each polish pass that `$orchestration` requires.
4. Run `$commit` from the repository directory. Pass only the files that this batch changed in that repository.
5. Push when the request or standing user instructions authorize it.

Do not commit a blocked, incomplete, or out-of-scope change. When every repository in the batch has a commit or a final
status, run `ai-coord done` to release the claim. Then start the next batch.

## Stops

Do not stop at these points:

- After the discovery wave.
- To ask for plan approval. The fan-out request authorizes the plan.
- To report a fixable item instead of fixing it.
- After one batch when more batches remain.

Stop only for missing task text, no valid repository, user-owned input, or a confirmation boundary. A confirmation
boundary includes a destructive action, a purchase, a deployment, and an external write that no instruction authorizes.
A progress update is not a stop.

## Report

`<total>` counts every supplied repository except duplicates. `<n>` counts the repositories with the status `completed`
or `no change`.

For an implementation fan-out, use this heading when `<n>` equals `<total>`:

```markdown
### ✅ Fan-out completed — <n>/<total> repositories
```

Otherwise, use this heading:

```markdown
### ⛔ Fan-out blocked — <n>/<total> repositories completed
```

Then show one table with one row for each supplied path:

| Repository | Status | Changed files | Verification | Commit | Blockers |
| ---------- | ------ | ------------- | ------------ | ------ | -------- |

- Repository: the repository root, with `~` for the home directory.
- Status: `completed`, `no change`, `blocked`, `rejected`, or `duplicate`.
- Changed files: the paths relative to the repository root, or `none`.
- Verification: each command and its outcome, or `none`.
- Commit: the short hash and the push state, or `none`.
- Blockers: the named obstacle, or `none`.

After the table, add one line for each batch. Give the `$orchestration` strategy, the agent count, and the worker
configuration.

Then add `Issues and caveats` when items exist. Group them as `Resolved` and `Open`. Omit an empty group. An `Open` item
may cite only user-owned input, an action outside the repository, or a confirmation boundary. Fix every other item
before the report.

For a research-only fan-out, use the heading `### 🔎 Fan-out research — <n>/<total> repositories`. Show one table with
the columns repository, status, key findings, and blockers. Then add one section for each repository with its findings,
evidence, and open questions.

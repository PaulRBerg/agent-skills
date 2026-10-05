# Maintain Repo Skills

Use task evidence to remove obsolete skills, merge overlapping skills, and capture proven reusable workflows. Success
means every selected lifecycle change preserves needed guidance, updates its local consumers, and passes the
repository's validation. Keep the work proportional to the opportunity discovered during the task.

## Bound the Review

Before following any skill path, set `repo_root` to the repository where the task was given. Honor a narrower requested
subtree and applicable repository instructions. Select only repo-owned skills whose canonical source lies inside that
boundary. These sources are project-local `.agents/skills/<name>/` or a repository-declared source catalog.

Exclude global installations, other repositories, submodules, vendored or generated copies, and symlinks resolving
outside the boundary. A local copy does not give you ownership of a third-party skill. Never redirect a repair to an
excluded source repository.

Start with skills used by the task or referring to changed tools, commands, architecture, or workflows. Inspect their
bundled files and local callers only as needed to establish impact. Do not audit unrelated skills. When using
`ai-skillet map`, read `ai-skillet map --help` first and pass `--root <repo_root>` plus candidate `--skill <name>`
filters. Never use its default home scan or `--portfolio-root` here.

Exclude out-of-bound results before reading them. When the CLI is unavailable, use bounded repository searches that
preserve the same exclusions.

Record the concrete evidence, affected skills, and intended action before editing. Use the repository's existing finding
and coordination process when present. Read-only requests, Plan Mode, and `--dry-run` produce candidates and validation
plans only. For authorized maintenance, perform the selected action rather than merely suggest it. Retain required
destructive-action approvals. When existing explicit or standing authorization covers the same action, do not ask again.

## Choose the Smallest Useful Change

One verified task can establish obsolescence or duplication. Creation requires more evidence. Require demonstrated
recurring use and a distinct contract, not just a difficult or lengthy task.

| Action | Required evidence                                                                                                                                                                                                                                                                         | Do not infer it from                                                                    |
| ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Delete | The task removed or replaced the skill's subject, or all useful guidance is already covered by a verified surviving skill or repository mechanism. No supported caller still needs unique behavior.                                                                                       | Age, lack of recent invocations, missing text references alone, or personal preference. |
| Merge  | Two skills serve the same trigger or tightly coupled workflow. One coherent entrypoint reduces duplicated decisions while preserving every still-supported outcome and safety boundary.                                                                                                   | Shared vocabulary or a few common paragraphs across otherwise independent capabilities. |
| Create | Current repository workflows, callers, or task/history evidence show at least two concrete recurring uses. A stable multi-step procedure or non-obvious constraint materially improves future execution. No existing skill, focused reference, or short AGENTS.md rule covers it cleanly. | A one-off fix, speculative future use, a task transcript, or complexity alone.          |

Prefer updating an existing skill or placing a short rule in existing context when that solves the evidenced need. Leave
useful independent skills separate. If evidence is insufficient, keep the existing skills. In that case, omit
speculative creation. Ask only when missing user-owned requirements prevent a required decision.

## Apply the Lifecycle Change

Follow a repository-owned catalog lifecycle when one exists. Its protected contracts and publication requirements take
precedence over the mechanics below, but this workflow never authorizes writes outside `repo_root`. If completion
requires external installation or publication, finish the local work and report the excluded step for a separately
authorized workflow.

- **Delete:** Inspect the complete skill and its inbound references first. Preserve still-useful unique guidance in the
  appropriate surviving local skill or context file. Then remove the obsolete skill and its exclusively owned bundled
  files. Remove local discovery links and wrappers. Update local dependency declarations, indexes, and instructions.

  Do not delete shared helpers or user data. Confirm remaining callers have a valid replacement or no longer apply.

- **Merge:** Choose an existing survivor when its name and trigger fit. Map each supported outcome, constraint,
  reference, and helper to the combined skill. Reconcile contradictions from current repository evidence. Consolidate
  its description and routing. Migrate needed bundled files without name collisions. Before removing the absorbed skill,
  update callers, metadata, dependencies, and local discovery links.

  Preserve self-containment and distinct approval boundaries. If the merge needs a new skill identity, use
  `$skill-writing` for that creation as below. The existing supported uses establish reuse. It need not clear a separate
  speculative-new-capability bar.

- **Create:** Invoke `$skill-writing` with the chosen name from the intended in-repository project directory. Honor its
  catalog guard, collision check, current format sources, metadata, symlink, and validation requirements. When it routes
  to the repository's source-catalog lifecycle, follow that route instead of creating `.agents/skills` there. Capture
  only the reusable procedure, triggers, constraints, and validation. Omit session narrative and incidental details.

Use `references/maintain.md` from this skill for ordinary context edits. Keep unrelated source changes out of scope.
Re-read current files before applying edits. Preserve concurrent work. Complete independently actionable candidates even
if another is blocked.

## Verify and Finish

Run the repository's scoped formatting and skill checks. Verify every changed link, dependency, wrapper, and discovery
path. Confirm affected surviving skills resolve within the allowed boundary. Search local consumers for retired names
and paths. Classify legitimate historical mentions instead of replacing them indiscriminately. For a merge, account for
every previously supported outcome.

When helpers or their usage changed, run affected helper tests. For a new skill, use `$skill-writing`'s completion
checks.

Commit through the repository's authorized workflow. Report deleted names, merge mappings, created names, the evidence
for each decision, exact checks and outcomes, and any concrete blocker. When this is a pass within another task, fold
its material results into that task's final report. A no-op needs no separate report. Stop when the task-backed
candidates are settled, without expanding into a fresh audit.

## Establish the Standing Instruction

When asked to enable continuous maintenance for future tasks, add or reconcile a concise rule in the applicable
repository AGENTS.md through `maintain`. Keep it self-contained so future agents can act without loading this skill on
every task. Adapt this text to local ownership and lifecycle conventions. Retain the scope and asymmetric bar:

> During implementation tasks, review repo-owned skills affected by the work. Keep all review and edits inside the
> repository where the task was given; exclude global skills, other repositories, externally owned copies, and symlinks
> resolving outside it. Delete skills made obsolete or wholly redundant by verified task evidence. Merge skills when one
> coherent workflow preserves their supported outcomes with less duplication. Create a skill through `$skill-writing`
> only with at least two concrete recurring uses, a stable reusable procedure, and no suitable existing skill or short
> context rule; honor its repository catalog guard. This is standing authorization for those local lifecycle changes,
> subject to protected contracts and required approvals. Preserve needed guidance, update local callers and discovery
> paths, and validate each change. Finish fixed-scope work before independent maintenance; keep read-only tasks and Plan
> Mode read-only. Review opportunities encountered during the task, without unrelated audits or background jobs.

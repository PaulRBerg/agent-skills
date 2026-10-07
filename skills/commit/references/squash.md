# Squash Mode

Replace every branch commit after the merge base with one commit that carries the net branch diff. The helper owns the
plan, reset, and rollback mechanics. `ai-commit` owns the commit. `SKILL.md` owns the message semantics.

Resolve `<skill-dir>` from the `SKILL.md` that loaded this reference. Write the plan JSON to a scratch path outside the
repository so the tree stays clean.

## 1. Preconditions

The helper enforces these preconditions:

- The working tree and the index are clean.
- HEAD is attached to a branch other than the resolved default branch.
- The branch has at least one commit after the merge base.

Without `--base`, the helper resolves `origin/HEAD`, then local or remote `main`, `master`, or `trunk`.

## 2. Plan

```sh
uv run "<skill-dir>/scripts/git-squash.py" plan [--cwd <repo>] [--base <branch>] > <plan.json>
```

`plan` is read-only. It verifies the Git worktree, the attached branch, the clean tree and index, the resolved base, the
non-default current branch, the merge base, the positive ahead count, and the remote facts. It prints
`schemaVersion: 1`, the original HEAD, the merge base, the base ref, the commits in chronological order, the unique
authors, the tree and remote state, and the rollback facts. A failed precondition exits without changing history.

Before mutation, show the branch, base ref, merge base, commits replaced, tree state, remote state, and rollback HEAD in
a compact plain table.

## 3. Reset

```sh
uv run "<skill-dir>/scripts/git-squash.py" reset --plan <plan.json>
```

Immediately before mutation, `reset` revalidates the plan: the original HEAD, the branch, the clean tree and index, and
the merge base. A stale plan fails without changing history. Then `reset` soft-resets the branch to the merge base,
which leaves the net diff staged. If the staged diff is empty, `reset` restores the original HEAD and index and fails
with `squash would produce an empty commit`.

On success, `reset` prints `status: "reset"` with `branch`, `originalHead`, `mergeBase`, `commitsReplaced`, `authors`,
and `rollback`. Keep the plan file until the commit completes.

## 4. Prepare and Compose

Run step 2 of `SKILL.md` as:

```bash
ai-commit prepare --staged --diff full
```

Add `--natural` or `--conventional` only as an explicit override. Then follow step 3 of `SKILL.md` with these additions:

- The prepared net diff is authoritative. The plan's commit subjects supply intent only. Do not describe intermediate
  states that the net diff does not contain.
- Compose the message from the printed message-format rules. Describe the surviving outcome, not the squash operation.
- Add one `Co-authored-by: Name <email>` trailer for each plan author other than the current Git user. Read that user
  with `git config user.name` and `git config user.email`. Put these trailers in the final trailer paragraph.
- A quoted positional subject override from step 1 still applies.
- Keep the `Agent-Session:` trailer handling from step 3.

## 5. Commit

Run step 4 of `SKILL.md` without `--push`:

```bash
ai-commit commit <transaction-id> -m '<subject>' [-m '<body>'] [-m '<trailers>']
```

After the squash, the branch diverges from any upstream copy, so a push requires `git push --force-with-lease`. When the
plan reports `remote.originBranchExists: true`, report that command as the exact next action. Do not run it unless the
user explicitly requests it.

## 6. Failure Recovery

Apply the step 4 retry rules of `SKILL.md` to the same transaction first. If the squash still cannot reach `COMMITTED`
after `reset`, recover in this order:

1. If preparation printed a transaction ID, run `ai-commit discard <transaction-id>` and require `DISCARDED`.
2. Run `uv run "<skill-dir>/scripts/git-squash.py" rollback --plan <plan.json>`.

`rollback` requires HEAD at the merge base on the planned branch. Otherwise it fails without changes. It restores the
original HEAD and the planned index, leaves working-tree files untouched, and prints `status: "restored"`. Never
reproduce the reset or rollback sequence with manual Git commands.

After `COMMITTED`, `rollback` refuses because HEAD left the merge base. When a helper or `ai-commit` command fails,
inspect its diagnostic and the current Git state before you continue.

## 7. Report

Lead with `### ✅ Squashed — <old count> commits → 1`. Include the resolved base ref, the `COMMITTED` receipt OID, and
the subject. Forward the step 5 receipt lines of `SKILL.md` unchanged. Keep hashes, commands, errors, and rollback
wording plain and exact.

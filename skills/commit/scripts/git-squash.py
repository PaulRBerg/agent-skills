#!/usr/bin/env python3
"""Plan, reset, or roll back a one-commit Git branch squash."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


PLAN_KEYS = ("repoRoot", "branch", "baseRef", "mergeBase", "originalHead", "aheadCount", "authors", "rollback")


class GitError(RuntimeError):
    pass


def git(cwd: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(["git", *args], cwd=cwd, text=True, capture_output=True)
    if check and result.returncode:
        raise GitError((result.stderr or result.stdout).strip() or f"git {' '.join(args)} failed")
    return result


def ref_exists(cwd: Path, ref: str) -> bool:
    return git(cwd, "show-ref", "--verify", "--quiet", ref, check=False).returncode == 0


def current_branch(cwd: Path) -> str:
    result = git(cwd, "symbolic-ref", "--quiet", "--short", "HEAD", check=False)
    if result.returncode:
        raise GitError("HEAD is detached")
    return result.stdout.strip()


def resolve_base(cwd: Path, requested: str | None) -> tuple[str, str]:
    branch = requested
    if not branch:
        symbolic = git(cwd, "symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD", check=False)
        if symbolic.returncode == 0 and symbolic.stdout.strip().startswith("origin/"):
            branch = symbolic.stdout.strip().removeprefix("origin/")
    if not branch:
        branch = next((candidate for candidate in ("main", "master", "trunk") if ref_exists(cwd, f"refs/heads/{candidate}") or ref_exists(cwd, f"refs/remotes/origin/{candidate}")), None)
    if not branch:
        raise GitError("cannot resolve a default branch; pass --base")
    branch = branch.removeprefix("refs/heads/").removeprefix("refs/remotes/origin/").removeprefix("origin/")
    if ref_exists(cwd, f"refs/heads/{branch}"):
        return branch, f"refs/heads/{branch}"
    if ref_exists(cwd, f"refs/remotes/origin/{branch}"):
        return branch, f"refs/remotes/origin/{branch}"
    raise GitError(f"base branch does not exist locally or on origin: {branch}")


def remote_facts(cwd: Path, branch: str) -> dict[str, Any]:
    upstream = git(cwd, "rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}", check=False)
    return {
        "originConfigured": git(cwd, "remote", "get-url", "origin", check=False).returncode == 0,
        "upstream": upstream.stdout.strip() if upstream.returncode == 0 else None,
        "originBranchExists": ref_exists(cwd, f"refs/remotes/origin/{branch}"),
    }


def preflight(cwd: Path, requested_base: str | None) -> dict[str, Any]:
    inside = git(cwd, "rev-parse", "--is-inside-work-tree", check=False)
    if inside.returncode or inside.stdout.strip() != "true":
        raise GitError("not inside a Git worktree")
    root = Path(git(cwd, "rev-parse", "--show-toplevel").stdout.strip())
    branch = current_branch(root)
    status = git(root, "status", "--porcelain=v1").stdout.splitlines()
    if status:
        raise GitError("working tree or index is not clean")
    base_branch, base_ref = resolve_base(root, requested_base)
    if branch == base_branch:
        raise GitError("current branch is the default branch")
    merge_base_result = git(root, "merge-base", "HEAD", base_ref, check=False)
    if merge_base_result.returncode:
        raise GitError(f"HEAD and {base_ref} have no merge base")
    merge_base = merge_base_result.stdout.strip()
    original_head = git(root, "rev-parse", "HEAD").stdout.strip()
    ahead_count = int(git(root, "rev-list", "--count", f"{merge_base}..HEAD").stdout.strip())
    if ahead_count == 0:
        raise GitError("branch has no commits ahead of the merge base")
    commits = []
    raw_commits = git(root, "log", "--reverse", "--format=%H%x09%aN%x09%aE%x09%s", f"{merge_base}..HEAD").stdout
    for line in raw_commits.splitlines():
        commit, author_name, author_email, commit_subject = line.split("\t", 3)
        commits.append({"commit": commit, "author": {"name": author_name, "email": author_email}, "subject": commit_subject})
    authors = sorted({f"{item['author']['name']} <{item['author']['email']}>" for item in commits})
    return {
        "schemaVersion": 1,
        "repoRoot": str(root),
        "branch": branch,
        "baseBranch": base_branch,
        "baseRef": base_ref,
        "mergeBase": merge_base,
        "originalHead": original_head,
        "aheadCount": ahead_count,
        "commits": commits,
        "authors": authors,
        "treeState": {"clean": True, "status": []},
        "remote": remote_facts(root, branch),
        "rollback": {"head": original_head, "indexTree": git(root, "write-tree").stdout.strip()},
    }


def restore(cwd: Path, original_head: str, index_tree: str) -> None:
    errors: list[str] = []
    if git(cwd, "update-ref", "HEAD", original_head, check=False).returncode:
        errors.append("failed to restore HEAD")
    if git(cwd, "read-tree", index_tree, check=False).returncode:
        errors.append("failed to restore index")
    if errors:
        raise GitError("; ".join(errors))


def load_plan(path: Path) -> tuple[dict[str, Any], Path]:
    try:
        facts = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise GitError(f"cannot read plan: {exc}") from exc
    if facts.get("schemaVersion") != 1:
        raise GitError("plan schemaVersion must be 1")
    missing = [key for key in PLAN_KEYS if key not in facts]
    if missing:
        raise GitError(f"plan is missing {', '.join(missing)}")
    root = Path(facts["repoRoot"])
    if not root.is_dir():
        raise GitError(f"repository root does not exist: {root}")
    return facts, root


def reset_squash(plan_path: Path) -> dict[str, Any]:
    facts, root = load_plan(plan_path)
    original_head = facts["originalHead"]
    if git(root, "rev-parse", "HEAD").stdout.strip() != original_head:
        raise GitError("stale plan: HEAD changed after planning")
    if current_branch(root) != facts["branch"]:
        raise GitError("stale plan: branch changed after planning")
    if git(root, "status", "--porcelain=v1").stdout:
        raise GitError("stale plan: working tree or index is not clean")
    if git(root, "merge-base", "HEAD", facts["baseRef"]).stdout.strip() != facts["mergeBase"]:
        raise GitError("stale plan: merge base changed after planning")
    index_tree = facts["rollback"]["indexTree"]
    try:
        git(root, "reset", "--soft", facts["mergeBase"])
        staged = git(root, "diff", "--cached", "--quiet", check=False)
        if staged.returncode == 0:
            raise GitError("squash would produce an empty commit")
        if staged.returncode != 1:
            raise GitError(staged.stderr.strip() or "git diff --cached failed")
    except BaseException:
        restore(root, original_head, index_tree)
        raise
    return {
        "schemaVersion": 1,
        "status": "reset",
        "branch": facts["branch"],
        "originalHead": original_head,
        "mergeBase": facts["mergeBase"],
        "commitsReplaced": facts["aheadCount"],
        "authors": facts["authors"],
        "rollback": {"head": original_head, "indexTree": index_tree},
    }


def rollback_squash(plan_path: Path) -> dict[str, Any]:
    facts, root = load_plan(plan_path)
    if current_branch(root) != facts["branch"]:
        raise GitError("nothing to roll back: branch changed after planning")
    if git(root, "rev-parse", "HEAD").stdout.strip() != facts["mergeBase"]:
        raise GitError("nothing to roll back: HEAD is not at the merge base")
    restore(root, facts["originalHead"], facts["rollback"]["indexTree"])
    return {"schemaVersion": 1, "status": "restored", "head": facts["originalHead"]}


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    plan = subparsers.add_parser("plan", help="read-only preflight; prints the plan JSON")
    plan.add_argument("--cwd", type=Path, default=Path.cwd())
    plan.add_argument("--base")
    reset = subparsers.add_parser("reset", help="revalidate the plan, then soft-reset to the merge base")
    reset.add_argument("--plan", required=True, type=Path)
    rollback = subparsers.add_parser("rollback", help="restore the planned HEAD and index after a failed squash")
    rollback.add_argument("--plan", required=True, type=Path)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    try:
        if args.command == "plan":
            result = preflight(args.cwd, args.base)
        elif args.command == "reset":
            result = reset_squash(args.plan)
        else:
            result = rollback_squash(args.plan)
    except (OSError, GitError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

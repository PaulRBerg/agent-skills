#!/usr/bin/env python3
"""Plan or atomically apply a one-commit Git branch squash."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any


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
    if not branch and git(cwd, "remote", "get-url", "origin", check=False).returncode == 0:
        remote = git(cwd, "remote", "show", "-n", "origin", check=False)
        for line in remote.stdout.splitlines():
            if "HEAD branch:" in line and not line.rstrip().endswith("(unknown)"):
                branch = line.split("HEAD branch:", 1)[1].strip()
                break
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


def preflight(cwd: Path, requested_base: str | None, subject: str | None) -> dict[str, Any]:
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
        "subjectOverride": subject,
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


def load_apply_facts(args: argparse.Namespace) -> dict[str, Any]:
    if args.plan:
        try:
            facts = json.loads(args.plan.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise GitError(f"cannot read plan: {exc}") from exc
        if facts.get("schemaVersion") != 1:
            raise GitError("plan schemaVersion must be 1")
        return facts
    required = {"expectedHead": args.expected_head, "mergeBase": args.merge_base, "baseRef": args.base_ref}
    missing = [name for name, value in required.items() if not value]
    if missing:
        raise GitError(f"apply requires --plan or {', '.join('--' + name.replace('Ref', '-ref').replace('Base', '-base').replace('Head', '-head').lower() for name in missing)}")
    return {
        "schemaVersion": 1,
        "repoRoot": str(args.cwd.resolve()),
        "branch": args.branch,
        "baseRef": args.base_ref,
        "baseBranch": args.base_ref.removeprefix("refs/heads/").removeprefix("refs/remotes/origin/"),
        "mergeBase": args.merge_base,
        "originalHead": args.expected_head,
        "rollback": {"head": args.expected_head, "indexTree": args.index_tree},
        "subjectOverride": args.subject,
    }


def apply_squash(args: argparse.Namespace) -> dict[str, Any]:
    facts = load_apply_facts(args)
    root = Path(facts["repoRoot"])
    if not root.is_dir():
        raise GitError(f"repository root does not exist: {root}")
    original_head = facts["originalHead"]
    expected_branch = facts.get("branch")
    if git(root, "rev-parse", "HEAD").stdout.strip() != original_head:
        raise GitError("stale plan: HEAD changed after planning")
    if expected_branch and current_branch(root) != expected_branch:
        raise GitError("stale plan: branch changed after planning")
    if git(root, "status", "--porcelain=v1").stdout:
        raise GitError("stale plan: working tree or index is not clean")
    current_merge_base = git(root, "merge-base", "HEAD", facts["baseRef"]).stdout.strip()
    if current_merge_base != facts["mergeBase"]:
        raise GitError("stale plan: merge base changed after planning")
    message = args.message_file.read_text(encoding="utf-8")
    if not message.strip():
        raise GitError("message file is empty")
    subject_override = args.subject or facts.get("subjectOverride")
    if subject_override and message.splitlines()[0] != subject_override:
        raise GitError("message subject does not match --subject")
    index_tree = facts.get("rollback", {}).get("indexTree") or git(root, "write-tree").stdout.strip()
    mutated = False
    committed = False
    try:
        mutated = True
        git(root, "reset", "--soft", facts["mergeBase"])
        if git(root, "diff", "--cached", "--quiet", check=False).returncode == 0:
            raise GitError("squash would produce an empty commit")
        commit = git(root, "commit", "-F", str(args.message_file), check=False)
        if commit.returncode:
            raise GitError((commit.stderr or commit.stdout).strip() or "replacement commit failed")
        committed = True
    finally:
        if mutated and not committed:
            restore(root, original_head, index_tree)
    new_head = git(root, "rev-parse", "HEAD").stdout.strip()
    return {
        "schemaVersion": 1,
        "status": "squashed",
        "originalHead": original_head,
        "newHead": new_head,
        "mergeBase": facts["mergeBase"],
        "commitsReplaced": facts.get("aheadCount"),
        "subject": git(root, "show", "-s", "--format=%s", "HEAD").stdout.strip(),
        "remote": remote_facts(root, current_branch(root)),
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    plan = subparsers.add_parser("plan")
    plan.add_argument("--cwd", type=Path, default=Path.cwd())
    plan.add_argument("--base")
    plan.add_argument("--subject")

    apply = subparsers.add_parser("apply")
    apply.add_argument("--cwd", type=Path, default=Path.cwd())
    apply.add_argument("--plan", type=Path)
    apply.add_argument("--expected-head")
    apply.add_argument("--merge-base")
    apply.add_argument("--base-ref")
    apply.add_argument("--branch")
    apply.add_argument("--index-tree")
    apply.add_argument("--message-file", required=True, type=Path)
    apply.add_argument("--subject")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    try:
        result = preflight(args.cwd, args.base, args.subject) if args.command == "plan" else apply_squash(args)
    except (OSError, GitError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

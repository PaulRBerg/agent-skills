#!/usr/bin/env -S uv run --script
"""Preview and apply a constrained GitHub repository rename."""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path


EXCLUDED_PARTS = {".git", "node_modules", ".venv", "venv", ".cache", "__pycache__", "dist", "build", ".next", "coverage"}
PROTECTED_FILES = {"PROMPT.md", "TODO.md"}


class RenameError(RuntimeError):
    pass


@dataclass(frozen=True)
class Replacement:
    path: Path
    pairs: tuple[tuple[bytes, bytes], ...]
    occurrences: int


def claude_project_name(root: Path) -> str:
    return re.sub(r"[^A-Za-z0-9]", "-", str(root))


def run(*args: str, cwd: Path | None = None) -> str:
    result = subprocess.run(args, cwd=cwd, check=False, text=True, capture_output=True)
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip() or f"exit {result.returncode}"
        raise RenameError(f"{' '.join(args)}: {detail}")
    return result.stdout.strip()


def readable_files(root: Path, *, repo: bool = False, include_local_state: bool = False) -> list[Path]:
    protected = set() if include_local_state else PROTECTED_FILES
    excluded = {".git"} if include_local_state else EXCLUDED_PARTS
    if root.is_symlink() or not root.exists() or root.name in protected:
        return []
    if root.is_file():
        return [root]
    files: list[Path] = []
    for directory, names, filenames in os.walk(root):
        parent = Path(directory)
        names[:] = [
            name for name in names
            if not (parent / name).is_symlink() and not (repo and name in excluded)
        ]
        for name in filenames:
            path = parent / name
            if name not in protected and not path.is_symlink() and path.is_file():
                files.append(path)
    return files


def count_pairs(path: Path, pairs: tuple[tuple[bytes, bytes], ...]) -> int:
    try:
        data = path.read_bytes()
    except OSError:
        return 0
    if b"\0" in data:
        return 0
    return sum(data.count(old) for old, _ in pairs)


def continuity_paths() -> tuple[Path, Path]:
    home = Path.home()
    return (
        Path(os.environ.get("CLAUDE_CONFIG_DIR") or home / ".claude") / "projects",
        Path(os.environ.get("CODEX_HOME") or home / ".codex"),
    )


def collect_replacements(
    old_root: Path, new_root: Path, old_name: str, new_name: str, *, include_local_state: bool = False
) -> list[Replacement]:
    claude_projects, codex_home = continuity_paths()
    path_pair = ((str(old_root).encode(), str(new_root).encode()),)
    replacements = []
    for path in readable_files(codex_home / "sessions"):
        occurrences = count_pairs(path, path_pair)
        if occurrences:
            replacements.append(Replacement(path, path_pair, occurrences))

    project = claude_projects / claude_project_name(old_root)
    claude_active = project.is_dir() and next(project.iterdir(), None) is not None
    if claude_active or replacements:
        candidates = set()
        for root in (claude_projects, Path.home() / "claude/projects", codex_home / "config.toml"):
            candidates.update(readable_files(root))
        for path in candidates:
            occurrences = count_pairs(path, path_pair)
            if occurrences:
                replacements.append(Replacement(path, path_pair, occurrences))

    name_pair = ((old_name.encode(), new_name.encode()),)
    for path in readable_files(old_root, repo=True, include_local_state=include_local_state):
        occurrences = count_pairs(path, name_pair)
        if occurrences:
            replacements.append(Replacement(path, name_pair, occurrences))
    return sorted(replacements, key=lambda item: str(item.path))


def preflight(new_name: str, *, include_local_state: bool = False) -> dict[str, object]:
    if not new_name or new_name.startswith(".") or "/" in new_name:
        raise RenameError(f"invalid repo name: {new_name!r}")

    old_root = Path(run("git", "rev-parse", "--show-toplevel")).resolve()
    status = run("git", "status", "--short", cwd=old_root)
    if status:
        raise RenameError(f"working tree is not clean:\n{status}")

    repo = json.loads(run("gh", "repo", "view", "--json", "nameWithOwner,sshUrl,url", cwd=old_root))
    old_repo = repo["nameWithOwner"]
    owner, repo_name = old_repo.split("/", 1)
    old_name = old_root.name
    if repo_name != old_name:
        raise RenameError(f"repo/folder mismatch: {repo_name} vs {old_name}")

    new_root = old_root.parent / new_name
    if new_root.exists():
        raise RenameError(f"target folder already exists: {new_root}")

    remote = run("git", "remote", "get-url", "origin", cwd=old_root)
    if remote.startswith("git@github.com:"):
        new_remote = f"git@github.com:{owner}/{new_name}.git"
    elif remote.startswith("https://github.com/"):
        new_remote = f"https://github.com/{owner}/{new_name}.git"
    else:
        new_remote = f"git@github.com:{owner}/{new_name}.git"

    claude_projects, _ = continuity_paths()
    old_claude = claude_projects / claude_project_name(old_root)
    new_claude = claude_projects / claude_project_name(new_root)
    if old_claude.exists() and new_claude.exists():
        raise RenameError(f"target Claude project directory exists: {new_claude}")

    replacements = collect_replacements(old_root, new_root, old_name, new_name, include_local_state=include_local_state)
    token = f"{old_repo}->{owner}/{new_name}"
    return {
        "old_root": old_root,
        "new_root": new_root,
        "old_name": old_name,
        "new_name": new_name,
        "owner": owner,
        "old_repo": old_repo,
        "new_repo": f"{owner}/{new_name}",
        "old_remote": remote,
        "new_remote": new_remote,
        "old_claude": old_claude,
        "new_claude": new_claude,
        "move_claude": old_claude.is_dir() and next(old_claude.iterdir(), None) is not None,
        "replacements": replacements,
        "confirm": token,
        "include_local_state": include_local_state,
    }


def preview(plan: dict[str, object]) -> dict[str, object]:
    replacements = plan["replacements"]
    assert isinstance(replacements, list)
    return {
        "mode": "dry-run",
        "old_repo": plan["old_repo"],
        "new_repo": plan["new_repo"],
        "old_root": str(plan["old_root"]),
        "new_root": str(plan["new_root"]),
        "confirmation_token": plan["confirm"],
        "include_local_state": plan["include_local_state"],
        "mutations": [
            {"kind": "github-rename", "command": f"gh repo rename {plan['new_name']} --yes"},
            {"kind": "origin", "from": plan["old_remote"], "to": plan["new_remote"]},
            {"kind": "folder", "from": str(plan["old_root"]), "to": str(plan["new_root"])},
            {
                "kind": "claude-project-folder",
                "enabled": plan["move_claude"],
                "from": str(plan["old_claude"]),
                "to": str(plan["new_claude"]),
            },
            {
                "kind": "literal-replacements",
                "files": [
                    {"path": str(item.path), "occurrences": item.occurrences} for item in replacements
                ],
            },
        ],
        "rollback": "replacement files are backed up; folder, origin, Claude directory, and GitHub name are restored in reverse order on failure",
    }


def moved_path(path: Path, plan: dict[str, object]) -> Path:
    old_root = plan["old_root"]
    new_root = plan["new_root"]
    old_claude = plan["old_claude"]
    new_claude = plan["new_claude"]
    assert all(isinstance(item, Path) for item in (old_root, new_root, old_claude, new_claude))
    try:
        return new_root / path.relative_to(old_root)
    except ValueError:
        pass
    if plan["move_claude"]:
        try:
            return new_claude / path.relative_to(old_claude)
        except ValueError:
            pass
    return path


def apply(plan: dict[str, object]) -> dict[str, object]:
    old_root = plan["old_root"]
    new_root = plan["new_root"]
    old_claude = plan["old_claude"]
    new_claude = plan["new_claude"]
    replacements = plan["replacements"]
    assert isinstance(old_root, Path) and isinstance(new_root, Path)
    assert isinstance(old_claude, Path) and isinstance(new_claude, Path)
    assert isinstance(replacements, list)

    backup_dir = Path(tempfile.mkdtemp(prefix="repo-rename-rollback-"))
    backups: dict[Path, Path] = {}
    github_done = remote_done = repo_moved = claude_moved = False
    rollback_errors: list[str] = []
    try:
        for index, item in enumerate(replacements):
            backup = backup_dir / str(index)
            shutil.copy2(item.path, backup)
            backups[item.path] = backup

        run("gh", "repo", "rename", str(plan["new_name"]), "--yes", cwd=old_root)
        github_done = True
        run("git", "remote", "set-url", "origin", str(plan["new_remote"]), cwd=old_root)
        remote_done = True
        old_root.rename(new_root)
        repo_moved = True
        if plan["move_claude"]:
            old_claude.rename(new_claude)
            claude_moved = True

        for item in replacements:
            target = moved_path(item.path, plan)
            data = target.read_bytes()
            for old, new in item.pairs:
                data = data.replace(old, new)
            temp = target.with_name(f".{target.name}.repo-rename.tmp")
            temp.write_bytes(data)
            shutil.copymode(target, temp)
            os.replace(temp, target)

        shutil.rmtree(backup_dir)
        return {
            "mode": "apply",
            "status": "success",
            "repo": plan["new_repo"],
            "root": str(new_root),
            "remote": plan["new_remote"],
            "files_updated": len(replacements),
        }
    except Exception as error:
        for original, backup in backups.items():
            try:
                target = moved_path(original, plan) if repo_moved or claude_moved else original
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(backup, target)
            except Exception as rollback_error:  # pragma: no cover - best effort reporting
                rollback_errors.append(f"restore {original}: {rollback_error}")
        if claude_moved:
            try:
                new_claude.rename(old_claude)
            except Exception as rollback_error:
                rollback_errors.append(f"restore Claude directory: {rollback_error}")
        if repo_moved:
            try:
                new_root.rename(old_root)
                repo_moved = False
            except Exception as rollback_error:
                rollback_errors.append(f"restore repository folder: {rollback_error}")
        rollback_cwd = old_root if old_root.exists() else new_root
        if remote_done:
            try:
                run("git", "remote", "set-url", "origin", str(plan["old_remote"]), cwd=rollback_cwd)
            except Exception as rollback_error:
                rollback_errors.append(f"restore origin: {rollback_error}")
        if github_done:
            try:
                run("gh", "repo", "rename", str(plan["old_name"]), "--repo", str(plan["new_repo"]), "--yes")
            except Exception as rollback_error:
                rollback_errors.append(f"restore GitHub name: {rollback_error}")
        detail = f"apply failed: {error}"
        if rollback_errors:
            detail += "; rollback incomplete: " + "; ".join(rollback_errors)
        else:
            detail += "; rollback completed"
        raise RenameError(detail) from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("new_name")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--dry-run", action="store_true")
    mode.add_argument("--apply", action="store_true")
    parser.add_argument("--confirm")
    parser.add_argument("--include-local-state", action="store_true", help="include explicitly authorized notes, dependencies, caches, and generated files")
    args = parser.parse_args()
    try:
        plan = preflight(args.new_name, include_local_state=args.include_local_state)
        if args.apply and args.confirm != plan["confirm"]:
            raise RenameError(f"--apply requires --confirm {plan['confirm']!r}")
        result = apply(plan) if args.apply else preview(plan)
    except RenameError as error:
        print(f"repo-rename: {error}", file=sys.stderr)
        return 64
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Maintain deterministic path coverage for a repository-wide naming refactor."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


STATUSES = ("pending", "retained", "renamed", "excluded", "blocked")
REASON_REQUIRED = ("excluded", "blocked")


class LedgerError(ValueError):
    pass


def git(root: Path, *args: str) -> bytes:
    result = subprocess.run(["git", *args], cwd=root, capture_output=True)
    if result.returncode:
        message = result.stderr.decode(errors="replace").strip()
        raise LedgerError(message or f"git {' '.join(args)} failed")
    return result.stdout


def repo_root(path: Path) -> Path:
    return Path(git(path, "rev-parse", "--show-toplevel").decode().strip()).resolve()


def nul_paths(data: bytes) -> list[str]:
    return [value.decode(errors="surrogateescape") for value in data.split(b"\0") if value]


def worktree_status(root: Path) -> dict[str, list[str]]:
    fields = nul_paths(git(root, "status", "--porcelain=v1", "-z", "--untracked-files=all"))
    statuses: dict[str, list[str]] = {}
    index = 0
    while index < len(fields):
        entry = fields[index]
        if len(entry) < 4:
            raise LedgerError("unexpected git status record")
        code, path = entry[:2], entry[3:]
        statuses.setdefault(path, []).append(code)
        if "R" in code or "C" in code:
            index += 1
            if index >= len(fields):
                raise LedgerError("incomplete git rename status record")
            statuses.setdefault(fields[index], []).append(f"{code}:source")
        index += 1
    return statuses


def repository_paths(root: Path) -> tuple[set[str], set[str]]:
    tracked = set(nul_paths(git(root, "ls-files", "-z")))
    untracked = set(nul_paths(git(root, "ls-files", "--others", "--exclude-standard", "-z")))
    return tracked, untracked


def path_present(root: Path, path: str) -> bool:
    return os.path.lexists(root / path)


def ensure_external_ledger(root: Path, ledger: Path) -> Path:
    resolved = ledger.resolve()
    try:
        resolved.relative_to(root)
    except ValueError:
        return resolved
    raise LedgerError(f"ledger must be outside the repository: {resolved}")


def atomic_write(path: Path, payload: dict[str, Any]) -> None:
    encoded = (json.dumps(payload, indent=2, ensure_ascii=False) + "\n").encode()
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(encoded)
            handle.flush()
            os.fsync(handle.fileno())
        if path.exists():
            os.chmod(temporary, path.stat().st_mode)
        os.replace(temporary, path)
    finally:
        if temporary.exists():
            temporary.unlink()


def validate_record(item: Any) -> None:
    if not isinstance(item, dict):
        raise LedgerError("ledger contains an invalid path record")
    if not isinstance(item.get("path"), str) or not item["path"]:
        raise LedgerError("ledger contains an invalid path")
    if item.get("status") not in STATUSES:
        raise LedgerError(f"ledger contains an invalid status for {item['path']}")
    if not isinstance(item.get("tracked"), bool) or not isinstance(item.get("present"), bool):
        raise LedgerError(f"ledger contains invalid path state for {item['path']}")
    if not isinstance(item.get("preexistingStatus"), list) or not isinstance(item.get("discoveredStatus"), list):
        raise LedgerError(f"ledger contains invalid worktree state for {item['path']}")
    if not isinstance(item.get("firstSeenRevision"), int) or item["firstSeenRevision"] < 0:
        raise LedgerError(f"ledger contains an invalid discovery revision for {item['path']}")
    if item["status"] in REASON_REQUIRED and not item.get("reason"):
        raise LedgerError(f"{item['status']} path has no reason: {item['path']}")


def load(path: Path) -> dict[str, Any]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise LedgerError(f"cannot read ledger: {exc}") from exc
    if (
        payload.get("schemaVersion") != 1
        or not isinstance(payload.get("revision"), int)
        or not isinstance(payload.get("repoRoot"), str)
        or not isinstance(payload.get("files"), list)
    ):
        raise LedgerError("unsupported ledger schema")
    paths = set()
    for item in payload["files"]:
        validate_record(item)
        if item["path"] in paths:
            raise LedgerError(f"ledger contains a duplicate path: {item['path']}")
        paths.add(item["path"])
    return payload


def counts(payload: dict[str, Any]) -> dict[str, int]:
    result = {status: 0 for status in STATUSES}
    for item in payload["files"]:
        result[item["status"]] += 1
    result["mapped"] = len(payload["files"])
    result["accounted"] = result["mapped"] - result["pending"]
    result["present"] = sum(item["present"] for item in payload["files"])
    result["missing"] = result["mapped"] - result["present"]
    return result


def summary(payload: dict[str, Any]) -> dict[str, Any]:
    summary_counts = counts(payload)
    return {
        "schemaVersion": 1,
        "revision": payload["revision"],
        "counts": summary_counts,
        "complete": summary_counts["pending"] == 0 and summary_counts["blocked"] == 0,
        "preexistingChangedPaths": sum(bool(item["preexistingStatus"]) for item in payload["files"]),
    }


def new_record(
    root: Path,
    path: str,
    tracked: set[str],
    status: dict[str, list[str]],
    revision: int,
) -> dict[str, Any]:
    discovered = status.get(path, [])
    return {
        "path": path,
        "tracked": path in tracked,
        "present": path_present(root, path),
        "preexistingStatus": discovered if revision == 0 else [],
        "discoveredStatus": discovered,
        "firstSeenRevision": revision,
        "status": "pending",
        "reason": None,
    }


def init_command(args: argparse.Namespace) -> dict[str, Any]:
    root = repo_root(args.root.resolve())
    ledger = ensure_external_ledger(root, args.ledger)
    if ledger.exists():
        raise LedgerError(f"ledger already exists: {ledger}")
    tracked, untracked = repository_paths(root)
    status = worktree_status(root)
    paths = sorted(tracked | untracked)
    payload = {
        "schemaVersion": 1,
        "revision": 0,
        "repoRoot": str(root),
        "files": [new_record(root, path, tracked, status, 0) for path in paths],
    }
    ledger.parent.mkdir(parents=True, exist_ok=True)
    atomic_write(ledger, payload)
    return {"schemaVersion": 1, "ledger": str(ledger), **summary(payload)}


def mark_command(args: argparse.Namespace) -> dict[str, Any]:
    payload = load(args.ledger)
    paths = list(dict.fromkeys(args.path))
    if not paths:
        raise LedgerError("mark requires at least one --path")
    if args.status in REASON_REQUIRED and not args.reason:
        raise LedgerError(f"{args.status} status requires --reason")
    by_path = {item["path"]: item for item in payload["files"]}
    missing = [path for path in paths if path not in by_path]
    if missing:
        raise LedgerError(f"paths are not in the ledger: {', '.join(missing)}")
    absent_retained = [path for path in paths if args.status == "retained" and not by_path[path]["present"]]
    if absent_retained:
        raise LedgerError(f"absent paths cannot be retained: {', '.join(absent_retained)}")
    for path in paths:
        by_path[path]["status"] = args.status
        by_path[path]["reason"] = args.reason
    payload["revision"] += 1
    atomic_write(args.ledger, payload)
    return {"schemaVersion": 1, "updated": paths, "status": args.status, **summary(payload)}


def pending_command(args: argparse.Namespace) -> dict[str, Any]:
    payload = load(args.ledger)
    paths = [item["path"] for item in payload["files"] if item["status"] == "pending"]
    if args.limit is not None:
        paths = paths[: args.limit]
    return {
        "schemaVersion": 1,
        "revision": payload["revision"],
        "paths": paths,
        "remaining": counts(payload)["pending"],
    }


def refresh_command(args: argparse.Namespace) -> dict[str, Any]:
    payload = load(args.ledger)
    root = repo_root(Path(payload["repoRoot"]))
    if str(root) != payload["repoRoot"]:
        raise LedgerError(f"repository root changed: {root}")
    tracked, untracked = repository_paths(root)
    status = worktree_status(root)
    current_paths = tracked | untracked
    by_path = {item["path"]: item for item in payload["files"]}
    changed = False
    for item in payload["files"]:
        present = path_present(root, item["path"])
        if item["present"] != present:
            item["present"] = present
            changed = True
    next_revision = payload["revision"] + 1
    added_paths = sorted(current_paths - by_path.keys())
    for path in added_paths:
        payload["files"].append(new_record(root, path, tracked, status, next_revision))
        changed = True
    if changed:
        payload["files"].sort(key=lambda item: item["path"])
        payload["revision"] = next_revision
        atomic_write(args.ledger, payload)
    missing_paths = [item["path"] for item in payload["files"] if not item["present"]]
    return {
        "schemaVersion": 1,
        "addedPaths": added_paths,
        "missingPaths": missing_paths,
        **summary(payload),
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    init = subparsers.add_parser("init")
    init.add_argument("--root", type=Path, default=Path.cwd())
    init.add_argument("--ledger", type=Path, required=True)
    init.set_defaults(handler=init_command)
    mark = subparsers.add_parser("mark")
    mark.add_argument("--ledger", type=Path, required=True)
    mark.add_argument("--status", choices=STATUSES, required=True)
    mark.add_argument("--path", action="append", default=[])
    mark.add_argument("--reason")
    mark.set_defaults(handler=mark_command)
    pending = subparsers.add_parser("pending")
    pending.add_argument("--ledger", type=Path, required=True)
    pending.add_argument("--limit", type=int)
    pending.set_defaults(handler=pending_command)
    refresh = subparsers.add_parser("refresh")
    refresh.add_argument("--ledger", type=Path, required=True)
    refresh.set_defaults(handler=refresh_command)
    show = subparsers.add_parser("summary")
    show.add_argument("--ledger", type=Path, required=True)
    show.set_defaults(handler=lambda args: summary(load(args.ledger)))
    return parser


def main() -> int:
    args = build_parser().parse_args()
    try:
        if getattr(args, "limit", 1) is not None and getattr(args, "limit", 1) < 1:
            raise LedgerError("--limit must be positive")
        result = args.handler(args)
    except LedgerError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 64
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

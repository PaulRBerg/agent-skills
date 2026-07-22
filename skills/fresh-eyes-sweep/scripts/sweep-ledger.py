#!/usr/bin/env python3
"""Maintain the deterministic coverage ledger for a fresh-eyes sweep."""

from __future__ import annotations

import argparse
import json
import math
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


STATUSES = ("pending", "inspected", "fixed", "reported", "excluded")


class LedgerError(ValueError):
    pass


def git(root: Path, *args: str) -> bytes:
    result = subprocess.run(["git", *args], cwd=root, capture_output=True)
    if result.returncode:
        raise LedgerError(result.stderr.decode(errors="replace").strip() or f"git {' '.join(args)} failed")
    return result.stdout


def repo_root(path: Path) -> Path:
    return Path(git(path, "rev-parse", "--show-toplevel").decode().strip())


def nul_paths(data: bytes) -> list[str]:
    return [value.decode(errors="surrogateescape") for value in data.split(b"\0") if value]


def preexisting_status(root: Path) -> dict[str, list[str]]:
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


def normalize_scopes(root: Path, values: list[str]) -> list[str]:
    if not values:
        return ["."]
    scopes: list[str] = []
    for value in values:
        candidate = Path(value)
        absolute = candidate.resolve() if candidate.is_absolute() else (root / candidate).resolve()
        try:
            relative = absolute.relative_to(root.resolve()).as_posix()
        except ValueError as exc:
            raise LedgerError(f"scope is outside the repository: {value}") from exc
        scopes.append(relative or ".")
    return list(dict.fromkeys(scopes))


def in_scope(path: str, scopes: list[str]) -> bool:
    return any(scope == "." or path == scope or path.startswith(f"{scope.rstrip('/')}/") for scope in scopes)


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


def load(path: Path) -> dict[str, Any]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise LedgerError(f"cannot read ledger: {exc}") from exc
    if payload.get("schemaVersion") != 1 or not isinstance(payload.get("files"), list):
        raise LedgerError("unsupported ledger schema")
    paths = set()
    for item in payload["files"]:
        if not isinstance(item, dict) or item.get("status") not in STATUSES or not isinstance(item.get("path"), str):
            raise LedgerError("ledger contains an invalid file record")
        if item["path"] in paths:
            raise LedgerError(f"ledger contains a duplicate path: {item['path']}")
        paths.add(item["path"])
        if item["status"] == "excluded" and not item.get("reason"):
            raise LedgerError(f"excluded path has no reason: {item['path']}")
    return payload


def counts(payload: dict[str, Any]) -> dict[str, int]:
    result = {status: 0 for status in STATUSES}
    for item in payload["files"]:
        result[item["status"]] += 1
    result["mapped"] = len(payload["files"])
    result["accounted"] = result["mapped"] - result["pending"]
    return result


def summary(payload: dict[str, Any]) -> dict[str, Any]:
    summary_counts = counts(payload)
    mapped = summary_counts["mapped"]
    filled = None if mapped == 0 else min(10, math.floor(10 * summary_counts["accounted"] / mapped + 0.5))
    return {
        "schemaVersion": 1,
        "revision": payload["revision"],
        "counts": summary_counts,
        "complete": summary_counts["pending"] == 0,
        "progress": None if filled is None else {"filled": filled, "empty": 10 - filled, "bar": "█" * filled + "░" * (10 - filled)},
        "preexistingChangedFiles": sum(bool(item.get("preexistingStatus")) for item in payload["files"]),
    }


def init_command(args: argparse.Namespace) -> dict[str, Any]:
    if args.ledger.exists():
        raise LedgerError(f"ledger already exists: {args.ledger}")
    root = repo_root(args.root.resolve())
    scopes = normalize_scopes(root, args.scopes + args.scope)
    tracked = set(nul_paths(git(root, "ls-files", "-z")))
    untracked = set(nul_paths(git(root, "ls-files", "--others", "--exclude-standard", "-z")))
    status = preexisting_status(root)
    paths = sorted(path for path in tracked | untracked if in_scope(path, scopes))
    payload = {
        "schemaVersion": 1,
        "revision": 0,
        "repoRoot": str(root),
        "scopes": scopes,
        "files": [
            {
                "path": path,
                "tracked": path in tracked,
                "preexistingStatus": status.get(path, []),
                "status": "pending",
                "reason": None,
            }
            for path in paths
        ],
    }
    args.ledger.parent.mkdir(parents=True, exist_ok=True)
    atomic_write(args.ledger, payload)
    return {"schemaVersion": 1, "ledger": str(args.ledger), "scopes": scopes, **summary(payload)}


def mark_command(args: argparse.Namespace) -> dict[str, Any]:
    payload = load(args.ledger)
    paths = list(dict.fromkeys(args.path))
    if not paths:
        raise LedgerError("mark requires at least one --path")
    if args.status == "excluded" and not args.reason:
        raise LedgerError("excluded status requires --reason")
    by_path = {item["path"]: item for item in payload["files"]}
    missing = [path for path in paths if path not in by_path]
    if missing:
        raise LedgerError(f"paths are not in the ledger: {', '.join(missing)}")
    for path in paths:
        by_path[path]["status"] = args.status
        by_path[path]["reason"] = args.reason if args.status == "excluded" else None
    payload["revision"] += 1
    atomic_write(args.ledger, payload)
    return {"schemaVersion": 1, "updated": paths, "status": args.status, **summary(payload)}


def pending_command(args: argparse.Namespace) -> dict[str, Any]:
    payload = load(args.ledger)
    paths = [item["path"] for item in payload["files"] if item["status"] == "pending"]
    if args.limit is not None:
        paths = paths[: args.limit]
    return {"schemaVersion": 1, "revision": payload["revision"], "paths": paths, "remaining": counts(payload)["pending"]}


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    init = subparsers.add_parser("init")
    init.add_argument("scopes", nargs="*")
    init.add_argument("--scope", action="append", default=[])
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

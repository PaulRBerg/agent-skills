#!/usr/bin/env python3
"""Preview or atomically apply selected Taze updates to Bun catalogs."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import tempfile
from pathlib import Path
from typing import Any

VERSION_RE = re.compile(r"^(?P<prefix>\^|~)?(?P<version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$")


class CatalogError(ValueError):
    pass


def parse_version(value: Any, context: str) -> tuple[str, str]:
    if not isinstance(value, str) or not (match := VERSION_RE.fullmatch(value)):
        raise CatalogError(f"unsupported version in {context}: {value!r}")
    return match.group("prefix") or "", match.group("version")


def selected_names(values: list[str]) -> list[str]:
    names: list[str] = []
    for value in values:
        names.extend(item.strip() for item in value.split(",") if item.strip())
    if not names:
        raise CatalogError("--include must select at least one package")
    return list(dict.fromkeys(names))


def plan_updates(plan: dict[str, Any], names: list[str]) -> dict[str, list[dict[str, Any]]]:
    updates = plan.get("updates")
    if not isinstance(updates, list):
        raise CatalogError("plan updates must be an array")
    selected: dict[str, list[dict[str, Any]]] = {}
    for name in names:
        matches = [entry for entry in updates if isinstance(entry, dict) and entry.get("package") == name]
        if not matches:
            raise CatalogError(f"selected package is missing from plan: {name}")
        selected[name] = matches
    return selected


def catalog_maps(document: dict[str, Any]) -> list[tuple[str, dict[str, Any]]]:
    workspaces = document.get("workspaces")
    if isinstance(workspaces, dict) and ("catalog" in workspaces or "catalogs" in workspaces):
        location = workspaces
        label = "workspaces."
    elif "catalog" in document or "catalogs" in document:
        location = document
        label = ""
    else:
        raise CatalogError("package.json has no Bun catalog configuration")
    maps: list[tuple[str, dict[str, Any]]] = []
    default = location.get("catalog")
    if default is not None:
        if not isinstance(default, dict):
            raise CatalogError(f"{label}catalog must be an object")
        maps.append(("default", default))
    named = location.get("catalogs")
    if named is not None:
        if not isinstance(named, dict):
            raise CatalogError(f"{label}catalogs must be an object")
        for catalog_name, catalog in sorted(named.items()):
            if not isinstance(catalog, dict):
                raise CatalogError(f"{label}catalogs.{catalog_name} must be an object")
            maps.append((catalog_name, catalog))
    return maps


def update_document(
    document: dict[str, Any], selected: dict[str, list[dict[str, Any]]], catalogs: list[str] | None = None
) -> list[dict[str, str]]:
    maps = catalog_maps(document)
    if catalogs is not None:
        unknown = set(catalogs) - {name for name, _ in maps}
        if unknown:
            raise CatalogError(f"unknown Bun catalogs: {', '.join(sorted(unknown))}")
        maps = [(name, catalog) for name, catalog in maps if name in catalogs]
    changes: list[dict[str, str]] = []
    for package, entries in selected.items():
        occurrences = [(name, catalog) for name, catalog in maps if package in catalog]
        if not occurrences:
            raise CatalogError(f"selected package is missing from selected Bun catalogs: {package}")
        for catalog_name, catalog in occurrences:
            prefix, current = parse_version(catalog[package], f"catalog {catalog_name} entry for {package}")
            matches = []
            for entry in entries:
                value = entry.get("current")
                # Unselected occurrences can have other versions or unsupported peer ranges.
                if not isinstance(value, str) or not (match := VERSION_RE.fullmatch(value)):
                    continue
                if match.group("version") == current:
                    matches.append(entry)
            if not matches:
                raise CatalogError(
                    f"stale plan for {package} in catalog {catalog_name}: no matching current {catalog[package]}"
                )
            if not prefix and any(entry.get("action") == "skip-fixed" for entry in matches):
                continue
            targets = {parse_version(entry.get("available"), f"plan available for {package}")[1] for entry in matches}
            if len(targets) != 1:
                raise CatalogError(f"ambiguous plan targets for {package} in catalog {catalog_name}")
            plan_prefixes = {parse_version(entry["current"], f"plan current for {package}")[0] for entry in matches}
            if "" not in plan_prefixes and prefix not in plan_prefixes:
                raise CatalogError(f"stale plan prefix for {package} in catalog {catalog_name}")
            replacement = f"{prefix}{targets.pop()}"
            changes.append({"package": package, "catalog": catalog_name, "from": catalog[package], "to": replacement})
    # Validate every occurrence before changing the in-memory document or writing it.
    for change in changes:
        next(catalog for name, catalog in maps if name == change["catalog"])[change["package"]] = change["to"]
    return changes


def atomic_write(path: Path, document: dict[str, Any]) -> None:
    encoded = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(encoded)
            handle.flush()
            os.fsync(handle.fileno())
        os.chmod(temporary, path.stat().st_mode)
        os.replace(temporary, path)
    finally:
        if temporary.exists():
            temporary.unlink()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--plan", required=True, type=Path)
    parser.add_argument("--include", required=True, action="append")
    parser.add_argument("--catalog", action="append", help="catalog to update (repeatable; default selects all)")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    package_path = args.root if args.root.name == "package.json" else args.root / "package.json"
    try:
        names = selected_names(args.include)
        document = json.loads(package_path.read_text(encoding="utf-8"))
        plan = json.loads(args.plan.read_text(encoding="utf-8"))
        changes = update_document(document, plan_updates(plan, names), args.catalog)
        if args.write:
            atomic_write(package_path, document)
    except (OSError, json.JSONDecodeError, CatalogError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 64
    print(
        json.dumps({"schemaVersion": 1, "wrote": args.write, "file": str(package_path), "changes": changes}, indent=2)
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

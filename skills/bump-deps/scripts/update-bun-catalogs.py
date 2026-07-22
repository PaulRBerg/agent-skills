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


def plan_updates(plan: dict[str, Any], names: list[str]) -> dict[str, dict[str, Any]]:
    updates = plan.get("updates")
    if not isinstance(updates, list):
        raise CatalogError("plan updates must be an array")
    selected: dict[str, dict[str, Any]] = {}
    for name in names:
        matches = [entry for entry in updates if isinstance(entry, dict) and entry.get("package") == name]
        if not matches:
            raise CatalogError(f"selected package is missing from plan: {name}")
        normalized = {(entry.get("current"), entry.get("available")) for entry in matches}
        if len(normalized) != 1:
            raise CatalogError(f"ambiguous plan entries for package: {name}")
        entry = matches[0]
        parse_version(entry.get("current"), f"plan current for {name}")
        parse_version(entry.get("available"), f"plan available for {name}")
        selected[name] = entry
    return selected


def catalog_maps(document: dict[str, Any]) -> list[tuple[str, dict[str, Any]]]:
    workspaces = document.get("workspaces")
    if not isinstance(workspaces, dict):
        raise CatalogError("package.json has no object-valued workspaces catalog configuration")
    maps: list[tuple[str, dict[str, Any]]] = []
    default = workspaces.get("catalog")
    if default is not None:
        if not isinstance(default, dict):
            raise CatalogError("workspaces.catalog must be an object")
        maps.append(("default", default))
    named = workspaces.get("catalogs")
    if named is not None:
        if not isinstance(named, dict):
            raise CatalogError("workspaces.catalogs must be an object")
        for catalog_name, catalog in sorted(named.items()):
            if not isinstance(catalog, dict):
                raise CatalogError(f"workspaces.catalogs.{catalog_name} must be an object")
            maps.append((catalog_name, catalog))
    return maps


def update_document(document: dict[str, Any], selected: dict[str, dict[str, Any]]) -> list[dict[str, str]]:
    maps = catalog_maps(document)
    changes: list[dict[str, str]] = []
    for package, entry in selected.items():
        plan_prefix, plan_current = parse_version(entry["current"], f"plan current for {package}")
        _available_prefix, available = parse_version(entry["available"], f"plan available for {package}")
        occurrences = [(catalog_name, catalog) for catalog_name, catalog in maps if package in catalog]
        if not occurrences:
            raise CatalogError(f"selected package is missing from Bun catalogs: {package}")
        for catalog_name, catalog in occurrences:
            prefix, current = parse_version(catalog[package], f"catalog {catalog_name} entry for {package}")
            if current != plan_current:
                raise CatalogError(
                    f"stale plan for {package} in catalog {catalog_name}: catalog has {current}, plan has {plan_current}"
                )
            if plan_prefix and prefix != plan_prefix:
                raise CatalogError(
                    f"stale plan prefix for {package} in catalog {catalog_name}: catalog has {prefix!r}, plan has {plan_prefix!r}"
                )
            replacement = f"{prefix}{available}"
            changes.append({"package": package, "catalog": catalog_name, "from": catalog[package], "to": replacement})
            catalog[package] = replacement
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
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    package_path = args.root if args.root.name == "package.json" else args.root / "package.json"
    try:
        names = selected_names(args.include)
        document = json.loads(package_path.read_text(encoding="utf-8"))
        plan = json.loads(args.plan.read_text(encoding="utf-8"))
        changes = update_document(document, plan_updates(plan, names))
        if args.write:
            atomic_write(package_path, document)
    except (OSError, json.JSONDecodeError, CatalogError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 64
    print(json.dumps({"schemaVersion": 1, "wrote": args.write, "file": str(package_path), "changes": changes}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

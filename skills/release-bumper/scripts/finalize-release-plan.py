#!/usr/bin/env python3
"""Finalize only the mechanical parts of a release discovery record."""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


SEMVER_RE = re.compile(
    r"^(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*)"
    r"(?:-(?P<pre>[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z.-]+)?$"
)
SIMPLE_RANGE_RE = re.compile(r"^(?P<operator>\^|~|>=|<=|>|<|=)?(?P<version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$")


class InputError(ValueError):
    pass


@dataclass(frozen=True)
class Version:
    major: int
    minor: int
    patch: int
    prerelease: tuple[str, ...] = ()

    @classmethod
    def parse(cls, value: str) -> "Version":
        match = SEMVER_RE.fullmatch(value)
        if not match:
            raise InputError(f"invalid semver: {value}")
        prerelease = tuple((match.group("pre") or "").split(".")) if match.group("pre") else ()
        return cls(int(match.group("major")), int(match.group("minor")), int(match.group("patch")), prerelease)

    def core(self) -> str:
        return f"{self.major}.{self.minor}.{self.patch}"

    def __str__(self) -> str:
        return self.core() + (f"-{'.'.join(self.prerelease)}" if self.prerelease else "")


def compare(left: Version, right: Version) -> int:
    for a, b in zip((left.major, left.minor, left.patch), (right.major, right.minor, right.patch)):
        if a != b:
            return -1 if a < b else 1
    if not left.prerelease and not right.prerelease:
        return 0
    if not left.prerelease:
        return 1
    if not right.prerelease:
        return -1
    for a, b in zip(left.prerelease, right.prerelease):
        if a == b:
            continue
        a_num, b_num = a.isdigit(), b.isdigit()
        if a_num and b_num:
            return -1 if int(a) < int(b) else 1
        if a_num != b_num:
            return -1 if a_num else 1
        return -1 if a < b else 1
    return (len(left.prerelease) > len(right.prerelease)) - (len(left.prerelease) < len(right.prerelease))


def beta_version(current: str, explicit: str | None) -> str:
    base = Version.parse(explicit) if explicit else Version.parse(current)
    if explicit:
        if base.prerelease:
            return str(base)
        return f"{base.core()}-beta.1"
    if len(base.prerelease) == 2 and base.prerelease[0] == "beta" and base.prerelease[1].isdigit():
        return f"{base.core()}-beta.{int(base.prerelease[1]) + 1}"
    if base.prerelease:
        raise InputError(f"unsupported prerelease transition: {current}")
    return f"{base.major}.{base.minor}.{base.patch + 1}-beta.1"


def stable_promotion(current: str) -> str | None:
    version = Version.parse(current)
    return version.core() if version.prerelease else None


def range_details(value: str) -> dict[str, Any]:
    workspace_prefix = ""
    body = value
    if body.startswith("workspace:"):
        workspace_prefix = "workspace:"
        body = body[len(workspace_prefix) :]
        if body in {"*", "^", "~"}:
            return {
                "kind": "workspace-dynamic",
                "supported": True,
                "workspacePrefix": workspace_prefix,
                "operator": body,
                "version": None,
            }
    match = SIMPLE_RANGE_RE.fullmatch(body)
    if not match:
        return {"kind": "complex", "supported": False, "workspacePrefix": workspace_prefix}
    return {
        "kind": "simple",
        "supported": True,
        "workspacePrefix": workspace_prefix,
        "operator": match.group("operator") or "",
        "version": match.group("version"),
    }


def satisfies(version_text: str, range_text: str) -> bool | None:
    details = range_details(range_text)
    if not details["supported"]:
        return None
    if details["kind"] == "workspace-dynamic":
        return True
    version = Version.parse(version_text)
    anchor = Version.parse(details["version"])
    operator = details["operator"]
    relation = compare(version, anchor)
    if version.prerelease and not anchor.prerelease:
        return False
    if operator in {"", "="}:
        return relation == 0
    if operator == ">=":
        return relation >= 0
    if operator == ">":
        return relation > 0
    if operator == "<=":
        return relation <= 0
    if operator == "<":
        return relation < 0
    if operator == "~":
        return relation >= 0 and version.major == anchor.major and version.minor == anchor.minor
    if operator == "^":
        if anchor.major > 0:
            return relation >= 0 and version.major == anchor.major
        if anchor.minor > 0:
            return relation >= 0 and version.major == 0 and version.minor == anchor.minor
        return relation >= 0 and version.major == 0 and version.minor == 0 and version.patch == anchor.patch
    raise AssertionError(operator)


def suggested_range(range_text: str, version_text: str) -> str | None:
    details = range_details(range_text)
    if not details["supported"] or details["kind"] != "simple":
        return None
    return f"{details['workspacePrefix']}{details['operator']}{version_text}"


def package_index(discovery: dict[str, Any]) -> tuple[dict[str, dict[str, Any]], dict[str, set[str]]]:
    packages = discovery.get("packages")
    if not isinstance(packages, list):
        raise InputError("discovery packages must be an array")
    by_id: dict[str, dict[str, Any]] = {}
    aliases: dict[str, set[str]] = {}
    for package in packages:
        if not isinstance(package, dict) or not isinstance(package.get("id"), str):
            raise InputError("every package must have a string id")
        package_id = package["id"]
        by_id[package_id] = package
        for alias in {package_id, package.get("name"), package.get("dir")} - {None}:
            aliases.setdefault(str(alias), set()).add(package_id)
    return by_id, aliases


def resolve_versions(discovery: dict[str, Any], explicit_args: list[str]) -> tuple[dict[str, dict[str, Any]], list[dict[str, str]]]:
    by_id, aliases = package_index(discovery)
    explicit: dict[str, str] = {}
    for assignment in explicit_args:
        if "=" not in assignment:
            raise InputError(f"--version must be PACKAGE=SEMVER: {assignment}")
        selector, version = assignment.split("=", 1)
        Version.parse(version)
        matches = aliases.get(selector, set())
        if len(matches) != 1:
            qualifier = "unknown" if not matches else "ambiguous"
            raise InputError(f"{qualifier} package selector: {selector}")
        package_id = next(iter(matches))
        if package_id in explicit:
            raise InputError(f"duplicate version for package: {package_id}")
        explicit[package_id] = version

    targets = discovery.get("targets")
    if not isinstance(targets, list):
        raise InputError("discovery targets must be an array")
    planner_explicit = discovery.get("explicitVersion")
    if planner_explicit is not None:
        Version.parse(planner_explicit)
        if len(targets) != 1:
            raise InputError("planner explicitVersion requires exactly one target")
        explicit.setdefault(targets[0]["id"], planner_explicit)

    versions: dict[str, dict[str, Any]] = {}
    unresolved: list[dict[str, str]] = []
    for target in targets:
        package_id = target.get("id")
        if package_id not in by_id:
            raise InputError(f"target is absent from packages: {package_id}")
        current = target.get("version")
        if not isinstance(current, str):
            raise InputError(f"package has no version: {package_id}")
        Version.parse(current)
        planned: str | None
        source: str
        if discovery.get("beta"):
            planned = beta_version(current, explicit.get(package_id))
            source = "explicit-beta" if package_id in explicit else "beta-transition"
        elif package_id in explicit:
            planned = explicit[package_id]
            source = "explicit"
        else:
            planned = stable_promotion(current)
            source = "prerelease-promotion" if planned else "agent-decision"
        versions[package_id] = {"current": current, "planned": planned, "source": source, "resolved": planned is not None}
        if planned is None:
            unresolved.append({"package": package_id, "decision": "choose patch, minor, or major version"})
    for package_id, explicit_version in explicit.items():
        if package_id in versions:
            continue
        current = by_id[package_id].get("version")
        if not isinstance(current, str):
            raise InputError(f"package has no version: {package_id}")
        Version.parse(current)
        planned = beta_version(current, explicit_version) if discovery.get("beta") else explicit_version
        versions[package_id] = {
            "current": current,
            "planned": planned,
            "source": "cascade-explicit-beta" if discovery.get("beta") else "cascade-explicit",
            "resolved": True,
        }
    return versions, unresolved


def dependency_order(nodes: set[str], edges: list[dict[str, Any]]) -> tuple[list[str], list[list[str]]]:
    outgoing = {node: set() for node in nodes}
    indegree = {node: 0 for node in nodes}
    for edge in edges:
        dependent, dependency = edge.get("from"), edge.get("to")
        if dependent in nodes and dependency in nodes and dependent not in outgoing[dependency]:
            outgoing[dependency].add(dependent)
            indegree[dependent] += 1
    ready = sorted(node for node, count in indegree.items() if count == 0)
    ordered: list[str] = []
    while ready:
        node = ready.pop(0)
        ordered.append(node)
        for dependent in sorted(outgoing[node]):
            indegree[dependent] -= 1
            if indegree[dependent] == 0:
                ready.append(dependent)
                ready.sort()
    cycles: list[list[str]] = []
    stack: list[str] = []
    on_stack: set[str] = set()
    indexes: dict[str, int] = {}
    lowlinks: dict[str, int] = {}
    next_index = 0

    def visit(node: str) -> None:
        nonlocal next_index
        indexes[node] = lowlinks[node] = next_index
        next_index += 1
        stack.append(node)
        on_stack.add(node)
        for adjacent in sorted(outgoing[node]):
            if adjacent not in indexes:
                visit(adjacent)
                lowlinks[node] = min(lowlinks[node], lowlinks[adjacent])
            elif adjacent in on_stack:
                lowlinks[node] = min(lowlinks[node], indexes[adjacent])
        if lowlinks[node] != indexes[node]:
            return
        component: list[str] = []
        while True:
            adjacent = stack.pop()
            on_stack.remove(adjacent)
            component.append(adjacent)
            if adjacent == node:
                break
        component.sort()
        if len(component) > 1 or node in outgoing[node]:
            cycles.append(component)

    for node in sorted(nodes):
        if node not in indexes:
            visit(node)
    return ordered, sorted(cycles)


def finalize(discovery: dict[str, Any], explicit_args: list[str]) -> dict[str, Any]:
    if discovery.get("schemaVersion") != 2:
        raise InputError("discovery schemaVersion must be 2")
    versions, unresolved = resolve_versions(discovery, explicit_args)
    edges = discovery.get("dependencyEdges")
    if not isinstance(edges, list):
        raise InputError("discovery dependencyEdges must be an array")
    evaluated: list[dict[str, Any]] = []
    release_nodes = set(versions)
    for edge in edges:
        dependency_version = versions.get(edge.get("to"), {}).get("planned")
        if dependency_version is None:
            continue
        declared_range = edge.get("range")
        if not isinstance(declared_range, str):
            raise InputError("dependency edge range must be a string")
        satisfied = satisfies(dependency_version, declared_range)
        details = range_details(declared_range)
        item = {
            **edge,
            "plannedDependencyVersion": dependency_version,
            "rangeKind": details["kind"],
            "satisfied": satisfied,
            "suggestedRange": None,
            "decision": None,
        }
        if satisfied is False:
            release_nodes.add(edge["from"])
            if edge.get("type") == "peerDependencies":
                item["decision"] = "choose peer dependency range policy"
            elif details["kind"] == "simple":
                item["suggestedRange"] = suggested_range(declared_range, dependency_version)
            else:
                item["decision"] = "choose complex dependency range policy"
        elif satisfied is None:
            item["decision"] = "evaluate unsupported dependency range"
        evaluated.append(item)

    order, cycles = dependency_order(release_nodes, edges)
    if cycles:
        unresolved.append({"package": ",".join(cycles[0]), "decision": "resolve dependency cycle ordering"})
    return {
        "schemaVersion": 1,
        "discoverySchemaVersion": 2,
        "versions": versions,
        "workspaceEdges": evaluated,
        "unsatisfiedWorkspaceEdges": [edge for edge in evaluated if edge["satisfied"] is not True],
        "dependencyOrder": order,
        "dependencyCycles": cycles,
        "unresolvedDecisions": unresolved,
        "tagFacts": discovery.get("previousTags", {}),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--discovery", required=True, type=Path)
    parser.add_argument("--version", action="append", default=[], metavar="PACKAGE=SEMVER")
    args = parser.parse_args()
    try:
        discovery = json.loads(args.discovery.read_text(encoding="utf-8"))
        result = finalize(discovery, args.version)
    except (OSError, json.JSONDecodeError, InputError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 64
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

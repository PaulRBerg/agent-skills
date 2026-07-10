#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = ["PyYAML>=6.0.2"]
# ///
"""Audit Agent Skills catalogs and installed skill roots.

Usage:
  uv run scripts/skill-doctor.py [--root PATH ...] [--format text|json]
      [--fix-safe]
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from collections import Counter
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import yaml


SKILL_NAME_RE = re.compile(r"^[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$")
FRONTMATTER_RE = re.compile(r"\A---\s*\n(?P<body>.*?)\n---\s*(?:\n|\Z)", re.DOTALL)
TOP_LEVEL_FIELD_RE = re.compile(r"^(?P<key>[A-Za-z0-9_-]+):(?:\s|$)")
MARKDOWN_RESOURCE_LINK_RE = re.compile(r"\]\((?P<path>(?:references|scripts|assets)/[^)\s]+)\)")
UV_SCRIPT_REF_RE = re.compile(r"(?<![A-Za-z0-9_./-])uv run (?P<path>scripts/[A-Za-z0-9][A-Za-z0-9._/-]*)")
SEMVER_RE = re.compile(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")
OPENAI_ALLOW_RE = re.compile(r"^(\s*allow_implicit_invocation:\s*)(true|false)(\s*)$", re.MULTILINE)
COMPLETION_EVIDENCE_RE = re.compile(
    r"(?im)^##+[^\n]*\b(?:completion|verify|verification|validation|output|report|result|exit codes)\b"
    r"|\b(?:success means|complete when|completion requires|completion is|finish with)\b"
)
REQUIREMENT_RE = re.compile(r"\b(?:always|must(?!\s+not\b)|required to)\s+([^.!?\n]+)", re.IGNORECASE)
PROHIBITION_RE = re.compile(r"\b(?:do not|don't|never|must not|forbid(?:s|den)?)\s+([^.!?\n]+)", re.IGNORECASE)
STALE_MODEL_PINS = {"opus"}


@dataclass(frozen=True)
class Finding:
    code: str
    severity: str
    path: str
    line: int | None
    fixable: bool
    message: str


@dataclass(frozen=True)
class Fix:
    code: str
    path: str
    message: str


@dataclass(frozen=True)
class SkillFile:
    root: Path
    path: Path
    active: bool

    @property
    def directory(self) -> Path:
        return self.path.parent

    @property
    def directory_name(self) -> str:
        return self.path.parent.name


def main() -> int:
    parser = argparse.ArgumentParser(description="Audit Agent Skills catalogs and installed skill roots.")
    parser.add_argument("--root", action="append", default=[], help="Catalog or installed skill root to scan")
    parser.add_argument("--format", choices=("text", "json"), default="text")
    parser.add_argument("--fix-safe", action="store_true", help="Apply narrow openai.yaml metadata fixes")
    args = parser.parse_args()

    try:
        roots = normalize_roots(args.root or [os.curdir])
    except ValueError as error:
        print(f"skill-doctor: {error}", file=sys.stderr)
        return 2

    report = audit(roots, fix_safe=args.fix_safe)

    if args.format == "json":
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_text_report(report)

    if report["counts"]["fix_errors"] > 0:
        return 3
    if report["counts"]["findings"] > 0:
        return 1
    return 0


def normalize_roots(raw_roots: list[str]) -> list[Path]:
    roots: list[Path] = []
    for raw_root in raw_roots:
        root = Path(os.path.expanduser(raw_root)).resolve()
        if not root.exists():
            raise ValueError(f"root does not exist: {root}")
        if not root.is_dir():
            raise ValueError(f"root is not a directory: {root}")
        roots.append(root)
    return roots


def audit(roots: list[Path], *, fix_safe: bool) -> dict[str, Any]:
    findings: list[Finding] = []
    fixes: list[Fix] = []
    root_reports: list[dict[str, Any]] = []
    seen_skill_paths: set[Path] = set()

    for root in roots:
        skills = [skill for skill in discover_skills(root) if skill.path not in seen_skill_paths]
        seen_skill_paths.update(skill.path for skill in skills)

        for skill in skills:
            check_skill(skill, findings, fixes, fix_safe=fix_safe)

        check_readme(root, skills, findings)
        root_reports.append(
            {
                "path": str(root),
                "active_skills": sum(1 for skill in skills if skill.active),
                "readme": str(root / "README.md") if (root / "README.md").exists() else None,
            }
        )

    counts = Counter(finding.severity for finding in findings)
    report_counts = {
        "findings": len(findings),
        "errors": counts.get("error", 0),
        "warnings": counts.get("warning", 0),
        "fixable": sum(1 for finding in findings if finding.fixable),
        "fixes": len(fixes),
        "fix_errors": counts.get("fix-error", 0),
    }
    return {
        "roots": root_reports,
        "counts": report_counts,
        "findings": [asdict(finding) for finding in findings],
        "fixes": [asdict(fix) for fix in fixes],
    }


def discover_skills(root: Path) -> list[SkillFile]:
    skills: list[SkillFile] = []

    if (root / "SKILL.md").is_file():
        skills.append(SkillFile(root=root, path=(root / "SKILL.md").resolve(), active=True))

    for skill_path in child_skill_paths(root, "skills"):
        skills.append(SkillFile(root=root, path=skill_path.resolve(), active=True))

    if root.name == "skills":
        for skill_path in sorted(root.glob("*/SKILL.md")):
            skills.append(SkillFile(root=root, path=skill_path.resolve(), active=True))

    deduped: dict[Path, SkillFile] = {}
    for skill in skills:
        deduped.setdefault(skill.path, skill)
    return sorted(deduped.values(), key=lambda skill: str(skill.path))


def child_skill_paths(root: Path, child: str) -> list[Path]:
    directory = root / child
    if not directory.is_dir():
        return []
    return sorted(path for path in directory.glob("*/SKILL.md") if path.is_file())


def check_skill(skill: SkillFile, findings: list[Finding], fixes: list[Fix], *, fix_safe: bool) -> None:
    text = read_text(skill.path)
    if text is None:
        findings.append(finding("READ_ERROR", "error", skill.path, None, False, "could not read SKILL.md"))
        return

    frontmatter, field_lines, field_order = parse_frontmatter(skill.path, text, findings)
    if frontmatter is None:
        return

    check_frontmatter_fields(skill, frontmatter, field_lines, field_order, findings)
    check_openai_metadata(skill, frontmatter, findings, fixes, fix_safe=fix_safe)
    if skill.active:
        check_cli_version(skill, frontmatter, findings)
    check_resource_links(skill, text, findings)
    check_prompt_hygiene(skill, text, frontmatter, field_lines, findings)


def parse_frontmatter(
    path: Path,
    text: str,
    findings: list[Finding],
) -> tuple[dict[str, Any] | None, dict[str, int], list[str]]:
    match = FRONTMATTER_RE.match(text)
    if not match:
        findings.append(finding("FRONTMATTER_MISSING", "error", path, 1, False, "missing YAML frontmatter"))
        return None, {}, []

    frontmatter_text = match.group("body")
    try:
        parsed = yaml.safe_load(frontmatter_text) or {}
    except yaml.YAMLError as error:
        findings.append(finding("FRONTMATTER_INVALID", "error", path, 2, False, f"invalid YAML frontmatter: {error}"))
        return None, {}, []

    if not isinstance(parsed, dict):
        findings.append(finding("FRONTMATTER_INVALID", "error", path, 2, False, "frontmatter must be a mapping"))
        return None, {}, []

    field_lines: dict[str, int] = {}
    field_order: list[str] = []
    for offset, line in enumerate(frontmatter_text.splitlines(), start=2):
        match = TOP_LEVEL_FIELD_RE.match(line)
        if not match:
            continue
        key = match.group("key")
        field_lines.setdefault(key, offset)
        field_order.append(key)

    return parsed, field_lines, field_order


def check_frontmatter_fields(
    skill: SkillFile,
    frontmatter: dict[str, Any],
    field_lines: dict[str, int],
    field_order: list[str],
    findings: list[Finding],
) -> None:
    name = frontmatter.get("name")
    if not isinstance(name, str) or not name.strip():
        findings.append(finding("NAME_MISSING", "error", skill.path, field_lines.get("name", 2), False, "missing required name"))
    else:
        stripped_name = name.strip()
        if stripped_name != skill.directory_name:
            findings.append(
                finding(
                    "NAME_DIRECTORY_MISMATCH",
                    "error",
                    skill.path,
                    field_lines.get("name"),
                    False,
                    f"name {stripped_name!r} does not match directory {skill.directory_name!r}",
                )
            )
        if len(stripped_name) > 64 or not SKILL_NAME_RE.match(stripped_name) or "--" in stripped_name:
            findings.append(
                finding(
                    "NAME_INVALID",
                    "error",
                    skill.path,
                    field_lines.get("name"),
                    False,
                    "name must be 1-64 chars, lowercase alphanumeric plus hyphens, no leading/trailing/consecutive hyphens",
                )
            )

    description = frontmatter.get("description")
    if not isinstance(description, str) or not description.strip():
        findings.append(
            finding("DESCRIPTION_MISSING", "error", skill.path, field_lines.get("description", 2), False, "missing required description")
        )
    elif len(description) > 1024:
        findings.append(
            finding(
                "DESCRIPTION_TOO_LONG",
                "error",
                skill.path,
                field_lines.get("description"),
                False,
                f"description is {len(description)} chars; max is 1024",
            )
        )

    compatibility = frontmatter.get("compatibility")
    if compatibility is not None:
        if not isinstance(compatibility, str):
            findings.append(
                finding("COMPATIBILITY_INVALID", "error", skill.path, field_lines.get("compatibility"), False, "compatibility must be a string")
            )
        elif len(compatibility) > 500:
            findings.append(
                finding(
                    "COMPATIBILITY_TOO_LONG",
                    "error",
                    skill.path,
                    field_lines.get("compatibility"),
                    False,
                    f"compatibility is {len(compatibility)} chars; max is 500",
                )
            )

    expected_order = sorted(key for key in field_order if key != "description")
    if "description" in field_order:
        expected_order.append("description")
    if field_order != expected_order:
        findings.append(
            finding(
                "FRONTMATTER_FIELD_ORDER",
                "warning",
                skill.path,
                2,
                False,
                "frontmatter fields must be alphabetized with description last",
            )
        )


def check_openai_metadata(
    skill: SkillFile,
    frontmatter: dict[str, Any],
    findings: list[Finding],
    fixes: list[Fix],
    *,
    fix_safe: bool,
) -> None:
    disable_model_invocation = frontmatter.get("disable-model-invocation", False)
    if not isinstance(disable_model_invocation, bool):
        findings.append(
            finding(
                "DISABLE_MODEL_INVOCATION_INVALID",
                "error",
                skill.path,
                None,
                False,
                "disable-model-invocation must be true or false when present",
            )
        )
        return

    expected = not disable_model_invocation
    openai_path = skill.directory / "agents" / "openai.yaml"
    if not openai_path.exists():
        if fix_safe:
            try:
                openai_path.parent.mkdir(parents=True, exist_ok=True)
                openai_path.write_text(f"policy:\n  allow_implicit_invocation: {str(expected).lower()}\n", encoding="utf-8")
            except OSError as error:
                findings.append(
                    finding("OPENAI_METADATA_FIX_FAILED", "fix-error", openai_path, None, True, f"failed to create openai.yaml: {error}")
                )
            else:
                fixes.append(Fix("OPENAI_METADATA_CREATED", str(openai_path), "created agents/openai.yaml"))
            return
        findings.append(
            finding("OPENAI_METADATA_MISSING", "error", openai_path, None, True, "missing agents/openai.yaml")
        )
        return

    text = read_text(openai_path)
    if text is None:
        findings.append(finding("OPENAI_METADATA_READ_ERROR", "error", openai_path, None, False, "could not read agents/openai.yaml"))
        return

    try:
        parsed = yaml.safe_load(text) or {}
    except yaml.YAMLError as error:
        findings.append(finding("OPENAI_METADATA_INVALID", "error", openai_path, 1, False, f"invalid YAML: {error}"))
        return

    actual = None
    if isinstance(parsed, dict):
        policy = parsed.get("policy")
        if isinstance(policy, dict):
            actual = policy.get("allow_implicit_invocation")

    if not isinstance(actual, bool):
        findings.append(
            finding(
                "OPENAI_POLICY_MISSING",
                "error",
                openai_path,
                line_number_for_pattern(text, "allow_implicit_invocation"),
                False,
                "missing boolean policy.allow_implicit_invocation",
            )
        )
        return

    if actual == expected:
        return

    if fix_safe:
        if not OPENAI_ALLOW_RE.search(text):
            findings.append(
                finding(
                    "OPENAI_METADATA_FIX_FAILED",
                    "fix-error",
                    openai_path,
                    None,
                    True,
                    "could not locate allow_implicit_invocation line to update",
                )
            )
            return
        try:
            openai_path.write_text(OPENAI_ALLOW_RE.sub(rf"\g<1>{str(expected).lower()}\g<3>", text, count=1), encoding="utf-8")
        except OSError as error:
            findings.append(
                finding("OPENAI_METADATA_FIX_FAILED", "fix-error", openai_path, None, True, f"failed to update openai.yaml: {error}")
            )
        else:
            fixes.append(
                Fix(
                    "OPENAI_POLICY_UPDATED",
                    str(openai_path),
                    f"updated allow_implicit_invocation to {str(expected).lower()}",
                )
            )
        return

    findings.append(
        finding(
            "OPENAI_POLICY_MISMATCH",
            "error",
            openai_path,
            line_number_for_pattern(text, "allow_implicit_invocation"),
            True,
            f"allow_implicit_invocation is {str(actual).lower()}, expected {str(expected).lower()}",
        )
    )


def check_cli_version(skill: SkillFile, frontmatter: dict[str, Any], findings: list[Finding]) -> None:
    name = frontmatter.get("name")
    if not isinstance(name, str) or not name.startswith("cli-"):
        return

    version_path = skill.directory / "references" / "version.txt"
    if not version_path.exists():
        findings.append(
            finding("CLI_VERSION_MISSING", "error", version_path, None, False, "cli-* skill must maintain references/version.txt")
        )
        return

    text = read_text(version_path)
    if text is None:
        findings.append(finding("CLI_VERSION_READ_ERROR", "error", version_path, None, False, "could not read references/version.txt"))
        return

    lines = text.splitlines()
    if len(lines) != 1 or not SEMVER_RE.match(lines[0]):
        findings.append(
            finding(
                "CLI_VERSION_INVALID",
                "error",
                version_path,
                1,
                False,
                "references/version.txt must contain exactly one normalized semver line",
            )
        )


def check_resource_links(skill: SkillFile, text: str, findings: list[Finding]) -> None:
    matches = list(MARKDOWN_RESOURCE_LINK_RE.finditer(text)) + list(UV_SCRIPT_REF_RE.finditer(text))
    for match in matches:
        raw_ref = match.group("path").rstrip(".,;:")
        raw_ref = raw_ref.split("#", 1)[0].split("?", 1)[0]
        raw_ref = raw_ref.rstrip(")]}'\"")
        if not raw_ref or raw_ref.endswith("/") or any(char in raw_ref for char in "*{}"):
            continue
        target = skill.directory / raw_ref
        if target.exists():
            continue
        findings.append(
            finding(
                "RESOURCE_LINK_MISSING",
                "error",
                skill.path,
                line_number_at_offset(text, match.start()),
                False,
                f"referenced resource does not exist: {raw_ref}",
            )
        )


def check_prompt_hygiene(
    skill: SkillFile,
    text: str,
    frontmatter: dict[str, Any],
    field_lines: dict[str, int],
    findings: list[Finding],
) -> None:
    model = frontmatter.get("model")
    if isinstance(model, str) and model.lower() in STALE_MODEL_PINS:
        findings.append(
            finding(
                "STALE_MODEL_PIN",
                "warning",
                skill.path,
                field_lines.get("model"),
                False,
                f"model pin {model!r} is a stale alias; verify that an explicit pin is still needed",
            )
        )

    if not COMPLETION_EVIDENCE_RE.search(text):
        findings.append(
            finding(
                "COMPLETION_EVIDENCE_MISSING",
                "warning",
                skill.path,
                None,
                False,
                "skill has no explicit completion, verification, validation, output, or report contract",
            )
        )

    for match in MARKDOWN_RESOURCE_LINK_RE.finditer(text):
        raw_ref = match.group("path").split("#", 1)[0].split("?", 1)[0]
        target = skill.directory / raw_ref
        if target.suffix.lower() != ".md" or not target.is_file():
            continue
        line_start = text.rfind("\n", 0, match.start()) + 1
        line_end = text.find("\n", match.end())
        line = text[line_start : None if line_end < 0 else line_end].lower()
        unconditional = "mandatory" in line or "always read" in line or ("read" in line and "before" in line)
        if not unconditional:
            continue
        reference_text = read_text(target)
        if reference_text is None or len(reference_text.splitlines()) < 400:
            continue
        findings.append(
            finding(
                "UNCONDITIONAL_REFERENCE_OVERSIZED",
                "warning",
                skill.path,
                line_number_at_offset(text, match.start()),
                False,
                f"unconditional reference has {len(reference_text.splitlines())} lines: {raw_ref}",
            )
        )

    requirements = [(match, normalized_clause(match.group(1))) for match in REQUIREMENT_RE.finditer(text)]
    prohibitions = [(match, normalized_clause(match.group(1))) for match in PROHIBITION_RE.finditer(text)]
    for required_match, required in requirements:
        if len(required) < 3:
            continue
        for prohibited_match, prohibited in prohibitions:
            if len(prohibited) < 3:
                continue
            overlap = len(required & prohibited) / len(required | prohibited)
            if overlap < 0.75:
                continue
            findings.append(
                finding(
                    "CONFLICTING_AUTHORITY",
                    "warning",
                    skill.path,
                    line_number_at_offset(text, min(required_match.start(), prohibited_match.start())),
                    False,
                    "similar action appears in both requirement and prohibition language; review authority",
                )
            )
            return


def normalized_clause(value: str) -> set[str]:
    words = re.findall(r"[a-z0-9_-]+", value.lower())[:10]
    return {word for word in words if word not in {"the", "a", "an", "to", "of", "and", "or", "when", "if"}}


def check_readme(root: Path, skills: list[SkillFile], findings: list[Finding]) -> None:
    if not (root / "skills").is_dir():
        return

    readme = root / "README.md"
    if not readme.exists():
        findings.append(finding("README_MISSING", "error", readme, None, False, "catalog root is missing README.md"))
        return

    text = read_text(readme)
    if text is None:
        findings.append(finding("README_READ_ERROR", "error", readme, None, False, "could not read README.md"))
        return

    table_names = parse_readme_skill_table(text)
    active_names = {skill.directory_name for skill in skills if skill.active}

    for name in sorted(active_names - set(table_names)):
        findings.append(finding("README_SKILL_MISSING", "error", readme, None, False, f"active skill missing from README table: {name}"))

    for name, line in sorted(table_names.items()):
        if name in active_names:
            continue
        findings.append(finding("README_LISTS_MISSING", "error", readme, line, False, f"README lists missing skill: {name}"))


def parse_readme_skill_table(text: str) -> dict[str, int]:
    names: dict[str, int] = {}
    in_skills = False
    for line_number, line in enumerate(text.splitlines(), start=1):
        if line.startswith("## "):
            in_skills = line.strip() == "## Skills"
            continue
        if not in_skills or not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) < 2:
            continue
        name = cells[0]
        if name in {"Skill", ""} or set(name) <= {"-"}:
            continue
        if SKILL_NAME_RE.match(name):
            names.setdefault(name, line_number)
    return names


def read_text(path: Path) -> str | None:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        try:
            return path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            return None
    except OSError:
        return None


def finding(code: str, severity: str, path: Path, line: int | None, fixable: bool, message: str) -> Finding:
    return Finding(code=code, severity=severity, path=str(path), line=line, fixable=fixable, message=message)


def line_number_for_pattern(text: str, pattern: str) -> int | None:
    offset = text.find(pattern)
    if offset < 0:
        return None
    return line_number_at_offset(text, offset)


def line_number_at_offset(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def print_text_report(report: dict[str, Any]) -> None:
    counts = report["counts"]
    print(
        "skill-doctor: "
        f"{counts['errors']} error(s), {counts['warnings']} warning(s), "
        f"{counts['fixes']} fix(es)"
    )

    if report["roots"]:
        print("\nRoots:")
        for root in report["roots"]:
            print(f"- {root['path']}: {root['active_skills']} active")

    if report["fixes"]:
        print("\nFixes:")
        for fix_item in report["fixes"]:
            print(f"- {fix_item['code']}: {fix_item['path']}: {fix_item['message']}")

    if report["findings"]:
        print("\nFindings:")
        for item in report["findings"]:
            location = item["path"]
            if item["line"] is not None:
                location = f"{location}:{item['line']}"
            fixable = " fixable" if item["fixable"] else ""
            print(f"- [{item['severity']}] {item['code']}{fixable}: {location}: {item['message']}")


if __name__ == "__main__":
    raise SystemExit(main())

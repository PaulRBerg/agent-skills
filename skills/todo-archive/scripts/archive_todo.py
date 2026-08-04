#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as dt
import pathlib
import re
import subprocess
import sys
from dataclasses import dataclass, field
from typing import Iterable, Optional


TASK_RE = re.compile(r"^(\s*)([-*+])\s+\[([ xX])\]\s+(.*?)(\r?\n)?$")
HEADING_RE = re.compile(r"^(#{1,6})\s+\S.*$")
BLANK_RE = re.compile(r"^\s*$")


@dataclass
class Node:
    kind: str
    line: str = ""
    level: int = 0
    indent: int = -1
    checked: Optional[bool] = None
    archive: Optional[bool] = None
    marker: str = "-"
    text: str = ""
    children: list["Node"] = field(default_factory=list)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Archive checked Markdown tasks from TODO.md into .ai/todos/."
    )
    parser.add_argument("--root", default=None, help="repository root; defaults to git root or cwd")
    parser.add_argument("--date", default=None, help="YYYY-MM-DD or YYYY_MM_DD; defaults to today")
    parser.add_argument("--dry-run", action="store_true", help="print rendered outputs without writing")
    parser.add_argument(
        "--hint",
        default=None,
        help="archive only the section whose heading contains this text (case-insensitive)",
    )
    args = parser.parse_args()

    root = resolve_root(args.root)
    todo_path = root / "TODO.md"
    if not todo_path.exists():
        print(f"TODO.md not found: {todo_path}", file=sys.stderr)
        return 1

    archive_date = normalize_date(args.date)
    archive_path = root / ".ai" / "todos" / archive_date[:7] / f"{archive_date[-2:]}.md"

    source = todo_path.read_text(encoding="utf-8")
    tree = parse_document(source.splitlines(keepends=True))

    matched = collect_matching_headings(tree, args.hint)
    if args.hint is not None and not matched:
        print(f"No heading matched hint {args.hint!r} in {todo_path}", file=sys.stderr)
        headings = [heading_text(node) for node in iter_headings(tree)]
        if headings:
            print("Available sections:", file=sys.stderr)
            for heading in headings:
                print(f"  - {heading}", file=sys.stderr)
        return 1

    mark_tasks(tree, args.hint, in_section=args.hint is None)
    archive_count = count_tasks(tree, True)
    remaining_count = count_tasks(tree, False)

    if archive_count == 0:
        scope = f" in section matching {args.hint!r}" if args.hint else ""
        print(f"No checked tasks found{scope} in {todo_path}; no archive written.")
        print(f"Tasks remaining: {remaining_count}")
        return 0

    new_archive_text = finalize(render(tree, True), fallback_heading="# TODO\n")
    existing_archive_text = archive_path.read_text(encoding="utf-8") if archive_path.exists() else None
    archive_text = (
        merge_archive_text(existing_archive_text, new_archive_text)
        if existing_archive_text is not None
        else new_archive_text
    )
    remaining_text = finalize(render(tree, False), fallback_heading=first_heading(source) or "# TODO\n")

    if args.dry_run:
        print(f"TODO: {todo_path}")
        print(f"ARCHIVE: {archive_path}")
        if matched:
            print(f"Section(s): {', '.join(matched)}")
        if existing_archive_text is not None:
            print(f"NOTE: {archive_path.name} exists; this batch will be merged into it.")
        print(f"Checked tasks to archive: {archive_count}")
        print(f"Tasks remaining: {remaining_count}")
        print("\n--- TODO.md ---")
        print(remaining_text, end="")
        print("\n--- archive ---")
        print(archive_text, end="")
        return 0

    archive_path.parent.mkdir(parents=True, exist_ok=True)
    archive_path.write_text(archive_text, encoding="utf-8")
    todo_path.write_text(remaining_text, encoding="utf-8")

    print(f"Archived checked tasks: {archive_count}")
    if matched:
        print(f"Section(s): {', '.join(matched)}")
    print(f"Tasks remaining: {remaining_count}")
    print(f"Rewrote: {todo_path}")
    if existing_archive_text is not None:
        print(f"Merged: {archive_path}")
    else:
        print(f"Created: {archive_path}")
    return 0


def resolve_root(root_arg: Optional[str]) -> pathlib.Path:
    if root_arg:
        return pathlib.Path(root_arg).expanduser().resolve()

    try:
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return pathlib.Path.cwd().resolve()

    return pathlib.Path(result.stdout.strip()).resolve()


def normalize_date(value: Optional[str]) -> str:
    if value is None:
        return dt.date.today().isoformat()

    normalized = value.replace("_", "-")
    try:
        dt.date.fromisoformat(normalized)
    except ValueError:
        raise SystemExit("--date must be YYYY-MM-DD or YYYY_MM_DD")
    return normalized


def merge_archive_text(existing_text: str, new_text: str) -> str:
    """Append a newly archived batch, omitting an identical leading H1.

    Daily archives normally begin with the same top-level heading as the source
    TODO. Keeping it once makes repeated same-day archives read as one document.
    Other content is intentionally left untouched and batches remain ordered by
    archive time.
    """
    existing_heading = leading_h1(existing_text)
    new_heading = leading_h1(new_text)
    if existing_heading is not None and new_heading is not None:
        existing_line, _ = existing_heading
        new_line, new_end = new_heading
        if existing_line == new_line:
            new_text = new_text[new_end:].lstrip("\r\n")

    return existing_text.rstrip("\r\n") + "\n\n" + new_text.lstrip("\r\n")


def leading_h1(text: str) -> Optional[tuple[str, int]]:
    """Return the first non-blank H1 line and its end offset, if present."""
    offset = 0
    for line in text.splitlines(keepends=True):
        end = offset + len(line)
        if BLANK_RE.match(line):
            offset = end
            continue
        if re.match(r"^#\s+\S.*(?:\r?\n)?$", line):
            return line.rstrip("\r\n"), end
        return None
    return None


def parse_document(lines: Iterable[str]) -> Node:
    root = Node("root")
    stack: list[Node] = [root]

    for line in lines:
        heading_match = HEADING_RE.match(line)
        if heading_match:
            level = len(heading_match.group(1))
            while len(stack) > 1 and (
                stack[-1].kind == "task"
                or (stack[-1].kind == "heading" and stack[-1].level >= level)
            ):
                stack.pop()
            node = Node("heading", line=line, level=level)
            stack[-1].children.append(node)
            stack.append(node)
            continue

        task_match = TASK_RE.match(line)
        if task_match:
            indent_text, marker, box, text, _newline = task_match.groups()
            indent = len(indent_text.expandtabs(4))
            while stack[-1].kind == "task" and stack[-1].indent >= indent:
                stack.pop()
            node = Node(
                "task",
                line=line,
                indent=indent,
                checked=box.lower() == "x",
                marker=marker,
                text=text,
            )
            stack[-1].children.append(node)
            stack.append(node)
            continue

        if stack[-1].kind == "task" and not is_task_continuation(line, stack[-1].indent):
            while stack[-1].kind == "task":
                stack.pop()

        stack[-1].children.append(Node("raw", line=line))

    return root


def is_task_continuation(line: str, task_indent: int) -> bool:
    if BLANK_RE.match(line):
        return True
    return leading_spaces(line) > task_indent


def leading_spaces(line: str) -> int:
    return len(line.expandtabs(4)) - len(line.expandtabs(4).lstrip(" "))


def heading_text(node: Node) -> str:
    return re.sub(r"^#{1,6}\s+", "", node.line).strip()


def iter_headings(node: Node) -> Iterable[Node]:
    for child in node.children:
        if child.kind == "heading":
            yield child
        yield from iter_headings(child)


def heading_matches(node: Node, hint: str) -> bool:
    return hint.lower() in heading_text(node).lower()


def collect_matching_headings(node: Node, hint: Optional[str]) -> list[str]:
    if hint is None:
        return []
    return [heading_text(h) for h in iter_headings(node) if heading_matches(h, hint)]


def mark_tasks(node: Node, hint: Optional[str], in_section: bool) -> None:
    """Mark every task with whether it belongs in the archive output.

    A task is archived when it is checked and within scope. Scope is the whole
    document when no hint is given, or the subtree of any heading matching the
    hint otherwise. Once inside a matched section, all descendants stay in scope.
    """
    if node.kind == "task":
        node.archive = bool(node.checked) and in_section

    child_in_section = in_section
    if node.kind == "heading" and hint is not None and heading_matches(node, hint):
        child_in_section = True

    for child in node.children:
        mark_tasks(child, hint, child_in_section)


def count_tasks(node: Node, archive: bool) -> int:
    total = 1 if node.kind == "task" and node.archive is archive else 0
    return total + sum(count_tasks(child, archive) for child in node.children)


def has_task(node: Node, archive: bool) -> bool:
    return count_tasks(node, archive) > 0


def render(node: Node, archive: bool) -> str:
    if node.kind == "root":
        if not archive:
            return "".join(render(child, archive) for child in node.children)
        return "".join(render(child, archive) for child in node.children if has_task(child, archive))

    if node.kind == "heading":
        if not archive:
            children = "".join(
                render_child(child, archive, parent_is_rendered=True) for child in node.children
            )
            return node.line + children if children.strip() else ""
        if not has_task(node, archive):
            return ""
        return node.line + "".join(render_child(child, archive, parent_is_rendered=True) for child in node.children)

    if node.kind == "task":
        if not has_task(node, archive):
            return ""
        if node.archive is archive:
            first_line = node.line
        else:
            first_line = context_line(node)
        return first_line + "".join(render_child(child, archive, parent_is_rendered=True) for child in node.children)

    if node.kind == "raw":
        return node.line

    raise AssertionError(f"unknown node kind: {node.kind}")


def render_child(node: Node, archive: bool, parent_is_rendered: bool) -> str:
    if node.kind == "raw":
        return node.line if parent_is_rendered else ""
    return render(node, archive)


def context_line(node: Node) -> str:
    newline = "\n" if node.line.endswith("\n") else ""
    return f"{' ' * node.indent}{node.marker} {node.text}{newline}"


def finalize(text: str, fallback_heading: str) -> str:
    lines = text.splitlines(keepends=True)

    while lines and BLANK_RE.match(lines[0]):
        lines.pop(0)
    while lines and BLANK_RE.match(lines[-1]):
        lines.pop()

    cleaned: list[str] = []
    blank_count = 0
    for line in lines:
        if BLANK_RE.match(line):
            blank_count += 1
            if blank_count <= 2:
                cleaned.append(line)
        else:
            blank_count = 0
            cleaned.append(line)

    spaced: list[str] = []
    for line in cleaned:
        if HEADING_RE.match(line) and spaced and not BLANK_RE.match(spaced[-1]):
            spaced.append("\n")
        spaced.append(line)

    text = "".join(spaced)
    if not text.strip():
        text = fallback_heading
    if not text.endswith("\n"):
        text += "\n"
    return text


def first_heading(source: str) -> Optional[str]:
    for line in source.splitlines(keepends=True):
        if HEADING_RE.match(line):
            return line if line.endswith("\n") else line + "\n"
    return None


if __name__ == "__main__":
    raise SystemExit(main())

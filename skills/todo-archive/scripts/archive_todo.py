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
        "--force",
        action="store_true",
        help="overwrite the dated archive in place instead of rolling over to a timestamped file",
    )
    args = parser.parse_args()

    root = resolve_root(args.root)
    todo_path = root / "TODO.md"
    if not todo_path.exists():
        print(f"TODO.md not found: {todo_path}", file=sys.stderr)
        return 1

    archive_date = normalize_date(args.date)
    dated_path = root / ".ai" / "todos" / f"TODO_UNTIL_{archive_date}.md"

    source = todo_path.read_text(encoding="utf-8")
    tree = parse_document(source.splitlines(keepends=True))
    checked_count = count_tasks(tree, True)
    unchecked_count = count_tasks(tree, False)

    if checked_count == 0:
        print(f"No checked tasks found in {todo_path}; no archive written.")
        print(f"Unchecked tasks remaining: {unchecked_count}")
        return 0

    archive_path, rolled_over = resolve_archive_path(dated_path, archive_date, force=args.force)

    archive_text = finalize(render(tree, True), fallback_heading="# TODO\n")
    remaining_text = finalize(render(tree, False), fallback_heading=first_heading(source) or "# TODO\n")

    if args.dry_run:
        print(f"TODO: {todo_path}")
        print(f"ARCHIVE: {archive_path}")
        if rolled_over:
            print(f"NOTE: {dated_path.name} exists; rolling this batch over to a timestamped file.")
        print(f"Checked tasks to archive: {checked_count}")
        print(f"Unchecked tasks remaining: {unchecked_count}")
        print("\n--- TODO.md ---")
        print(remaining_text, end="")
        print("\n--- archive ---")
        print(archive_text, end="")
        return 0

    archive_path.parent.mkdir(parents=True, exist_ok=True)
    archive_path.write_text(archive_text, encoding="utf-8")
    todo_path.write_text(remaining_text, encoding="utf-8")

    print(f"Archived checked tasks: {checked_count}")
    print(f"Unchecked tasks remaining: {unchecked_count}")
    print(f"Rewrote: {todo_path}")
    if rolled_over:
        print(f"Kept existing archive: {dated_path}")
        print(f"Created timestamped archive: {archive_path}")
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
        return dt.date.today().strftime("%Y_%m_%d")

    normalized = value.replace("-", "_")
    try:
        dt.datetime.strptime(normalized, "%Y_%m_%d")
    except ValueError:
        raise SystemExit("--date must be YYYY-MM-DD or YYYY_MM_DD")
    return normalized


def resolve_archive_path(
    dated_path: pathlib.Path, archive_date: str, force: bool
) -> tuple[pathlib.Path, bool]:
    """Pick the archive file to write and report whether it rolled over.

    Default to the date-only file. When it already exists and we are not forcing
    an in-place overwrite, roll the new batch over to a distinct timestamped
    sibling (``TODO_UNTIL_<date>_<HHMM>.md``) so the earlier batch is never
    clobbered. Append a counter on the rare same-minute collision.
    """
    if force or not dated_path.exists():
        return dated_path, False

    stamp = dt.datetime.now().strftime("%H%M")
    todos_dir = dated_path.parent
    candidate = todos_dir / f"TODO_UNTIL_{archive_date}_{stamp}.md"
    counter = 2
    while candidate.exists():
        candidate = todos_dir / f"TODO_UNTIL_{archive_date}_{stamp}_{counter}.md"
        counter += 1
    return candidate, True


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


def count_tasks(node: Node, checked: bool) -> int:
    total = 1 if node.kind == "task" and node.checked is checked else 0
    return total + sum(count_tasks(child, checked) for child in node.children)


def has_task(node: Node, checked: bool) -> bool:
    return count_tasks(node, checked) > 0


def render(node: Node, checked: bool) -> str:
    if node.kind == "root":
        return "".join(render(child, checked) for child in node.children if has_task(child, checked))

    if node.kind == "heading":
        if not has_task(node, checked):
            return ""
        return node.line + "".join(render_child(child, checked, parent_is_rendered=True) for child in node.children)

    if node.kind == "task":
        if not has_task(node, checked):
            return ""
        if node.checked is checked:
            first_line = node.line
        else:
            first_line = context_line(node)
        return first_line + "".join(render_child(child, checked, parent_is_rendered=True) for child in node.children)

    if node.kind == "raw":
        return node.line

    raise AssertionError(f"unknown node kind: {node.kind}")


def render_child(node: Node, checked: bool, parent_is_rendered: bool) -> str:
    if node.kind == "raw":
        return node.line if parent_is_rendered else ""
    return render(node, checked)


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

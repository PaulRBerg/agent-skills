from __future__ import annotations

import importlib.util
import pathlib
import subprocess
import sys
import tempfile
import unittest


REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "skills" / "todo-archive" / "scripts" / "archive_todo.py"
SPEC = importlib.util.spec_from_file_location("archive_todo", SCRIPT_PATH)
assert SPEC is not None and SPEC.loader is not None
archive_todo = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = archive_todo
SPEC.loader.exec_module(archive_todo)


class RenderTests(unittest.TestCase):
    def test_remaining_todo_preserves_task_free_sections(self) -> None:
        source = """# TODO

- [x] Archive me

## Fixes

### Fix this 1

Explanation that must not be removed.

### Fix this 2

More non-task content.
"""

        tree = archive_todo.parse_document(source.splitlines(keepends=True))
        archive_todo.mark_tasks(tree, hint=None, in_section=True)

        remaining = archive_todo.finalize(
            archive_todo.render(tree, archive=False), fallback_heading="# TODO\n"
        )

        self.assertEqual(
            remaining,
            """# TODO

## Fixes

### Fix this 1

Explanation that must not be removed.

### Fix this 2

More non-task content.
""",
        )


class ArchiveCommandTests(unittest.TestCase):
    def run_helper(self, root: pathlib.Path, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT_PATH), "--root", str(root), *args],
            check=False,
            capture_output=True,
            text=True,
        )

    def test_archives_into_month_directories_for_both_date_formats(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = pathlib.Path(temp_dir).resolve()
            todo_path = root / "TODO.md"
            todo_path.write_text("# TODO\n\n- [x] First\n- [ ] Keep\n", encoding="utf-8")

            result = self.run_helper(root, "--date", "2026-07-06")

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue((root / ".ai/todos/2026-07/06.md").exists())
            self.assertFalse((root / ".ai/todos/TODO_UNTIL_2026_07_06.md").exists())
            self.assertEqual(todo_path.read_text(encoding="utf-8"), "# TODO\n\n- [ ] Keep\n")

            todo_path.write_text("# TODO\n\n- [x] Second\n", encoding="utf-8")
            result = self.run_helper(root, "--date", "2026_08_07")

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                (root / ".ai/todos/2026-08/07.md").read_text(encoding="utf-8"),
                "# TODO\n\n- [x] Second\n",
            )

    def test_same_day_runs_append_to_one_archive_with_one_matching_h1(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = pathlib.Path(temp_dir).resolve()
            todo_path = root / "TODO.md"
            todo_path.write_text("# TODO\n\n- [x] First\n", encoding="utf-8")

            first = self.run_helper(root, "--date", "2026-07-06")
            todo_path.write_text("# TODO\n\n- [x] Second\n", encoding="utf-8")
            second = self.run_helper(root, "--date", "2026-07-06")

            archive_path = root / ".ai/todos/2026-07/06.md"
            self.assertEqual(first.returncode, 0, first.stderr)
            self.assertEqual(second.returncode, 0, second.stderr)
            self.assertIn(f"Merged: {archive_path}", second.stdout)
            self.assertEqual(
                archive_path.read_text(encoding="utf-8"),
                "# TODO\n\n- [x] First\n\n- [x] Second\n",
            )
            self.assertEqual(list(archive_path.parent.glob("*.md")), [archive_path])

    def test_dry_run_renders_merged_archive_without_writing(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = pathlib.Path(temp_dir).resolve()
            todo_path = root / "TODO.md"
            archive_path = root / ".ai/todos/2026-07/06.md"
            archive_path.parent.mkdir(parents=True)
            archive_path.write_text("# TODO\n\n- [x] First\n", encoding="utf-8")
            source = "# TODO\n\n- [x] Second\n"
            todo_path.write_text(source, encoding="utf-8")

            result = self.run_helper(root, "--date", "2026-07-06", "--dry-run")

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("NOTE: 06.md exists; this batch will be merged into it.", result.stdout)
            self.assertIn("# TODO\n\n- [x] First\n\n- [x] Second\n", result.stdout)
            self.assertEqual(archive_path.read_text(encoding="utf-8"), "# TODO\n\n- [x] First\n")
            self.assertEqual(todo_path.read_text(encoding="utf-8"), source)

    def test_no_checked_tasks_and_unknown_hint_remain_noops(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = pathlib.Path(temp_dir).resolve()
            todo_path = root / "TODO.md"
            source = "# TODO\n\n## Current\n\n- [ ] Keep\n"
            todo_path.write_text(source, encoding="utf-8")

            no_tasks = self.run_helper(root, "--date", "2026-07-06")
            unknown_hint = self.run_helper(root, "--hint", "missing", "--date", "2026-07-06")

            self.assertEqual(no_tasks.returncode, 0, no_tasks.stderr)
            self.assertIn("No checked tasks found", no_tasks.stdout)
            self.assertEqual(unknown_hint.returncode, 1)
            self.assertIn("Available sections:", unknown_hint.stderr)
            self.assertEqual(todo_path.read_text(encoding="utf-8"), source)
            self.assertFalse((root / ".ai/todos").exists())


class MergeArchiveTests(unittest.TestCase):
    def test_keeps_distinct_leading_h1s(self) -> None:
        merged = archive_todo.merge_archive_text(
            "# Earlier\n\n- [x] First\n", "# TODO\n\n- [x] Second\n"
        )

        self.assertEqual(
            merged,
            "# Earlier\n\n- [x] First\n\n# TODO\n\n- [x] Second\n",
        )

if __name__ == "__main__":
    unittest.main()

from __future__ import annotations

import importlib.util
import pathlib
import sys
import unittest


SCRIPT_PATH = pathlib.Path(__file__).with_name("archive_todo.py")
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


if __name__ == "__main__":
    unittest.main()

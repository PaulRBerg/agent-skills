from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("parse-taze-plan.py")
SPEC = importlib.util.spec_from_file_location("parse_taze_plan", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ParseTazePlanTests(unittest.TestCase):
    def test_parses_and_prioritizes_updates(self) -> None:
        entries = MODULE.parse(
            """repo - 1 major, 1 minor, 1 patch
 react dependencies ^18.2.0 → ^19.0.0
 eslint devDependencies ^9.1.0 → ^9.2.0
 lodash dependencies 4.17.20 → 4.17.21
"""
        )
        self.assertEqual([item["action"] for item in entries], ["review-major", "apply", "skip-fixed"])
        self.assertEqual(entries[0]["package"], "react")

    def test_ignores_non_update_output(self) -> None:
        self.assertEqual(MODULE.parse("dependencies are already up-to-date\n"), [])


if __name__ == "__main__":
    unittest.main()

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "skills" / "node-deps-bumper" / "scripts" / "parse-taze-plan.py"
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

    def test_classifies_partial_numeric_ranges_and_preserves_them(self) -> None:
        entries = MODULE.parse(
            """alpha dependencies ^3.5 → ^3.9.10
beta dependencies ^5.1 → ^6.2.5
fixed dependencies 4.17.20 → 4.17.21
"""
        )
        by_package = {item["package"]: item for item in entries}
        self.assertEqual(
            (by_package["alpha"]["current"], by_package["alpha"]["type"], by_package["alpha"]["action"]),
            ("^3.5", "minor", "apply"),
        )
        self.assertEqual(
            (
                by_package["beta"]["current"],
                by_package["beta"]["available"],
                by_package["beta"]["type"],
                by_package["beta"]["action"],
            ),
            ("^5.1", "^6.2.5", "major", "review-major"),
        )
        self.assertEqual(by_package["fixed"]["action"], "skip-fixed")

    def test_wildcard_range_with_unknown_transition_requires_review(self) -> None:
        entries = MODULE.parse("pkg dependencies ^5.x → ^5.2.0\n")
        self.assertEqual(len(entries), 1)
        self.assertEqual((entries[0]["type"], entries[0]["action"]), ("unknown", "review"))


if __name__ == "__main__":
    unittest.main()

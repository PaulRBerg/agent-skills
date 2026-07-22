from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("update-bun-catalogs.py")


class UpdateBunCatalogsTests(unittest.TestCase):
    def run_helper(self, root: Path, plan: Path, *extra: str, check: bool = True):
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(root), "--plan", str(plan), "--include", "react,jest", *extra],
            text=True,
            capture_output=True,
        )
        if check:
            self.assertEqual(result.returncode, 0, result.stderr)
        return result

    def test_preview_write_prefixes_and_multiple_occurrences(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "package.json"
            original = {
                "name": "demo",
                "workspaces": {
                    "packages": ["packages/*"],
                    "catalog": {"react": "^18.2.0"},
                    "catalogs": {"testing": {"react": "~18.2.0", "jest": "29.0.0"}},
                },
            }
            package.write_text(json.dumps(original), encoding="utf-8")
            plan = root / "plan.json"
            plan.write_text(json.dumps({"updates": [
                {"package": "react", "current": "18.2.0", "available": "19.0.0"},
                {"package": "jest", "current": "29.0.0", "available": "30.0.0"},
            ]}), encoding="utf-8")
            preview = json.loads(self.run_helper(root, plan).stdout)
            self.assertFalse(preview["wrote"])
            self.assertEqual(json.loads(package.read_text()), original)
            written = json.loads(self.run_helper(root, plan, "--write").stdout)
            self.assertTrue(written["wrote"])
            updated = json.loads(package.read_text())
            self.assertEqual(updated["workspaces"]["catalog"]["react"], "^19.0.0")
            self.assertEqual(updated["workspaces"]["catalogs"]["testing"]["react"], "~19.0.0")
            self.assertEqual(updated["workspaces"]["catalogs"]["testing"]["jest"], "30.0.0")

    def test_stale_plan_fails_without_writing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "package.json"
            package.write_text(json.dumps({"workspaces": {"catalog": {"react": "^18.3.0", "jest": "29.0.0"}}}))
            before = package.read_bytes()
            plan = root / "plan.json"
            plan.write_text(json.dumps({"updates": [
                {"package": "react", "current": "18.2.0", "available": "19.0.0"},
                {"package": "jest", "current": "29.0.0", "available": "30.0.0"},
            ]}))
            result = self.run_helper(root, plan, "--write", check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(package.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()

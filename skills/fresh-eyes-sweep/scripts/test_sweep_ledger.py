from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("sweep-ledger.py")


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


class SweepLedgerTests(unittest.TestCase):
    def helper(self, *args: str, check: bool = True):
        result = subprocess.run([sys.executable, str(SCRIPT), *args], text=True, capture_output=True)
        if check:
            self.assertEqual(result.returncode, 0, result.stderr)
        return result

    def test_scope_mapping_statuses_preexisting_and_exact_progress(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = root / "repo"
            repo.mkdir()
            git(repo, "init", "-q", "-b", "main")
            git(repo, "config", "user.name", "Test")
            git(repo, "config", "user.email", "test@example.com")
            (repo / "src").mkdir()
            (repo / "docs").mkdir()
            (repo / "src" / "a.py").write_text("a = 1\n")
            (repo / "src" / "b.py").write_text("b = 1\n")
            (repo / "docs" / "readme.md").write_text("docs\n")
            (repo / ".gitignore").write_text("ignored\n")
            git(repo, "add", ".")
            git(repo, "commit", "-qm", "initial")
            (repo / "src" / "a.py").write_text("a = 2\n")
            (repo / "src" / "new.py").write_text("new = 1\n")
            (repo / "ignored").write_text("ignored\n")
            ledger = root / "ledger.json"
            initialized = json.loads(self.helper("init", "--root", str(repo), "--ledger", str(ledger), "src").stdout)
            self.assertEqual(initialized["counts"]["mapped"], 3)
            payload = json.loads(ledger.read_text())
            by_path = {item["path"]: item for item in payload["files"]}
            self.assertTrue(by_path["src/a.py"]["preexistingStatus"])
            self.assertFalse(by_path["src/new.py"]["tracked"])
            self.assertNotIn("ignored", by_path)

            failed = self.helper("mark", "--ledger", str(ledger), "--status", "excluded", "--path", "src/a.py", check=False)
            self.assertNotEqual(failed.returncode, 0)
            self.assertEqual(json.loads(ledger.read_text())["revision"], 0)
            self.helper("mark", "--ledger", str(ledger), "--status", "inspected", "--path", "src/a.py")
            marked = json.loads(self.helper(
                "mark", "--ledger", str(ledger), "--status", "excluded", "--reason", "agent classified generated",
                "--path", "src/b.py",
            ).stdout)
            self.assertEqual(marked["counts"]["accounted"], 2)
            self.assertEqual(marked["progress"]["bar"], "███████░░░")
            pending = json.loads(self.helper("pending", "--ledger", str(ledger), "--limit", "1").stdout)
            self.assertEqual(pending["paths"], ["src/new.py"])
            final = json.loads(self.helper("mark", "--ledger", str(ledger), "--status", "fixed", "--path", "src/new.py").stdout)
            self.assertTrue(final["complete"])
            self.assertEqual(final["counts"]["fixed"], 1)

    def test_atomic_mark_rejects_unknown_path_without_partial_change(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = root / "repo"
            repo.mkdir()
            git(repo, "init", "-q")
            git(repo, "config", "user.name", "Test")
            git(repo, "config", "user.email", "test@example.com")
            (repo / "a").write_text("a")
            git(repo, "add", "a")
            git(repo, "commit", "-qm", "initial")
            ledger = root / "ledger.json"
            self.helper("init", "--root", str(repo), "--ledger", str(ledger))
            before = ledger.read_bytes()
            result = self.helper(
                "mark", "--ledger", str(ledger), "--status", "reported", "--path", "a", "--path", "missing",
                check=False,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(ledger.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()

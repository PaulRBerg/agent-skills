from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "skills" / "naming-refactor" / "scripts" / "naming-ledger.py"


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


def make_repo(root: Path) -> Path:
    repo = root / "repo"
    repo.mkdir()
    git(repo, "init", "-q", "-b", "main")
    git(repo, "config", "user.name", "Test")
    git(repo, "config", "user.email", "test@example.com")
    return repo


class NamingLedgerTests(unittest.TestCase):
    def helper(self, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run([sys.executable, str(SCRIPT), *args], text=True, capture_output=True)
        if check:
            self.assertEqual(result.returncode, 0, result.stderr)
        return result

    def test_maps_repository_and_enforces_status_contract(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root)
            (repo / "src").mkdir()
            (repo / "src" / "a.py").write_text("a = 1\n")
            (repo / "src" / "b.py").write_text("b = 1\n")
            (repo / ".gitignore").write_text("ignored\n")
            git(repo, "add", ".")
            git(repo, "commit", "-qm", "initial")
            (repo / "src" / "a.py").write_text("a = 2\n")
            (repo / "src" / "new.py").write_text("new = 1\n")
            (repo / "ignored").write_text("ignored\n")
            ledger = root / "ledger.json"

            initialized = json.loads(
                self.helper("init", "--root", str(repo), "--ledger", str(ledger)).stdout
            )
            self.assertEqual(initialized["counts"]["mapped"], 4)
            self.assertEqual(initialized["preexistingChangedPaths"], 2)
            payload = json.loads(ledger.read_text())
            by_path = {item["path"]: item for item in payload["files"]}
            self.assertTrue(by_path["src/a.py"]["preexistingStatus"])
            self.assertFalse(by_path["src/new.py"]["tracked"])
            self.assertNotIn("ignored", by_path)

            before = ledger.read_bytes()
            failed = self.helper(
                "mark", "--ledger", str(ledger), "--status", "excluded", "--path", "src/a.py", check=False
            )
            self.assertNotEqual(failed.returncode, 0)
            self.assertEqual(ledger.read_bytes(), before)

            self.helper("mark", "--ledger", str(ledger), "--status", "retained", "--path", ".gitignore")
            self.helper("mark", "--ledger", str(ledger), "--status", "renamed", "--path", "src/a.py")
            self.helper("mark", "--ledger", str(ledger), "--status", "retained", "--path", "src/b.py")
            final = json.loads(
                self.helper(
                    "mark",
                    "--ledger",
                    str(ledger),
                    "--status",
                    "excluded",
                    "--reason",
                    "generated fixture",
                    "--path",
                    "src/new.py",
                ).stdout
            )
            self.assertTrue(final["complete"])
            self.assertEqual(final["counts"]["accounted"], 4)

    def test_mark_is_atomic_and_blocked_prevents_completion(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root)
            (repo / "a.py").write_text("a = 1\n")
            git(repo, "add", "a.py")
            git(repo, "commit", "-qm", "initial")
            ledger = root / "ledger.json"
            self.helper("init", "--root", str(repo), "--ledger", str(ledger))
            before = ledger.read_bytes()

            failed = self.helper(
                "mark",
                "--ledger",
                str(ledger),
                "--status",
                "retained",
                "--path",
                "a.py",
                "--path",
                "missing.py",
                check=False,
            )
            self.assertNotEqual(failed.returncode, 0)
            self.assertEqual(ledger.read_bytes(), before)

            blocked = json.loads(
                self.helper(
                    "mark",
                    "--ledger",
                    str(ledger),
                    "--status",
                    "blocked",
                    "--reason",
                    "external contract is unknown",
                    "--path",
                    "a.py",
                ).stdout
            )
            self.assertFalse(blocked["complete"])
            unblocked = json.loads(
                self.helper("mark", "--ledger", str(ledger), "--status", "retained", "--path", "a.py").stdout
            )
            self.assertTrue(unblocked["complete"])

    def test_mark_from_file_applies_mixed_dispositions(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root)
            for path in ("a.py", "b.py", "c.py"):
                (repo / path).write_text(f"{path} = 1\n")
            git(repo, "add", ".")
            git(repo, "commit", "-qm", "initial")
            ledger = root / "ledger.json"
            self.helper("init", "--root", str(repo), "--ledger", str(ledger))

            (repo / "b.py").unlink()
            refreshed = json.loads(self.helper("refresh", "--ledger", str(ledger)).stdout)
            dispositions = root / "dispositions.tsv"
            dispositions.write_bytes(
                b"  # delegated results\r\n"
                b"\r\n"
                b"retained\ta.py\t\r\n"
                b"renamed\tb.py\tmoved to clearer.py\r\n"
                b"excluded\tc.py\tgenerated fixture\r\n"
            )

            result = json.loads(
                self.helper(
                    "mark", "--ledger", str(ledger), "--from-file", str(dispositions)
                ).stdout
            )
            self.assertEqual(result["applied"], 3)
            self.assertEqual(result["skipped"], [])
            self.assertEqual(result["revision"], refreshed["revision"] + 1)
            self.assertEqual(result["counts"]["pending"], 0)
            self.assertTrue(result["complete"])

            payload = json.loads(ledger.read_text())
            by_path = {item["path"]: item for item in payload["files"]}
            self.assertEqual(by_path["a.py"]["status"], "retained")
            self.assertIsNone(by_path["a.py"]["reason"])
            self.assertEqual(by_path["b.py"]["status"], "renamed")
            self.assertEqual(by_path["b.py"]["reason"], "moved to clearer.py")
            self.assertEqual(by_path["c.py"]["status"], "excluded")
            self.assertEqual(by_path["c.py"]["reason"], "generated fixture")

    def test_mark_from_file_unknown_paths_fail_closed_or_skip(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root)
            (repo / "a.py").write_text("a = 1\n")
            (repo / "b.py").write_text("b = 1\n")
            git(repo, "add", ".")
            git(repo, "commit", "-qm", "initial")
            ledger = root / "ledger.json"
            self.helper("init", "--root", str(repo), "--ledger", str(ledger))
            dispositions = root / "dispositions.tsv"
            dispositions.write_text(
                "retained\ta.py\t\n"
                "excluded\tcache.stats.csv\tignored cache\n"
                "renamed\tempty-directory\tdelegated path\n"
            )
            before = ledger.read_bytes()

            failed = self.helper(
                "mark",
                "--ledger",
                str(ledger),
                "--from-file",
                str(dispositions),
                check=False,
            )
            self.assertNotEqual(failed.returncode, 0)
            self.assertIn("cache.stats.csv", failed.stderr)
            self.assertIn("empty-directory", failed.stderr)
            self.assertEqual(ledger.read_bytes(), before)

            result = json.loads(
                self.helper(
                    "mark",
                    "--ledger",
                    str(ledger),
                    "--from-file",
                    str(dispositions),
                    "--skip-unknown",
                ).stdout
            )
            self.assertEqual(result["applied"], 1)
            self.assertEqual(result["skipped"], ["cache.stats.csv", "empty-directory"])
            payload = json.loads(ledger.read_text())
            by_path = {item["path"]: item for item in payload["files"]}
            self.assertEqual(by_path["a.py"]["status"], "retained")
            self.assertEqual(by_path["b.py"]["status"], "pending")

    def test_mark_from_file_rejects_invalid_lines_atomically(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root)
            (repo / "a.py").write_text("a = 1\n")
            git(repo, "add", "a.py")
            git(repo, "commit", "-qm", "initial")
            ledger = root / "ledger.json"
            self.helper("init", "--root", str(repo), "--ledger", str(ledger))
            dispositions = root / "dispositions.tsv"
            before = ledger.read_bytes()
            invalid_files = (
                ("retained\n", "line 1"),
                ("unknown\ta.py\t\n", "line 1"),
                ("retained\ta.py\t\nrenamed\ta.py\t\n", "line 2"),
                ("excluded\ta.py\t\n", "line 1"),
                ("blocked\ta.py\t\n", "line 1"),
            )

            for contents, expected_error in invalid_files:
                with self.subTest(contents=contents):
                    dispositions.write_text(contents)
                    failed = self.helper(
                        "mark",
                        "--ledger",
                        str(ledger),
                        "--from-file",
                        str(dispositions),
                        check=False,
                    )
                    self.assertNotEqual(failed.returncode, 0)
                    self.assertIn(expected_error, failed.stderr)
                    self.assertEqual(ledger.read_bytes(), before)

    def test_mark_from_file_is_mutually_exclusive_with_single_status_options(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            ledger = root / "ledger.json"
            dispositions = root / "dispositions.tsv"
            conflicting_options = (
                ("--status", "retained"),
                ("--path", "a.py"),
                ("--reason", "reviewed"),
            )

            for options in conflicting_options:
                with self.subTest(options=options):
                    failed = self.helper(
                        "mark",
                        "--ledger",
                        str(ledger),
                        "--from-file",
                        str(dispositions),
                        *options,
                        check=False,
                    )
                    self.assertEqual(failed.returncode, 2)

    def test_refresh_tracks_both_sides_of_a_move_and_is_idempotent(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root)
            (repo / "old-name.py").write_text("value = 1\n")
            git(repo, "add", "old-name.py")
            git(repo, "commit", "-qm", "initial")
            ledger = root / "ledger.json"
            self.helper("init", "--root", str(repo), "--ledger", str(ledger))

            (repo / "old-name.py").rename(repo / "clear-name.py")
            refreshed = json.loads(self.helper("refresh", "--ledger", str(ledger)).stdout)
            self.assertEqual(refreshed["addedPaths"], ["clear-name.py"])
            self.assertIn("old-name.py", refreshed["missingPaths"])
            self.assertEqual(refreshed["counts"]["pending"], 2)
            revision = refreshed["revision"]

            repeated = json.loads(self.helper("refresh", "--ledger", str(ledger)).stdout)
            self.assertEqual(repeated["addedPaths"], [])
            self.assertEqual(repeated["revision"], revision)

            self.helper("mark", "--ledger", str(ledger), "--status", "renamed", "--path", "old-name.py")
            final = json.loads(
                self.helper("mark", "--ledger", str(ledger), "--status", "renamed", "--path", "clear-name.py").stdout
            )
            self.assertTrue(final["complete"])


if __name__ == "__main__":
    unittest.main()

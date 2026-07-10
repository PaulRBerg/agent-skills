from __future__ import annotations

import json
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("repo-rename.py")


class RepoRenameTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.repo = self.root / "old-repo"
        self.repo.mkdir()
        subprocess.run(["git", "init", "-q", "-b", "main"], cwd=self.repo, check=True)
        subprocess.run(["git", "config", "user.name", "Test"], cwd=self.repo, check=True)
        subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=self.repo, check=True)
        (self.repo / "README.md").write_text("old-repo\n", encoding="utf-8")
        subprocess.run(["git", "add", "README.md"], cwd=self.repo, check=True)
        subprocess.run(["git", "commit", "-qm", "init"], cwd=self.repo, check=True)
        subprocess.run(
            ["git", "remote", "add", "origin", "git@github.com:owner/old-repo.git"],
            cwd=self.repo,
            check=True,
        )
        self.bin = self.root / "bin"
        self.bin.mkdir()
        gh = self.bin / "gh"
        gh.write_text(
            "#!/bin/sh\n"
            "if [ \"$1 $2 $3\" = \"repo view --json\" ]; then\n"
            "  printf '%s\\n' '{\"nameWithOwner\":\"owner/old-repo\",\"sshUrl\":\"git@github.com:owner/old-repo.git\",\"url\":\"https://github.com/owner/old-repo\"}'\n"
            "  exit 0\n"
            "fi\n"
            "printf '%s\\n' \"$*\" >> \"$HOME/gh-writes\"\n",
            encoding="utf-8",
        )
        gh.chmod(gh.stat().st_mode | stat.S_IXUSR)
        self.env = os.environ | {
            "HOME": str(self.root / "home"),
            "PATH": f"{self.bin}:{os.environ['PATH']}",
        }
        Path(self.env["HOME"]).mkdir()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def run_script(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), *args],
            cwd=self.repo,
            env=self.env,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_dry_run_has_no_mutations(self) -> None:
        before = subprocess.run(["git", "status", "--short"], cwd=self.repo, text=True, capture_output=True, check=True).stdout
        result = self.run_script("new-repo", "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["confirmation_token"], "owner/old-repo->owner/new-repo")
        self.assertTrue(self.repo.exists())
        self.assertFalse((self.root / "new-repo").exists())
        self.assertFalse((Path(self.env["HOME"]) / "gh-writes").exists())
        after = subprocess.run(["git", "status", "--short"], cwd=self.repo, text=True, capture_output=True, check=True).stdout
        self.assertEqual(after, before)

    def test_apply_rejects_missing_confirmation(self) -> None:
        result = self.run_script("new-repo", "--apply")
        self.assertEqual(result.returncode, 64)
        self.assertIn("--confirm", result.stderr)
        self.assertFalse((Path(self.env["HOME"]) / "gh-writes").exists())


if __name__ == "__main__":
    unittest.main()

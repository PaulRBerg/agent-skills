from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("git-squash.py")


def git(repo: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(["git", "-C", str(repo), *args], text=True, capture_output=True)
    if check and result.returncode:
        raise AssertionError(result.stderr)
    return result


def make_repo(root: Path, commits: int = 2) -> Path:
    root.mkdir()
    git(root, "init", "-q", "-b", "main")
    git(root, "config", "user.name", "Squasher")
    git(root, "config", "user.email", "squasher@example.com")
    (root / "file.txt").write_text("base\n")
    git(root, "add", "file.txt")
    git(root, "commit", "-qm", "base")
    git(root, "switch", "-qc", "feature")
    for index in range(commits):
        (root / "file.txt").write_text((root / "file.txt").read_text() + f"change {index}\n")
        git(root, "add", "file.txt")
        env = os.environ | {"GIT_AUTHOR_NAME": "Contributor" if index == 0 else "Squasher", "GIT_AUTHOR_EMAIL": "contributor@example.com" if index == 0 else "squasher@example.com"}
        subprocess.run(["git", "-C", str(root), "commit", "-qm", f"change {index}"], check=True, env=env)
    return root


class GitSquashTests(unittest.TestCase):
    def helper(self, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run([sys.executable, str(SCRIPT), *args], text=True, capture_output=True)
        if check:
            self.assertEqual(result.returncode, 0, result.stderr)
        return result

    def test_plan_and_successful_apply_without_remote(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root / "repo")
            plan = json.loads(self.helper("plan", "--cwd", str(repo), "--base", "main", "--subject", "feat: final").stdout)
            self.assertEqual(plan["aheadCount"], 2)
            self.assertFalse(plan["remote"]["originConfigured"])
            self.assertIn("Contributor <contributor@example.com>", plan["authors"])
            plan_file = root / "plan.json"
            plan_file.write_text(json.dumps(plan))
            message = root / "message.txt"
            message.write_text("feat: final\n\nCo-authored-by: Contributor <contributor@example.com>\n")
            result = json.loads(self.helper("apply", "--plan", str(plan_file), "--message-file", str(message)).stdout)
            self.assertEqual(result["subject"], "feat: final")
            self.assertEqual(git(repo, "rev-list", "--count", "main..HEAD").stdout.strip(), "1")

    def test_dirty_detached_default_zero_ahead_and_bad_base_fail(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = make_repo(Path(directory) / "repo", commits=1)
            (repo / "dirty").write_text("x")
            self.assertNotEqual(self.helper("plan", "--cwd", str(repo), check=False).returncode, 0)
            (repo / "dirty").unlink()
            git(repo, "switch", "--detach", "-q")
            self.assertNotEqual(self.helper("plan", "--cwd", str(repo), check=False).returncode, 0)
            git(repo, "switch", "-q", "main")
            self.assertNotEqual(self.helper("plan", "--cwd", str(repo), check=False).returncode, 0)
            git(repo, "switch", "-qc", "empty")
            self.assertNotEqual(self.helper("plan", "--cwd", str(repo), check=False).returncode, 0)
            self.assertNotEqual(self.helper("plan", "--cwd", str(repo), "--base", "missing", check=False).returncode, 0)

    def test_stale_plan_and_commit_failure_restore_exact_state(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root / "repo", commits=2)
            plan = json.loads(self.helper("plan", "--cwd", str(repo)).stdout)
            plan_file = root / "plan.json"
            plan_file.write_text(json.dumps(plan))
            message = root / "message.txt"
            message.write_text("fix: squash\n")
            (repo / "file.txt").write_text((repo / "file.txt").read_text() + "stale\n")
            stale = self.helper("apply", "--plan", str(plan_file), "--message-file", str(message), check=False)
            self.assertNotEqual(stale.returncode, 0)
            git(repo, "restore", "file.txt")

            hook = repo / ".git" / "hooks" / "commit-msg"
            hook.write_text("#!/bin/sh\nexit 9\n")
            hook.chmod(0o755)
            original = git(repo, "rev-parse", "HEAD").stdout.strip()
            failed = self.helper("apply", "--plan", str(plan_file), "--message-file", str(message), check=False)
            self.assertNotEqual(failed.returncode, 0)
            self.assertEqual(git(repo, "rev-parse", "HEAD").stdout.strip(), original)
            self.assertEqual(git(repo, "status", "--porcelain").stdout, "")


if __name__ == "__main__":
    unittest.main()

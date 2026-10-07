from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "skills" / "commit" / "scripts" / "git-squash.py"


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


def head(repo: Path) -> str:
    return git(repo, "rev-parse", "HEAD").stdout.strip()


class GitSquashTests(unittest.TestCase):
    def helper(self, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run([sys.executable, str(SCRIPT), *args], text=True, capture_output=True)
        if check:
            self.assertEqual(result.returncode, 0, result.stderr)
        return result

    def write_plan(self, root: Path, repo: Path, *args: str) -> tuple[dict, Path]:
        plan = json.loads(self.helper("plan", "--cwd", str(repo), *args).stdout)
        plan_file = root / "plan.json"
        plan_file.write_text(json.dumps(plan))
        return plan, plan_file

    def assert_failed(self, result: subprocess.CompletedProcess[str], message: str) -> None:
        self.assertEqual(result.returncode, 1)
        self.assertIn(f"ERROR: {message}", result.stderr)

    def test_plan_reset_and_rollback_without_remote(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root / "repo")
            plan, plan_file = self.write_plan(root, repo, "--base", "main")
            self.assertEqual(plan["aheadCount"], 2)
            self.assertFalse(plan["remote"]["originConfigured"])
            self.assertIn("Contributor <contributor@example.com>", plan["authors"])
            self.assertNotIn("subjectOverride", plan)

            reset = json.loads(self.helper("reset", "--plan", str(plan_file)).stdout)
            self.assertEqual(reset["status"], "reset")
            self.assertEqual(reset["commitsReplaced"], 2)
            self.assertIn("Contributor <contributor@example.com>", reset["authors"])
            self.assertEqual(head(repo), plan["mergeBase"])
            self.assertEqual(git(repo, "diff", "--cached", "--quiet", check=False).returncode, 1)
            self.assertEqual(git(repo, "diff", "--quiet", check=False).returncode, 0)

            restored = json.loads(self.helper("rollback", "--plan", str(plan_file)).stdout)
            self.assertEqual(restored, {"schemaVersion": 1, "status": "restored", "head": plan["originalHead"]})
            self.assertEqual(head(repo), plan["originalHead"])
            self.assertEqual(git(repo, "status", "--porcelain").stdout, "")

    def test_origin_without_head_falls_back_to_main(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root / "repo")
            git(repo, "remote", "add", "origin", str(root / "missing.git"))
            plan = json.loads(self.helper("plan", "--cwd", str(repo)).stdout)
            self.assertEqual(plan["baseBranch"], "main")
            self.assertTrue(plan["remote"]["originConfigured"])

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

    def test_stale_plan_reset_fails_without_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root / "repo")
            plan, plan_file = self.write_plan(root, repo)
            (repo / "file.txt").write_text((repo / "file.txt").read_text() + "stale\n")
            self.assert_failed(self.helper("reset", "--plan", str(plan_file), check=False), "stale plan")
            self.assertEqual(head(repo), plan["originalHead"])
            self.assertEqual(git(repo, "status", "--porcelain").stdout, " M file.txt\n")

    def test_empty_net_diff_reset_restores_head_and_index(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root / "repo", commits=1)
            git(repo, "revert", "--no-edit", "HEAD")
            plan, plan_file = self.write_plan(root, repo)
            self.assertEqual(plan["aheadCount"], 2)
            self.assert_failed(self.helper("reset", "--plan", str(plan_file), check=False), "squash would produce an empty commit")
            self.assertEqual(head(repo), plan["originalHead"])
            self.assertEqual(git(repo, "write-tree").stdout.strip(), plan["rollback"]["indexTree"])
            self.assertEqual(git(repo, "status", "--porcelain").stdout, "")

    def test_rollback_requires_head_at_merge_base(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root / "repo")
            plan, plan_file = self.write_plan(root, repo)
            result = self.helper("rollback", "--plan", str(plan_file), check=False)
            self.assert_failed(result, "nothing to roll back: HEAD is not at the merge base")
            self.assertEqual(head(repo), plan["originalHead"])
            self.assertEqual(git(repo, "status", "--porcelain").stdout, "")


if __name__ == "__main__":
    unittest.main()

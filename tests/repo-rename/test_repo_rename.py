from __future__ import annotations

import json
import os
import re
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "skills" / "repo-rename" / "scripts" / "repo-rename.py"


class RepoRenameTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="repo.rename_")
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
        self.env.pop("CLAUDE_CONFIG_DIR", None)
        self.env.pop("CODEX_HOME", None)
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

    def test_preview_and_apply_preserve_protected_and_external_files(self) -> None:
        notes = self.repo / ".ai"
        notes.mkdir()
        for name in ("PROMPT.md", "TODO.md"):
            (notes / name).write_text("old-repo\n", encoding="utf-8")
        external = self.root / "external.md"
        external.write_text("old-repo\n", encoding="utf-8")
        (self.repo / "linked.md").symlink_to(external)
        (self.repo / "linked-dir").symlink_to(notes, target_is_directory=True)
        subprocess.run(["git", "add", "-f", ".ai", "linked.md", "linked-dir"], cwd=self.repo, check=True)
        subprocess.run(["git", "commit", "-qm", "fixtures"], cwd=self.repo, check=True)
        generated = self.repo / ".git" / "generated.md"
        generated.write_text("old-repo\n", encoding="utf-8")
        for directory in (".venv", ".cache"):
            excluded = self.repo / directory
            excluded.mkdir()
            (excluded / "generated.txt").write_text("old-repo\n", encoding="utf-8")
        (self.repo / ".git" / "info" / "exclude").write_text(".venv/\n.cache/\n", encoding="utf-8")

        result = self.run_script("new-repo", "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        replacements = json.loads(result.stdout)["mutations"][-1]["files"]
        self.assertEqual(replacements, [{"path": str((self.repo / "README.md").resolve()), "occurrences": 1}])

        result = self.run_script("new-repo", "--apply", "--confirm", "owner/old-repo->owner/new-repo")
        self.assertEqual(result.returncode, 0, result.stderr)
        renamed = self.root / "new-repo"
        self.assertEqual((renamed / "README.md").read_text(), "new-repo\n")
        for name in ("PROMPT.md", "TODO.md"):
            self.assertEqual((renamed / ".ai" / name).read_text(), "old-repo\n")
        self.assertEqual((renamed / ".git" / "generated.md").read_text(), "old-repo\n")
        self.assertTrue((renamed / "linked.md").is_symlink())
        self.assertTrue((renamed / "linked-dir").is_symlink())
        self.assertEqual(external.read_text(), "old-repo\n")
        for directory in (".venv", ".cache"):
            self.assertEqual((renamed / directory / "generated.txt").read_text(), "old-repo\n")

    def test_continuity_requires_active_transcript_and_only_replaces_paths(self) -> None:
        codex = self.root / "custom-codex"
        codex.mkdir()
        self.env["CODEX_HOME"] = str(codex)
        config = codex / "config.toml"
        old_path = str(self.repo.resolve())
        config.write_text(f'project = "{old_path}"\nother = "old-repo"\n', encoding="utf-8")
        result = self.run_script("new-repo", "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        files = json.loads(result.stdout)["mutations"][-1]["files"]
        self.assertEqual(len(files), 1)

        sessions = codex / "sessions"
        sessions.mkdir()
        session = sessions / "session.jsonl"
        session.write_text(json.dumps({"cwd": old_path}) + "\n", encoding="utf-8")
        result = self.run_script("new-repo", "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        files = json.loads(result.stdout)["mutations"][-1]["files"]
        self.assertIn({"path": str(config), "occurrences": 1}, files)
        self.assertEqual(len(files), 3)
        result = self.run_script("new-repo", "--apply", "--confirm", "owner/old-repo->owner/new-repo")
        self.assertEqual(result.returncode, 0, result.stderr)
        new_path = str((self.root / "new-repo").resolve())
        self.assertEqual(config.read_text(), f'project = "{new_path}"\nother = "old-repo"\n')
        self.assertEqual(json.loads(session.read_text())["cwd"], new_path)

    def test_custom_claude_project_is_moved(self) -> None:
        claude = self.root / "custom-claude"
        self.env["CLAUDE_CONFIG_DIR"] = str(claude)
        old_path = str(self.repo.resolve())
        project = claude / "projects" / re.sub(r"[^A-Za-z0-9]", "-", old_path)
        project.mkdir(parents=True)
        (project / "session.jsonl").write_text(json.dumps({"cwd": old_path}), encoding="utf-8")
        result = self.run_script("new-repo", "--apply", "--confirm", "owner/old-repo->owner/new-repo")
        self.assertEqual(result.returncode, 0, result.stderr)
        new_path = str((self.root / "new-repo").resolve())
        moved = claude / "projects" / re.sub(r"[^A-Za-z0-9]", "-", new_path)
        self.assertFalse(project.exists())
        self.assertEqual(json.loads((moved / "session.jsonl").read_text())["cwd"], new_path)

    def test_explicit_local_state_updates_preserve_executable_mode(self) -> None:
        files = ("PROMPT.md", "TODO.md", ".venv/bin/tool", ".cache/state")
        for name in files:
            target = self.repo / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("old-repo\n", encoding="utf-8")
            target.chmod(0o755)
        subprocess.run(["git", "add", "-f", *files], cwd=self.repo, check=True)
        subprocess.run(["git", "commit", "-qm", "fixtures"], cwd=self.repo, check=True)
        result = self.run_script("new-repo", "--dry-run", "--include-local-state")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertTrue(report["include_local_state"])
        self.assertEqual(len(report["mutations"][-1]["files"]), 5)
        result = self.run_script("new-repo", "--apply", "--include-local-state", "--confirm", "owner/old-repo->owner/new-repo")
        self.assertEqual(result.returncode, 0, result.stderr)
        for name in files:
            target = self.root / "new-repo" / name
            self.assertEqual(target.read_text(), "new-repo\n")
            self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o755)


if __name__ == "__main__":
    unittest.main()

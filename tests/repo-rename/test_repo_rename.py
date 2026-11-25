# /// script
# requires-python = ">=3.14"
# dependencies = []
# ///
from __future__ import annotations

import contextlib
import importlib.util
import json
import os
import re
import sqlite3
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from compression import zstd

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "skills" / "repo-rename" / "scripts" / "repo-rename.py"
SPEC = importlib.util.spec_from_file_location("repo_rename", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
HELPER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = HELPER
SPEC.loader.exec_module(HELPER)

THREAD_ID = "01953a84-42ea-7ae2-a6ad-22ec7dca7000"
OTHER_THREAD_ID = "01953a84-42ea-7ae2-a6ad-22ec7dca7001"


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
            'if [ "$1 $2 $3" = "repo view --json" ]; then\n'
            '  printf \'%s\\n\' \'{"nameWithOwner":"owner/old-repo","sshUrl":"git@github.com:owner/old-repo.git","url":"https://github.com/owner/old-repo"}\'\n'
            "  exit 0\n"
            "fi\n"
            'printf \'%s\\n\' "$*" >> "$HOME/gh-writes"\n',
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

    def write_rollout(self, path: Path, cwd: Path, thread_id: str = THREAD_ID) -> bytes:
        header = {"type": "session_meta", "payload": {"id": thread_id, "cwd": str(cwd)}}
        message = {"type": "response_item", "payload": {"text": f"Worked in {self.repo.resolve()}"}}
        data = "".join(json.dumps(record) + "\n" for record in (header, message)).encode()
        if path.suffix == ".zst":
            data = zstd.compress(data)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        return data

    def test_dry_run_has_no_mutations(self) -> None:
        before = subprocess.run(
            ["git", "status", "--short"], cwd=self.repo, text=True, capture_output=True, check=True
        ).stdout
        result = self.run_script("new-repo", "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["confirmation_token"], "owner/old-repo->owner/new-repo")
        self.assertTrue(self.repo.exists())
        self.assertFalse((self.root / "new-repo").exists())
        self.assertFalse((Path(self.env["HOME"]) / "gh-writes").exists())
        after = subprocess.run(
            ["git", "status", "--short"], cwd=self.repo, text=True, capture_output=True, check=True
        ).stdout
        self.assertEqual(after, before)

    def test_apply_rejects_missing_confirmation(self) -> None:
        result = self.run_script("new-repo", "--apply")
        self.assertEqual(result.returncode, 64)
        self.assertIn("--confirm", result.stderr)
        self.assertFalse((Path(self.env["HOME"]) / "gh-writes").exists())

    def test_same_name_move_previews_then_preserves_remote_and_updates_paths(self) -> None:
        old_path = str(self.repo.resolve())
        destination = self.root / "work" / "apps" / "old-repo"
        new_path = str(destination.resolve())
        readme = self.repo / "README.md"
        readme.write_text(f"old-repo\n{old_path}/src\n", encoding="utf-8")
        subprocess.run(["git", "commit", "-qam", "path fixture"], cwd=self.repo, check=True)
        claude = Path(self.env["HOME"]) / ".claude" / "projects"
        project = claude / re.sub(r"[^A-Za-z0-9]", "-", old_path)
        project.mkdir(parents=True)
        (project / "session.jsonl").write_text(json.dumps({"cwd": old_path}), encoding="utf-8")
        result = self.run_script(old_path, str(destination), "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertFalse(report["rename_github"])
        self.assertEqual(report["old_repo"], report["new_repo"])
        self.assertEqual(report["old_remote"], report["new_remote"])
        self.assertEqual(report["confirmation_token"], f"move:{old_path}->{new_path}")
        parents = [m["path"] for m in report["mutations"] if m["kind"] == "create-parent"]
        self.assertEqual(parents, [str(destination.parent.parent.resolve()), str(destination.parent.resolve())])
        self.assertFalse(destination.parent.parent.exists())
        self.assertEqual(readme.read_text(), f"old-repo\n{old_path}/src\n")
        self.assertTrue(project.exists())

        result = self.run_script(old_path, str(destination), "--apply", "--confirm", report["confirmation_token"])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(self.repo.exists())
        self.assertEqual((destination / "README.md").read_text(), f"old-repo\n{new_path}/src\n")
        moved_project = claude / re.sub(r"[^A-Za-z0-9]", "-", new_path)
        self.assertEqual(json.loads((moved_project / "session.jsonl").read_text())["cwd"], new_path)
        remote = subprocess.run(
            ["git", "remote", "get-url", "origin"], cwd=destination, text=True, capture_output=True, check=True
        )
        self.assertEqual(remote.stdout.strip(), "git@github.com:owner/old-repo.git")
        self.assertFalse((Path(self.env["HOME"]) / "gh-writes").exists())

    def test_move_rejects_dirty_tree_and_wrong_confirmation(self) -> None:
        destination = self.root / "work" / "old-repo"
        result = self.run_script(str(self.repo), str(destination), "--apply", "--confirm", "wrong")
        self.assertEqual(result.returncode, 64)
        (self.repo / "untracked").write_text("keep")
        result = self.run_script(str(self.repo), str(destination), "--dry-run")
        self.assertEqual(result.returncode, 64)
        self.assertIn("working tree is not clean", result.stderr)
        self.assertFalse(destination.parent.exists())

    def test_move_rejects_invalid_targets(self) -> None:
        symlink = self.root / "work" / "old-repo"
        symlink.parent.mkdir()
        symlink.symlink_to(self.root / "missing")
        for target in (self.repo, self.repo / "old-repo", self.root / "other-name", symlink):
            with self.subTest(target=target):
                result = self.run_script(str(self.repo), str(target), "--dry-run")
                self.assertEqual(result.returncode, 64)
                self.assertTrue(self.repo.exists())

    def test_move_without_remotes_does_not_require_github(self) -> None:
        subprocess.run(["git", "remote", "remove", "origin"], cwd=self.repo, check=True)
        destination = self.root / "work" / "old-repo"
        result = self.run_script(str(self.repo), str(destination), "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertTrue(report["local_only"])
        self.assertEqual(report["mutations"][-1]["files"], [])
        result = self.run_script(str(self.repo), str(destination), "--apply", "--confirm", report["confirmation_token"])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((destination / "README.md").read_text(), "old-repo\n")

    def test_move_failure_restores_source_and_removes_created_parents(self) -> None:
        subprocess.run(["git", "remote", "remove", "origin"], cwd=self.repo, check=True)
        destination = self.root / "work" / "apps" / "old-repo"
        readme = self.repo / "README.md"
        old_text = str(self.repo.resolve()) + "\n"
        readme.write_text(old_text)
        subprocess.run(["git", "commit", "-qam", "path fixture"], cwd=self.repo, check=True)
        with patch.dict(os.environ, self.env, clear=True):
            plan = HELPER.preflight(str(self.repo), str(destination))
        with (
            patch.object(HELPER.os, "replace", side_effect=OSError("injected write failure")),
            self.assertRaisesRegex(HELPER.RenameError, "rollback completed"),
        ):
            HELPER.apply(plan)
        self.assertEqual(readme.read_text(), old_text)
        self.assertFalse(destination.parent.parent.exists())
        status = subprocess.run(
            ["git", "status", "--porcelain"], cwd=self.repo, text=True, capture_output=True, check=True
        )
        self.assertEqual(status.stdout, "")

    def test_claude_move_failure_restores_files_in_place(self) -> None:
        subprocess.run(["git", "remote", "remove", "origin"], cwd=self.repo, check=True)
        old_path = str(self.repo.resolve())
        claude = Path(self.env["HOME"]) / ".claude" / "projects"
        project = claude / re.sub(r"[^A-Za-z0-9]", "-", old_path)
        project.mkdir(parents=True)
        (project / "session.jsonl").write_text(json.dumps({"cwd": old_path}), encoding="utf-8")
        with patch.dict(os.environ, self.env, clear=True), contextlib.chdir(self.repo):
            plan = HELPER.preflight("new-repo")
        real_rename = Path.rename

        def fail_claude_move(path: Path, target: Path) -> Path:
            if path == plan["old_claude"]:
                raise OSError("injected Claude move failure")
            return real_rename(path, target)

        with (
            patch.object(HELPER.Path, "rename", fail_claude_move),
            self.assertRaisesRegex(HELPER.RenameError, "rollback completed"),
        ):
            HELPER.apply(plan)
        self.assertTrue(self.repo.exists())
        self.assertEqual(json.loads((project / "session.jsonl").read_text())["cwd"], old_path)
        self.assertFalse(plan["new_claude"].exists())

    def test_default_preview_excludes_ignored_native_build_output(self) -> None:
        ignored = self.repo / "apps" / "mobile" / "ios" / "Pods" / "config.txt"
        ignored.parent.mkdir(parents=True)
        ignored.write_text(str(self.repo.resolve()))
        (self.repo / ".gitignore").write_text("apps/mobile/ios/\n")
        subprocess.run(["git", "add", ".gitignore"], cwd=self.repo, check=True)
        subprocess.run(["git", "commit", "-qm", "ignore native output"], cwd=self.repo, check=True)
        destination = self.root / "work" / "old-repo"
        result = self.run_script(str(self.repo), str(destination), "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["mutations"][-1]["files"], [])
        result = self.run_script(str(self.repo), str(destination), "--dry-run", "--include-local-state")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            json.loads(result.stdout)["mutations"][-1]["files"],
            [
                {"path": str(ignored.resolve()), "occurrences": 1},
            ],
        )

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

    def test_local_only_repo_skips_github_and_origin(self) -> None:
        subprocess.run(["git", "remote", "remove", "origin"], cwd=self.repo, check=True)
        result = self.run_script("new-repo", "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertTrue(report["local_only"])
        self.assertEqual(report["confirmation_token"], "local:old-repo->new-repo")
        kinds = [mutation["kind"] for mutation in report["mutations"]]
        self.assertNotIn("github-rename", kinds)
        self.assertNotIn("origin", kinds)
        result = self.run_script("new-repo", "--apply", "--confirm", "local:old-repo->new-repo")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.root / "new-repo" / "README.md").read_text(), "new-repo\n")
        self.assertFalse((Path(self.env["HOME"]) / "gh-writes").exists())

    def test_rename_preserves_imports_prose_and_command_selectors(self) -> None:
        subprocess.run(["git", "remote", "remove", "origin"], cwd=self.repo, check=True)
        self.repo = self.repo.rename(self.root / "quantum")
        old_path = str(self.repo.resolve())
        readme = self.repo / "README.md"
        readme.write_text(f"# quantum\nA quantum system.\n`just run 1 quantum_algorithm`\n{old_path}/src\n")
        source = self.repo / "example.py"
        source.write_text('from qiskit.quantum_info import Statevector\nprint("quantum")\n')
        manifest = self.repo / "pyproject.toml"
        manifest.write_text('[project]\nname = "quantum"\ndescription = "quantum mechanics"\n')
        subprocess.run(["git", "add", "README.md", "example.py", "pyproject.toml"], cwd=self.repo, check=True)
        subprocess.run(["git", "commit", "-qm", "quantum fixture"], cwd=self.repo, check=True)

        result = self.run_script("emergent-qiskit", "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(
            report["mutations"][-1]["files"],
            [{"path": str(readme.resolve()), "occurrences": 2}, {"path": str(manifest.resolve()), "occurrences": 1}],
        )
        result = self.run_script("emergent-qiskit", "--apply", "--confirm", report["confirmation_token"])
        self.assertEqual(result.returncode, 0, result.stderr)
        renamed = self.root / "emergent-qiskit"
        self.assertEqual(
            (renamed / "README.md").read_text(),
            f"# emergent-qiskit\nA quantum system.\n`just run 1 quantum_algorithm`\n{renamed.resolve()}/src\n",
        )
        self.assertEqual(
            (renamed / "example.py").read_text(), 'from qiskit.quantum_info import Statevector\nprint("quantum")\n'
        )
        self.assertEqual(
            (renamed / "pyproject.toml").read_text(),
            '[project]\nname = "emergent-qiskit"\ndescription = "quantum mechanics"\n',
        )

    def test_rename_updates_only_matching_github_identity(self) -> None:
        readme = self.repo / "README.md"
        before = (
            "https://github.com/owner/old-repo/issues\n"
            "git@github.com:owner/old-repo.git\n"
            "https://github.com/owner/old-repo-other\n"
            "https://github.com/other/old-repo\n"
        )
        readme.write_text(before)
        subprocess.run(["git", "commit", "-qam", "URL fixture"], cwd=self.repo, check=True)
        result = self.run_script("new-repo", "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            json.loads(result.stdout)["mutations"][-1]["files"], [{"path": str(readme.resolve()), "occurrences": 2}]
        )
        result = self.run_script("new-repo", "--apply", "--confirm", "owner/old-repo->owner/new-repo")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            (self.root / "new-repo" / "README.md").read_text(),
            before.replace("owner/old-repo/issues", "owner/new-repo/issues").replace(
                "owner/old-repo.git", "owner/new-repo.git"
            ),
        )

    def test_compressed_rollout_activates_continuity_without_rewriting_history(self) -> None:
        codex = self.root / "custom-codex"
        codex.mkdir()
        self.env["CODEX_HOME"] = str(codex)
        config = codex / "config.toml"
        old_path = str(self.repo.resolve())
        config.write_text(f'project = "{old_path}"\nother = "old-repo"\n', encoding="utf-8")
        result = self.run_script("new-repo", "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["codex_resume"], {})
        self.assertEqual(len(report["mutations"][-1]["files"]), 1)

        session = codex / "sessions" / "2026" / "rollout-compressed.jsonl.zst"
        (self.repo / "src").mkdir()
        before = self.write_rollout(session, self.repo.resolve() / "src")
        new_root = (self.root / "new-repo").resolve()
        expected = {THREAD_ID: [str(new_root / "src")]}
        result = self.run_script("new-repo", "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        files = report["mutations"][-1]["files"]
        self.assertEqual(report["codex_resume"], expected)
        self.assertIn({"path": str(config.resolve()), "occurrences": 1}, files)
        self.assertEqual(len(files), 2)
        self.assertEqual(session.read_bytes(), before)
        result = self.run_script("new-repo", "--apply", "--confirm", "owner/old-repo->owner/new-repo")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["codex_resume"], expected)
        self.assertEqual(config.read_text(), f'project = "{new_root}"\nother = "old-repo"\n')
        self.assertEqual(session.read_bytes(), before)

    def test_resume_reports_stable_ids_and_destination_cwds_for_same_name_move(self) -> None:
        sessions = Path(self.env["HOME"]) / ".codex" / "sessions"
        source = self.repo.resolve()
        (source / "src").mkdir()
        (source / "docs with spaces").mkdir()
        plain = sessions / f"{THREAD_ID}_fixture-revision.jsonl"
        before = self.write_rollout(plain, source)
        first_message = before.index(b"\n") + 1
        duplicate = sessions / "rollout-duplicate.jsonl.zst"
        other_revision = sessions / f"{THREAD_ID}_other-revision.jsonl"
        other = sessions / "rollout-other.jsonl"
        snapshots = {
            duplicate: self.write_rollout(duplicate, source),
            other_revision: self.write_rollout(other_revision, source / "src"),
            other: self.write_rollout(other, source / "docs with spaces", OTHER_THREAD_ID),
        }
        destination = (self.root / "path with spaces" / source.name).resolve()
        expected = {
            THREAD_ID: [str(destination), str(destination / "src")],
            OTHER_THREAD_ID: [str(destination / "docs with spaces")],
        }
        result = self.run_script(str(source), str(destination), "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["codex_resume"], expected)
        result = self.run_script(str(source), str(destination), "--apply", "--confirm", report["confirmation_token"])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["codex_resume"], expected)
        self.assertEqual(plain.read_bytes(), before)
        with plain.open("rb") as stream:
            stream.seek(first_message)
            self.assertEqual(stream.readline(), before[first_message:])
        for path, original in snapshots.items():
            self.assertEqual(path.read_bytes(), original)

    def continuity_fixture(self) -> tuple[Path, Path]:
        codex = self.root / "prefilter-codex"
        project = Path(self.env["HOME"]) / ".claude" / "projects" / "example-project"
        project.mkdir(parents=True)
        codex.mkdir()
        old_root = self.repo.resolve()
        needle = str(old_root)
        (project / "hit.jsonl").write_text(f"{needle}\n{needle}/src\n", encoding="utf-8")
        (project / ".hidden.jsonl").write_text(needle, encoding="utf-8")
        (project / "miss.jsonl").write_text("unrelated\n", encoding="utf-8")
        (project / "binary.bin").write_bytes(b"\0" + needle.encode())
        (project / "link.jsonl").symlink_to(project / "hit.jsonl")
        (codex / "config.toml").write_text(f'project = "{needle}"\n', encoding="utf-8")
        return codex, old_root

    def test_path_replacements_preserve_sibling_repositories(self) -> None:
        old_path = str(self.repo.resolve())
        paths = [
            old_path,
            old_path + "/src",
            *(old_path + suffix for suffix in ("-qiskit", "_extra", ".git", ".other")),
        ]
        before = json.dumps(paths) + "\n"
        (self.repo / "README.md").write_text(before)
        subprocess.run(["git", "commit", "-qam", "path boundary fixture"], cwd=self.repo, check=True)
        codex = Path(self.env["HOME"]) / ".codex"
        sessions = codex / "sessions"
        sessions.mkdir(parents=True)
        session = sessions / "session.jsonl"
        session_bytes = self.write_rollout(session, self.repo.resolve())
        config = codex / "config.toml"
        config.write_text("paths = " + before)
        claude = Path(self.env["HOME"]) / ".claude" / "projects" / "another-project"
        claude.mkdir(parents=True)
        claude_session = claude / "session.jsonl"
        claude_session.write_text(before)

        result = self.run_script("new-repo", "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(len(report["mutations"][-1]["files"]), 3)
        self.assertEqual({item["occurrences"] for item in report["mutations"][-1]["files"]}, {2})
        result = self.run_script("new-repo", "--apply", "--confirm", report["confirmation_token"])
        self.assertEqual(result.returncode, 0, result.stderr)
        renamed = self.root / "new-repo"
        after = json.dumps([str(renamed.resolve()), str(renamed.resolve()) + "/src", *paths[2:]]) + "\n"
        self.assertEqual((renamed / "README.md").read_text(), after)
        self.assertEqual(session.read_bytes(), session_bytes)
        self.assertEqual(config.read_text(), "paths = " + after)
        self.assertEqual(claude_session.read_text(), after)

    def test_only_active_rollout_workspaces_activate_continuity(self) -> None:
        codex = Path(self.env["HOME"]) / ".codex"
        sessions = codex / "sessions"
        source = self.repo.resolve()
        self.write_rollout(sessions / "rollout-sibling.jsonl", source.with_name(source.name + "-sibling"))
        self.write_rollout(sessions / "rollout-elsewhere.jsonl", source / ".." / "elsewhere")
        fake_meta = {"type": "session_meta", "payload": {"cwd": str(source), "id": THREAD_ID}}
        (sessions / f"rollout-{THREAD_ID}.jsonl").write_text(
            json.dumps({"type": "response_item", "payload": {"text": str(source)}})
            + "\n"
            + json.dumps(fake_meta)
            + "\n"
        )
        (sessions / "rollout-invalid-id.jsonl").write_text(
            json.dumps({"type": "session_meta", "payload": {"cwd": str(source), "id": "not-a-uuid"}}) + "\n"
        )
        (sessions / "rollout-corrupt.jsonl.zst").write_bytes(b"not zstd")
        archived = codex / "archived_sessions" / "rollout-archived.jsonl"
        archived_bytes = self.write_rollout(archived, source)
        (sessions / "rollout-link.jsonl").symlink_to(archived)
        state = codex / "state_5.sqlite"
        with contextlib.closing(sqlite3.connect(state)) as db, db:
            db.execute("CREATE TABLE threads (id TEXT, cwd TEXT, rollout_path TEXT, archived INTEGER)")
            db.execute(
                "INSERT INTO threads VALUES (?, ?, ?, 0)", (THREAD_ID, str(source), str(sessions / "missing.jsonl"))
            )
            db.execute("INSERT INTO threads VALUES (?, ?, ?, 1)", (OTHER_THREAD_ID, str(source), str(archived)))
        db_bytes = state.read_bytes()
        config = codex / "config.toml"
        config_before = f'project = "{source}"\n'
        config.write_text(config_before)
        result = self.run_script("new-repo", "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["codex_resume"], {})
        self.assertEqual(
            report["mutations"][-1]["files"],
            [{"path": str((self.repo / "README.md").resolve()), "occurrences": 1}],
        )
        result = self.run_script("new-repo", "--apply", "--confirm", report["confirmation_token"])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["codex_resume"], {})
        self.assertEqual(config.read_text(), config_before)
        self.assertEqual(archived.read_bytes(), archived_bytes)
        self.assertEqual(state.read_bytes(), db_bytes)

    def test_repo_scan_preserves_nested_codex_history(self) -> None:
        source = self.repo.resolve()
        codex = source / ".codex"
        self.env["CODEX_HOME"] = str(codex)
        active = codex / "sessions" / "rollout-active.jsonl"
        archived = codex / "archived_sessions" / "rollout-archived.jsonl.zst"
        history = {
            active: self.write_rollout(active, source),
            archived: self.write_rollout(archived, source, OTHER_THREAD_ID),
        }
        message = json.dumps({"text": str(source)}) + "\n"
        for name in ("history.jsonl", "session_index.jsonl", "state_5.sqlite-wal"):
            path = codex / name
            history[path] = message.encode()
            path.write_bytes(history[path])
        (codex / "config.toml").write_text(f'project = "{source}"\nname = "old-repo"\n')
        subprocess.run(["git", "add", "-f", ".codex"], cwd=source, check=True)
        subprocess.run(["git", "commit", "-qm", "nested codex fixture"], cwd=source, check=True)

        result = self.run_script("new-repo", "--dry-run", "--include-local-state")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        destination = source.with_name("new-repo")
        self.assertEqual(report["codex_resume"], {THREAD_ID: [str(destination)]})
        self.assertEqual(
            report["mutations"][-1]["files"],
            [
                {"path": str(codex / "config.toml"), "occurrences": 1},
                {"path": str(source / "README.md"), "occurrences": 1},
            ],
        )
        result = self.run_script(
            "new-repo", "--apply", "--include-local-state", "--confirm", report["confirmation_token"]
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        for original, before in history.items():
            self.assertEqual((destination / original.relative_to(source)).read_bytes(), before)
        self.assertEqual(
            (destination / ".codex/config.toml").read_text(), f'project = "{destination}"\nname = "old-repo"\n'
        )

    def collect(self, codex: Path, old_root: Path) -> list[tuple[str, int]]:
        env = self.env | {"PATH": os.environ["PATH"], "CODEX_HOME": str(codex)}
        with patch.dict(os.environ, env, clear=True):
            items = HELPER.collect_replacements(
                old_root, self.root / "new-repo", "old-repo", "new-repo", continuity_active=True
            )
        return [(str(item.path), item.occurrences) for item in items if not item.path.is_relative_to(old_root)]

    def test_prefilter_matches_full_scan(self) -> None:
        codex, old_root = self.continuity_fixture()
        if HELPER.shutil.which("rg") is None:
            self.skipTest("rg is not on PATH")
        filtered = self.collect(codex, old_root)
        with patch.object(HELPER, "prefilter", side_effect=lambda files, roots, pairs: files):
            full = self.collect(codex, old_root)
        self.assertEqual(filtered, full)
        names = {Path(path).name: count for path, count in filtered}
        self.assertEqual(names, {"hit.jsonl": 2, ".hidden.jsonl": 1, "config.toml": 1})

    def test_prefilter_falls_back_without_rg(self) -> None:
        codex, old_root = self.continuity_fixture()
        no_rg = self.root / "no-rg-bin"
        no_rg.mkdir()
        (no_rg / "git").symlink_to(HELPER.shutil.which("git") or "git")
        with patch.dict(os.environ, {"PATH": str(no_rg)}):
            self.assertIsNone(HELPER.shutil.which("rg"))
            files = [codex / "config.toml"]
            self.assertEqual(HELPER.prefilter(files, [codex], ((b"x", b"y"),)), files)
            fallback = self.collect(codex, old_root)
        names = {Path(path).name: count for path, count in fallback}
        self.assertEqual(names, {"hit.jsonl": 2, ".hidden.jsonl": 1, "config.toml": 1})

    def test_prefilter_falls_back_on_rg_error(self) -> None:
        codex, _ = self.continuity_fixture()
        bad = self.root / "bad-bin"
        bad.mkdir()
        rg = bad / "rg"
        rg.write_text("#!/bin/sh\nexit 2\n", encoding="utf-8")
        rg.chmod(rg.stat().st_mode | stat.S_IXUSR)
        files = [codex / "config.toml"]
        with patch.dict(os.environ, {"PATH": f"{bad}:{os.environ['PATH']}"}):
            self.assertEqual(HELPER.prefilter(files, [codex], ((b"x", b"y"),)), files)

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
        result = self.run_script(
            "new-repo", "--apply", "--include-local-state", "--confirm", "owner/old-repo->owner/new-repo"
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        for name in files:
            target = self.root / "new-repo" / name
            self.assertEqual(target.read_text(), "new-repo\n")
            self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o755)


if __name__ == "__main__":
    unittest.main()

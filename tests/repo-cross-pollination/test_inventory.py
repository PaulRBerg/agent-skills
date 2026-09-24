from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
INVENTORY = REPO_ROOT / "skills" / "repo-cross-pollination" / "scripts" / "inventory.ts"


def make_repo(path: Path, files: dict[str, str]) -> Path:
    path.mkdir(parents=True)
    for name, content in files.items():
        target = path / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(textwrap.dedent(content).lstrip())
    git(path, "init", "-q", "-b", "main")
    git(path, "add", "-A")
    git(path, "-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-q", "-m", "init")
    return path


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(repo), *args], check=True, text=True, capture_output=True).stdout


def run_inventory(*paths: Path | str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(["bun", "run", str(INVENTORY), *map(str, paths)], text=True, capture_output=True)


DONOR = {
    "AGENTS.md": "# Donor\n",
    ".claude/skills/review/SKILL.md": "---\nname: review\n---\n",
    ".claude/skills/review/scripts/run.sh": "echo run\n",
    ".github/workflows/ci.yml": "on: push\n",
    "justfile": "check:\n    echo check\n",
    "bun.lock": "{}\n",
    "package.json": json.dumps(
        {
            "scripts": {"lint": "oxlint ."},
            "dependencies": {"internal": "workspace:*"},
            "devDependencies": {"knip": "^5.0.0", "oxlint": "^1.0.0"},
        }
    ),
    "pyproject.toml": """
        [project]
        dependencies = ["Requests>=2", "internal-pkg"]

        [dependency-groups]
        dev = ["pytest>=8", { include-group = "lint" }]

        [tool.uv.sources]
        internal-pkg = { workspace = true }
    """,
    "Cargo.toml": """
        [dependencies]
        serde = "1"
        renamed = { package = "foo_bar", version = "1" }

        [dev-dependencies]
        local = { path = "crates/local" }

        [target.'cfg(unix)'.dependencies]
        libc = "0.2"
    """,
    "go.mod": """
        module example.com/donor

        require (
            github.com/spf13/cobra v1.8.0
            golang.org/x/sys v0.1.0 // indirect
        )

        tool golang.org/x/tools/cmd/stringer
    """,
}

RECIPIENT = {
    "package-lock.json": "{}\n",
    "package.json": json.dumps({"devDependencies": {"knip": "^5.1.0"}}),
    "pyproject.toml": """
        [tool.poetry.dependencies]
        python = "^3.12"
        requests = "^2.31"
    """,
    "Cargo.toml": """
        [dependencies]
        foo-bar = "1"
    """,
    "go.mod": """
        module example.com/recipient

        require github.com/stretchr/testify v1.9.0
    """,
}


class InventoryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        base = Path(self.tmp.name)
        self.donor = make_repo(base / "donor", DONOR)
        self.recipient = make_repo(base / "recipient", RECIPIENT)

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_reports_files_package_managers_and_dependency_gaps(self) -> None:
        result = run_inventory(self.donor, self.recipient)
        self.assertEqual(result.returncode, 0, result.stderr)
        data = json.loads(result.stdout)
        donor, recipient = data["repos"]

        self.assertEqual(donor["id"], "donor")
        self.assertEqual(donor["files"]["guidance"], [".claude/skills/review/SKILL.md", "AGENTS.md"])
        self.assertEqual(donor["files"]["workflow"], [".github/workflows/ci.yml", "justfile"])
        self.assertEqual(donor["packageManagers"], ["bun", "cargo", "go"])
        self.assertEqual(recipient["packageManagers"], ["cargo", "go", "npm"])
        self.assertIn({"file": "package.json", "name": "lint", "runner": "package.json"}, donor["tasks"])
        if shutil.which("just"):
            self.assertIn({"file": "justfile", "name": "check", "runner": "just"}, donor["tasks"])

        gaps = {(gap["ecosystem"], gap["name"]): gap for gap in data["dependencyGaps"]}
        self.assertEqual(
            sorted(gaps),
            [
                ("cargo", "libc"),
                ("cargo", "serde"),
                ("go", "github.com/spf13/cobra"),
                ("go", "github.com/stretchr/testify"),
                ("go", "golang.org/x/tools/cmd/stringer"),
                ("npm", "oxlint"),
                ("pypi", "pytest"),
            ],
        )
        self.assertEqual(gaps[("pypi", "pytest")]["missingIn"], ["recipient"])
        self.assertEqual(
            gaps[("pypi", "pytest")]["presentIn"],
            [
                {
                    "manifest": "pyproject.toml",
                    "repo": "donor",
                    "section": "dependency-groups.dev",
                    "spec": ">=8",
                    "uses": 1,
                }
            ],
        )
        self.assertEqual(gaps[("cargo", "libc")]["presentIn"][0]["section"], "target.cfg(unix).dependencies")
        self.assertEqual(gaps[("go", "golang.org/x/tools/cmd/stringer")]["presentIn"][0]["section"], "tool")
        self.assertEqual(
            data["summary"]["dependencies"]["pypi"], {"gaps": 1, "repos": ["donor", "recipient"], "shared": 1}
        )
        self.assertEqual(data["summary"]["dependencies"]["cargo"]["shared"], 1)
        self.assertEqual(data["summary"]["dependencies"]["npm"]["shared"], 1)
        self.assertIn({"missingIn": ["recipient"], "name": "lint", "presentIn": ["donor"]}, data["taskGaps"])

    def test_rejects_invalid_repository_sets(self) -> None:
        (self.donor / "sub").mkdir()
        cases = [
            ((self.donor,), "at least two repository paths"),
            ((self.donor, self.donor / "sub"), "not a repository root"),
            ((self.donor, self.donor), "same repository"),
            ((self.donor, Path(self.tmp.name) / "missing"), "directory not found"),
        ]
        for paths, message in cases:
            with self.subTest(message=message):
                result = run_inventory(*paths)
                self.assertEqual(result.returncode, 2)
                self.assertIn(message, result.stderr)
                self.assertEqual(result.stdout, "")


if __name__ == "__main__":
    unittest.main()

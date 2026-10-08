from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
INVENTORY = REPO_ROOT / "skills" / "repo-harmonization" / "scripts" / "inventory.ts"


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
    "uv.lock": "version = 1\n",
    "pyproject.toml": """
        [project]
        dependencies = ["requests>=2.31"]
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


LONG_COMMAND = "echo " + "x" * 100

TEMPLATE = {
    "package.json": json.dumps(
        {
            "scripts": {"lint": "oxlint .", "test": "vitest run", "long": LONG_COMMAND},
            "devDependencies": {"oxlint": "^1.0.0", "vitest": "^3.0.0"},
        }
    ),
    ".editorconfig": "root = true\n",
    ".oxlintrc.json": '{ "rules": {} }\n',
    "tsconfig.json": """
        {
          // Template compiler options.
          "$schema": "https://json.schemastore.org/tsconfig",
          "compilerOptions": { "strict": true, "target": "ES2022", },
        }
    """,
    "justfile": "check:\n    echo check\n\nlint:\n    echo lint\n",
    ".husky/pre-commit": "just check\n",
    ".github/workflows/ci.yml": "on: push\n",
}

PROJECT = {
    "package.json": json.dumps(
        {
            "scripts": {"lint": "oxlint .", "build": "tsc", "long": LONG_COMMAND},
            "devDependencies": {"oxlint": "^1.2.0", "typescript": "^5.0.0"},
        }
    ),
    ".editorconfig": "root = true\n",
    "tsconfig.json": '{ "compilerOptions": { "strict": true, "target": "ES2023", "noEmit": true } }\n',
    "justfile": "check:\n    echo check\n",
    "lefthook.yml": "pre-commit: {}\n",
    ".github/workflows/ci.yml": "on: [push, pull_request]\n",
    ".github/workflows/release.yml": "on: release\n",
}


def drift_rows(stdout: str) -> dict[str, tuple[str, str, str]]:
    rows: dict[str, tuple[str, str, str]] = {}
    for line in stdout.splitlines():
        if not line.startswith("| ") or line.startswith(("| surface", "| ---")):
            continue
        surface, template, project, status = [cell.strip() for cell in line.strip("|").split(" | ")]
        rows[surface] = (template, project, status)
    return rows


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
        self.assertEqual(recipient["packageManagers"], ["cargo", "go", "npm", "uv"])
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
                ("uv", "pytest"),
            ],
        )
        self.assertEqual(gaps[("uv", "pytest")]["missingIn"], ["recipient"])
        self.assertEqual(
            gaps[("uv", "pytest")]["presentIn"],
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
            data["summary"]["dependencies"]["uv"], {"gaps": 1, "repos": ["donor", "recipient"], "shared": 1}
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



class DriftTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        base = Path(self.tmp.name)
        self.template = make_repo(base / "template", TEMPLATE)
        self.project = make_repo(base / "project", PROJECT)

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_drift_table_compares_surfaces_in_both_directions(self) -> None:
        result = run_inventory("--drift", self.template, self.project)
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = drift_rows(result.stdout)
        statuses = {surface: row[2] for surface, row in rows.items()}

        expected = {
            "package.json scripts.lint": "same",
            "package.json scripts.test": "missing-in-project",
            "package.json scripts.build": "missing-in-template",
            "package.json devDependencies.oxlint": "differs",
            "package.json devDependencies.vitest": "missing-in-project",
            "package.json devDependencies.typescript": "missing-in-template",
            ".editorconfig": "same",
            ".oxlintrc.json": "missing-in-project",
            "tsconfig.json": "differs",
            "tsconfig.json compilerOptions.strict": "same",
            "tsconfig.json compilerOptions.target": "differs",
            "tsconfig.json compilerOptions.noEmit": "missing-in-template",
            ".husky/pre-commit": "missing-in-project",
            "lefthook.yml": "missing-in-template",
            ".github/workflows/ci.yml": "differs",
            ".github/workflows/release.yml": "missing-in-template",
        }
        for surface, status in expected.items():
            with self.subTest(surface=surface):
                self.assertEqual(statuses.get(surface), status)
        self.assertEqual(rows["package.json devDependencies.oxlint"][:2], ("^1.0.0", "^1.2.0"))
        self.assertEqual(rows["tsconfig.json compilerOptions.target"][:2], ('"ES2022"', '"ES2023"'))
        self.assertEqual(rows["package.json scripts.test"][1], "-")
        long_value = rows["package.json scripts.long"][0]
        self.assertTrue(long_value.endswith("... (+45 chars)"), long_value)
        self.assertEqual(rows["package.json scripts.long"][2], "same")
        if shutil.which("just"):
            self.assertEqual(statuses["justfile recipe check"], "same")
            self.assertEqual(statuses["justfile recipe lint"], "missing-in-project")
        surfaces = list(rows)
        self.assertLess(surfaces.index("package.json scripts.lint"), surfaces.index(".editorconfig"))
        self.assertLess(surfaces.index("tsconfig.json"), surfaces.index(".github/workflows/ci.yml"))
        self.assertIn("rows: ", result.stdout)

    def test_drift_rejects_invalid_arguments(self) -> None:
        cases = [
            (("--drift", self.template), "exactly two paths"),
            (("--drift", self.template, self.template), "same repository"),
            (("--drift", self.template, Path(self.tmp.name) / "missing"), "directory not found"),
        ]
        for paths, message in cases:
            with self.subTest(message=message):
                result = run_inventory(*paths)
                self.assertEqual(result.returncode, 2)
                self.assertIn(message, result.stderr)
                self.assertEqual(result.stdout, "")

if __name__ == "__main__":
    unittest.main()

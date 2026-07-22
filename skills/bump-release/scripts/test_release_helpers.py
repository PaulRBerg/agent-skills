from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent
PLANNER = ROOT / "plan-release.mjs"
FINALIZER = ROOT / "finalize-release-plan.py"
VALIDATOR = ROOT / "validate-changelog.py"


def load_script(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


finalizer = load_script("finalize_release_plan", FINALIZER)
validator = load_script("validate_changelog", VALIDATOR)


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(repo), *args], check=True, text=True, capture_output=True).stdout


def discovery(*, beta: bool = False) -> dict:
    packages = [
        {"id": "a", "dir": "packages/a", "name": "@scope/a", "version": "1.2.3"},
        {"id": "b", "dir": "packages/b", "name": "@scope/b", "version": "2.0.0"},
    ]
    return {
        "schemaVersion": 2,
        "beta": beta,
        "explicitVersion": None,
        "packages": packages,
        "targets": [packages[0]],
        "dependencyEdges": [
            {"from": "b", "to": "a", "type": "dependencies", "name": "@scope/a", "range": "^1.2.3"}
        ],
        "previousTags": {},
    }


class PlannerTests(unittest.TestCase):
    def test_changed_files_are_complete_and_hints_are_not_authoritative(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            git(repo, "init", "-q", "-b", "main")
            git(repo, "config", "user.name", "Test")
            git(repo, "config", "user.email", "test@example.com")
            (repo / "package.json").write_text('{"name":"demo","version":"1.0.0"}\n')
            (repo / "Dockerfile").write_text("FROM node:22\n")
            (repo / "src.js").write_text("export const value = 1;\n")
            (repo / "src.test.js").write_text("test('x', () => {});\n")
            git(repo, "add", "package.json", "Dockerfile", "src.js", "src.test.js")
            git(repo, "commit", "-qm", "initial")
            git(repo, "tag", "v1.0.0")
            for path in ("Dockerfile", "src.js", "src.test.js"):
                (repo / path).write_text((repo / path).read_text() + "# changed\n")
            git(repo, "add", "Dockerfile", "src.js", "src.test.js")
            git(repo, "commit", "-qm", "change runtime tooling and tests")
            result = subprocess.run(
                ["node", str(PLANNER), "--cwd", str(repo)], check=True, text=True, capture_output=True
            )
            payload = json.loads(result.stdout)
            self.assertEqual(payload["schemaVersion"], 2)
            self.assertEqual(payload["changedFiles"]["demo"], ["Dockerfile", "src.js", "src.test.js"])
            self.assertNotIn("includedFiles", payload)
            self.assertNotIn("excludedFiles", payload)
            self.assertFalse(payload["changeHints"]["authoritative"])


class FinalizerTests(unittest.TestCase):
    def test_explicit_beta_and_promotion_transitions(self) -> None:
        payload = discovery(beta=True)
        result = finalizer.finalize(payload, ["a=3.0.0"])
        self.assertEqual(result["versions"]["a"]["planned"], "3.0.0-beta.1")
        payload["packages"][0]["version"] = payload["targets"][0]["version"] = "1.2.3-beta.4"
        self.assertEqual(finalizer.finalize(payload, [])["versions"]["a"]["planned"], "1.2.3-beta.5")
        payload["beta"] = False
        self.assertEqual(finalizer.finalize(payload, [])["versions"]["a"]["planned"], "1.2.3")

    def test_ranges_and_dependency_order(self) -> None:
        payload = discovery()
        payload["dependencyEdges"] = [
            {"from": "b", "to": "a", "type": "dependencies", "name": "a", "range": "~1.2.3"},
            {"from": "b", "to": "a", "type": "peerDependencies", "name": "a", "range": "^1.2.3"},
            {"from": "b", "to": "a", "type": "dependencies", "name": "a", "range": ">=1 <2"},
            {"from": "b", "to": "a", "type": "dependencies", "name": "a", "range": "workspace:^1.2.3"},
        ]
        result = finalizer.finalize(payload, ["a=2.0.0"])
        self.assertEqual(result["dependencyOrder"], ["a", "b"])
        by_range = {item["range"]: item for item in result["workspaceEdges"]}
        self.assertEqual(by_range["~1.2.3"]["suggestedRange"], "~2.0.0")
        self.assertIsNone(by_range["^1.2.3"]["suggestedRange"])
        self.assertEqual(by_range["^1.2.3"]["decision"], "choose peer dependency range policy")
        self.assertIsNone(by_range[">=1 <2"]["satisfied"])
        self.assertEqual(by_range["workspace:^1.2.3"]["suggestedRange"], "workspace:^2.0.0")
        self.assertFalse(finalizer.satisfies("2.0.0-beta.1", "^2.0.0"))
        cascaded = finalizer.finalize(payload, ["a=2.0.0", "b=2.0.1"])
        self.assertEqual(cascaded["versions"]["b"]["planned"], "2.0.1")

    def test_cycles_are_reported_without_failing(self) -> None:
        payload = discovery()
        payload["targets"] = payload["packages"]
        payload["dependencyEdges"] = [
            {"from": "a", "to": "b", "type": "dependencies", "name": "b", "range": "^2.0.0"},
            {"from": "b", "to": "a", "type": "dependencies", "name": "a", "range": "^1.2.3"},
        ]
        result = finalizer.finalize(payload, ["a=1.3.0", "b=2.1.0"])
        self.assertEqual(result["dependencyCycles"], [["a", "b"]])


class ChangelogTests(unittest.TestCase):
    def test_validates_structure_and_release_link(self) -> None:
        text = """# Changelog

## [2.0.0] - 2026-07-22

### Changed

- Change the runtime contract

### Fixed

- Fix startup ordering

[2.0.0]: https://github.com/acme/demo/releases/tag/v2.0.0
"""
        self.assertEqual(validator.validate(text, "2.0.0", "2026-07-22", "v2.0.0"), [])
        broken = text.replace("### Changed", "### Fixed", 1)
        errors = validator.validate(broken, "2.0.0", "2026-07-22", "v2.0.0")
        self.assertTrue(any("repeat" in error or "order" in error for error in errors))
        self.assertTrue(validator.validate(text, "2.0.0", "2026-07-22", "release-2.0.0"))
        self.assertTrue(validator.validate(text.replace("[2.0.0]", "2.0.0", 1), "2.0.0", "2026-07-22", None))


if __name__ == "__main__":
    unittest.main()

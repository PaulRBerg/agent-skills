from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "skills" / "skill-doctor" / "scripts" / "skill-doctor.py"


def run_doctor(root: Path, *args: str) -> tuple[subprocess.CompletedProcess[str], dict[str, object] | None]:
    result = subprocess.run(
        ["uv", "run", str(SCRIPT), "--root", str(root), "--format", "json", *args],
        text=True,
        capture_output=True,
        check=False,
    )
    report = json.loads(result.stdout) if result.stdout else None
    return result, report


class SkillDoctorAdvisoryTests(unittest.TestCase):
    def make_catalog(self, model: str, body: str, *, coordination: str | None = None) -> Path:
        root = Path(self.temp.name)
        skill = root / "skills/demo"
        (skill / "agents").mkdir(parents=True)
        (root / "README.md").write_text(
            "# Catalog\n\n## Skills\n\n| Skill | Description |\n| ----- | ----------- |\n| demo | Demo |\n",
            encoding="utf-8",
        )
        coordination_line = f"coordination: {coordination}\n" if coordination is not None else ""
        (skill / "SKILL.md").write_text(
            "---\n"
            f"{coordination_line}"
            "disable-model-invocation: false\n"
            f"model: {model}\n"
            "name: demo\n"
            "description: Demo.\n"
            "---\n\n"
            f"# Demo\n\n{body}\n",
            encoding="utf-8",
        )
        (skill / "agents/openai.yaml").write_text(
            "policy:\n  allow_implicit_invocation: true\n", encoding="utf-8"
        )
        return root

    def run_doctor(self, root: Path) -> dict[str, object]:
        _, report = run_doctor(root)
        assert report is not None
        return report

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_warns_for_stale_pin_and_missing_completion(self) -> None:
        report = self.run_doctor(self.make_catalog("opus", "Do the work."))
        codes = {item["code"] for item in report["findings"]}
        self.assertIn("STALE_MODEL_PIN", codes)
        self.assertIn("COMPLETION_EVIDENCE_MISSING", codes)

    def test_accepts_current_pin_with_completion_contract(self) -> None:
        report = self.run_doctor(self.make_catalog("sonnet", "## Completion\n\nReport verified output."))
        self.assertEqual(report["counts"]["findings"], 0)

    def test_accepts_matching_coordination_exemption(self) -> None:
        sentence = "This skill is coordination-exempt: skip the ai-coord gate for its declared work."
        report = self.run_doctor(
            self.make_catalog("sonnet", f"{sentence}\n\n## Completion\n\nReport verified output.", coordination="exempt")
        )
        self.assertEqual(report["counts"]["findings"], 0)

    def test_reports_frontmatter_without_coordination_sentence(self) -> None:
        report = self.run_doctor(
            self.make_catalog("sonnet", "## Completion\n\nReport verified output.", coordination="exempt")
        )
        finding = next(item for item in report["findings"] if item["code"] == "COORDINATION_EXEMPT_SENTENCE_MISSING")
        self.assertFalse(finding["fixable"])
        self.assertIn("This skill is coordination-exempt:", finding["message"])

    def test_reports_coordination_sentence_without_frontmatter(self) -> None:
        body = "This skill is coordination-exempt: skip the gate.\n\n## Completion\n\nReport verified output."
        report = self.run_doctor(self.make_catalog("sonnet", body))
        codes = {item["code"] for item in report["findings"]}
        self.assertIn("COORDINATION_EXEMPT_FRONTMATTER_MISSING", codes)
        self.assertIn("COORDINATION_EXEMPT_SENTENCE_DRIFT", codes)

    def test_reports_drifted_coordination_sentence_with_expected_text(self) -> None:
        body = "This skill is coordination-exempt: skip the coordination gate.\n\n## Completion\n\nReport verified output."
        report = self.run_doctor(self.make_catalog("sonnet", body, coordination="exempt"))
        finding = next(item for item in report["findings"] if item["code"] == "COORDINATION_EXEMPT_SENTENCE_DRIFT")
        self.assertFalse(finding["fixable"])
        self.assertIn("expected: This skill is coordination-exempt:", finding["message"])


class SkillDoctorDependencyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_skill(self, name: str, declaration: str | None = None) -> None:
        skill = self.root / "skills" / name
        skill.mkdir(parents=True)
        dependency_field = f"skill-dependencies: {declaration}\n" if declaration is not None else ""
        (skill / "SKILL.md").write_text(
            "---\n"
            "disable-model-invocation: false\n"
            f"name: {name}\n"
            f"{dependency_field}"
            f"description: {name}.\n"
            "---\n\n"
            f"# {name}\n",
            encoding="utf-8",
        )

    def dependency_findings(self) -> list[dict[str, object]]:
        _, report = run_doctor(self.root, "--dependencies-only")
        assert report is not None
        return report["findings"]

    def test_accepts_absent_and_valid_mixed_declarations(self) -> None:
        self.write_skill(
            "alpha",
            "[beta, Acme/Tools#codebase-design, code-polish, Acme/Tools#shared, shared]",
        )
        self.write_skill("beta")
        self.write_skill("code-polish")
        self.write_skill("shared")

        result, report = run_doctor(self.root, "--dependencies-only")

        self.assertEqual(result.returncode, 0)
        assert report is not None
        self.assertEqual(report["counts"]["findings"], 0)

    def test_rejects_scalar_and_empty_declarations(self) -> None:
        self.write_skill("alpha", "beta")
        self.write_skill("beta", "[]")

        codes = {item["code"] for item in self.dependency_findings()}

        self.assertIn("SKILL_DEPENDENCIES_NOT_ARRAY", codes)
        self.assertIn("SKILL_DEPENDENCIES_EMPTY", codes)

    def test_rejects_non_string_and_malformed_external_identifiers(self) -> None:
        self.write_skill("alpha", "[123, Acme/Tools, Acme/Tools#Bad_Skill]")

        codes = {item["code"] for item in self.dependency_findings()}

        self.assertIn("SKILL_DEPENDENCY_NOT_STRING", codes)
        invalid_findings = [item for item in self.dependency_findings() if item["code"] == "SKILL_DEPENDENCY_INVALID"]
        self.assertEqual(len(invalid_findings), 2)

    def test_rejects_duplicates_wrong_order_self_and_unresolved_bare_targets(self) -> None:
        self.write_skill("alpha", "[zeta, alpha, beta, beta]")
        self.write_skill("beta")

        codes = {item["code"] for item in self.dependency_findings()}

        self.assertIn("SKILL_DEPENDENCY_DUPLICATE", codes)
        self.assertIn("SKILL_DEPENDENCIES_ORDER", codes)
        self.assertIn("SKILL_DEPENDENCY_SELF", codes)
        self.assertIn("SKILL_DEPENDENCY_UNRESOLVED", codes)

    def test_rejects_misplaced_dependency_field(self) -> None:
        self.write_skill("alpha", "[beta]")
        self.write_skill("beta")
        skill_path = self.root / "skills" / "alpha" / "SKILL.md"
        text = skill_path.read_text(encoding="utf-8")
        skill_path.write_text(
            text.replace(
                "name: alpha\nskill-dependencies: [beta]\n",
                "skill-dependencies: [beta]\nname: alpha\n",
            ),
            encoding="utf-8",
        )

        codes = {item["code"] for item in self.dependency_findings()}

        self.assertIn("SKILL_DEPENDENCIES_FIELD_ORDER", codes)

    def test_dependencies_only_excludes_unrelated_hygiene_findings(self) -> None:
        self.write_skill("alpha")

        result, report = run_doctor(self.root, "--dependencies-only")

        self.assertEqual(result.returncode, 0)
        assert report is not None
        self.assertEqual(report["findings"], [])


if __name__ == "__main__":
    unittest.main()

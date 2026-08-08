from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "skills" / "skill-doctor" / "scripts" / "skill-doctor.py"


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
        result = subprocess.run(
            ["uv", "run", str(SCRIPT), "--root", str(root), "--format", "json"],
            text=True,
            capture_output=True,
            check=False,
        )
        return json.loads(result.stdout)

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
        sentence = (
            "This skill is coordination-exempt: skip the ai-coord gate "
            "(`git status` / `ai-coord status` / `ai-coord start`) for this skill's own work."
        )
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


if __name__ == "__main__":
    unittest.main()

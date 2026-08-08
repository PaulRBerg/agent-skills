#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = ["pyyaml>=6.0"]
# ///

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path
from contextlib import redirect_stderr
from io import StringIO
from unittest.mock import patch


REPO_ROOT = Path(__file__).resolve().parents[2]
SKILL_ROOT = REPO_ROOT / "skills" / "yeet"
SCRIPT = SKILL_ROOT / "scripts" / "issue-form.py"
FIXTURE = SKILL_ROOT / "fixtures" / "issue-form.yml"
SPEC = importlib.util.spec_from_file_location("issue_form", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules["issue_form"] = MODULE
SPEC.loader.exec_module(MODULE)


class IssueFormTests(unittest.TestCase):
    def setUp(self) -> None:
        self.form = MODULE.inspect_form(FIXTURE.read_text(), "acme/demo", "bug.yml")

    def test_inspects_metadata_fields_options_render_and_attestations(self) -> None:
        self.assertEqual(self.form["titlePrefix"], "[BUG] ")
        self.assertEqual(self.form["labels"], ["bug"])
        self.assertEqual(self.form["issueType"], "Bug")
        by_id = {field["id"]: field for field in self.form["fields"]}
        self.assertIn("__field_1", by_id)
        self.assertTrue(by_id["summary"]["required"])
        self.assertEqual(by_id["logs"]["render"], "shell")
        self.assertTrue(by_id["affected-surfaces"]["multiple"])
        self.assertTrue(by_id["terms"]["checkboxAttestations"][0]["required"])

    def test_renders_exact_body_and_posting_metadata(self) -> None:
        answers = {
            "summary": "Startup fails",
            "logs": "error: boom",
            "operating-system": "macOS",
            "affected-surfaces": ["CLI", "Extension"],
            "terms": {"I searched for duplicates": True},
        }
        result = MODULE.render_form(self.form, answers)
        expected = """### Summary

Startup fails

### Logs

```shell
error: boom
```

### Operating system

macOS

### Affected surfaces

CLI, Extension

### Attestations

- [x] I searched for duplicates
- [ ] I can provide more details
"""
        self.assertEqual(result["body"], expected)
        self.assertEqual(
            result["posting"],
            {
                "titlePrefix": "[BUG] ",
                "labels": ["bug"],
                "assignees": [],
                "projects": [],
                "issueType": "Bug",
            },
        )

    def test_rejects_missing_invalid_and_unverified_answers(self) -> None:
        base = {"summary": "x", "operating-system": "macOS"}
        with self.assertRaisesRegex(MODULE.FormError, "unverified"):
            MODULE.render_form(self.form, base | {"terms": ["I searched for duplicates"]})
        with self.assertRaisesRegex(MODULE.FormError, "missing required"):
            MODULE.render_form(self.form, {"terms": {"I searched for duplicates": True}, "operating-system": "macOS"})
        with self.assertRaisesRegex(MODULE.FormError, "invalid dropdown"):
            MODULE.render_form(self.form, base | {"terms": {"I searched for duplicates": True}, "operating-system": "Windows"})

    def test_inspects_metadata_defaults_and_upload_accept(self) -> None:
        form = MODULE.inspect_form(
            """
name: Example
description: Example form
labels: bug, needs-triage, bug
assignees: [alice, bob, alice]
projects: "acme/1, acme/2, acme/1"
body:
  - type: input
    id: context
    attributes:
      label: Context
      value: prefilled
  - type: textarea
    id: details
    attributes:
      label: Details
      value: |-
        default details
  - type: dropdown
    id: version
    attributes:
      label: Version
      options: [one, two]
      default: 1
  - type: upload
    id: attachments
    attributes:
      label: Attachments
    validations:
      accept: .png,.txt
""",
            "acme/demo",
            "example.yml",
        )
        self.assertEqual(form["labels"], ["bug", "needs-triage"])
        self.assertEqual(form["assignees"], ["alice", "bob"])
        self.assertEqual(form["projects"], ["acme/1", "acme/2"])
        fields = {field["id"]: field for field in form["fields"]}
        self.assertEqual(fields["context"]["value"], "prefilled")
        self.assertEqual(fields["details"]["value"], "default details")
        self.assertEqual(fields["version"]["default"], 1)
        self.assertEqual(fields["attachments"]["accept"], ".png,.txt")
        rendered = MODULE.render_form(form, {"attachments": ["a.png", "b.txt"]})
        self.assertEqual(rendered["posting"]["assignees"], ["alice", "bob"])
        self.assertEqual(rendered["posting"]["projects"], ["acme/1", "acme/2"])
        self.assertIn("two", MODULE.render_form(form, {})["body"])

    def test_rejects_schema_constraints(self) -> None:
        cases = [
            ("id: bad.id", "invalid id"),
            ("id: ''", "invalid id"),
            ("options: [one, one]", "distinct"),
            ("options: []", "options must be strings"),
            ("default: true", "default must be an integer"),
            ("default: 2", "out of bounds"),
            ("multiple: 1", "multiple must be a boolean"),
            ("required: 1", "required must be a boolean"),
            ("accept: [.png]", "accept must be a non-empty string"),
        ]
        for setting, message in cases:
            with self.subTest(setting=setting):
                if setting.startswith("accept"):
                    text = f"""
body:
  - type: upload
    id: files
    attributes: {{label: Files}}
    validations:
      {setting}
"""
                elif setting.startswith("required"):
                    text = f"""
body:
  - type: input
    id: value
    attributes: {{label: Value}}
    validations:
      {setting}
"""
                elif setting.startswith(("options", "default", "multiple")):
                    text = f"""
body:
  - type: dropdown
    id: choice
    attributes:
      label: Choice
      options: [one, two]
      {setting}
"""
                else:
                    text = f"""
body:
  - type: dropdown
    {setting}
    attributes:
      label: Choice
      options: [one, two]
"""
                with self.assertRaisesRegex(MODULE.FormError, message):
                    MODULE.inspect_form(text)

        with self.assertRaisesRegex(MODULE.FormError, "OWNER/NUMBER"):
            MODULE.inspect_form("body: []\nprojects: [https://github.com/orgs/acme/projects/2]")
        with self.assertRaisesRegex(MODULE.FormError, "invalid render mode"):
            MODULE.inspect_form(
                """
body:
  - type: textarea
    id: logs
    attributes:
      label: Logs
      render: "text\\n### injected"
"""
            )

    def test_checkbox_selection_and_verification_must_agree(self) -> None:
        with self.assertRaisesRegex(MODULE.FormError, "must be selected"):
            MODULE.render_form(
                self.form,
                {
                    "summary": "x",
                    "operating-system": "macOS",
                    "terms": {"selected": ["I searched for duplicates"], "verified": ["I can provide more details"]},
                },
            )
        with self.assertRaisesRegex(MODULE.FormError, "invalid checkbox value"):
            MODULE.render_form(
                self.form,
                {
                    "summary": "x",
                    "operating-system": "macOS",
                    "terms": {"selected": ["unknown"], "verified": []},
                },
            )
        with self.assertRaisesRegex(MODULE.FormError, "invalid verified checkbox"):
            MODULE.render_form(
                self.form,
                {
                    "summary": "x",
                    "operating-system": "macOS",
                    "terms": {
                        "selected": ["I searched for duplicates"],
                        "verified": ["unknown"],
                    },
                },
            )
        with self.assertRaisesRegex(MODULE.FormError, "not selected"):
            MODULE.render_form(
                self.form,
                {
                    "summary": "x",
                    "operating-system": "macOS",
                    "terms": {"selected": [], "verified": []},
                },
            )

    def test_render_fence_exceeds_backtick_runs(self) -> None:
        form = MODULE.inspect_form(
            """
body:
  - type: textarea
    id: logs
    attributes:
      label: Logs
      render: text
"""
        )
        answer = "before\n````\nafter"
        body = MODULE.render_form(form, {"logs": answer})["body"]
        self.assertIn("`````text", body)
        self.assertTrue(body.endswith("`````\n"))

    def test_malformed_gh_responses_and_missing_executable_are_form_errors(self) -> None:
        args = Namespace(input=None, fixture=None, repo="acme/demo", template="bug.yml")
        with patch.object(MODULE.subprocess, "run", side_effect=FileNotFoundError):
            with self.assertRaisesRegex(MODULE.FormError, "gh executable not found"):
                MODULE.load_yaml_text(args)
        malformed = ["[]", '{"content": 4}', '{"type": "dir", "content": "eA=="}', '{"content": "not base64!"}']
        for output in malformed:
            with self.subTest(output=output):
                result = subprocess.CompletedProcess(["gh"], 0, stdout=output, stderr="")
                with patch.object(MODULE.subprocess, "run", return_value=result):
                    with self.assertRaises(MODULE.FormError):
                        MODULE.load_yaml_text(args)

    def test_malformed_render_forms_are_form_errors(self) -> None:
        bad_forms = [
            None,
            {"schemaVersion": 1, "fields": [{}]},
            {"schemaVersion": 1, "fields": [{"id": "x", "type": "input"}]},
            {"schemaVersion": 1, "fields": [{"id": "x", "type": "input", "label": "X", "required": True}]},
        ]
        for form in bad_forms:
            with self.subTest(form=form):
                with self.assertRaises(MODULE.FormError):
                    MODULE.render_form(form, {})

    def test_cli_maps_malformed_render_form_to_exit_64(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            form_path = Path(directory) / "form.json"
            answers_path = Path(directory) / "answers.json"
            form_path.write_text('{"schemaVersion": 1, "fields": [{}]}', encoding="utf-8")
            answers_path.write_text("{}", encoding="utf-8")
            with patch.object(
                sys,
                "argv",
                ["issue-form.py", "render", "--form", str(form_path), "--answers", str(answers_path)],
            ):
                with redirect_stderr(StringIO()):
                    self.assertEqual(MODULE.main(), 64)


if __name__ == "__main__":
    unittest.main()

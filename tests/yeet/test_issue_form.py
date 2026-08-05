#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = ["pyyaml>=6.0"]
# ///

from __future__ import annotations

import importlib.util
import json
import sys
import unittest
from pathlib import Path


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
        self.assertEqual(result["posting"], {"titlePrefix": "[BUG] ", "labels": ["bug"], "issueType": "Bug"})

    def test_rejects_missing_invalid_and_unverified_answers(self) -> None:
        base = {"summary": "x", "operating-system": "macOS"}
        with self.assertRaisesRegex(MODULE.FormError, "unverified"):
            MODULE.render_form(self.form, base | {"terms": ["I searched for duplicates"]})
        with self.assertRaisesRegex(MODULE.FormError, "missing required"):
            MODULE.render_form(self.form, {"terms": {"I searched for duplicates": True}, "operating-system": "macOS"})
        with self.assertRaisesRegex(MODULE.FormError, "invalid dropdown"):
            MODULE.render_form(self.form, base | {"terms": {"I searched for duplicates": True}, "operating-system": "Windows"})


if __name__ == "__main__":
    unittest.main()

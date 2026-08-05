from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "skills" / "autoresearch" / "scripts" / "autoresearch-session.py"
SPEC = importlib.util.spec_from_file_location("autoresearch_session", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules["autoresearch_session"] = MODULE
SPEC.loader.exec_module(MODULE)


class SessionTests(unittest.TestCase):
    def run_cli(self, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run([sys.executable, str(SCRIPT), *args], text=True, capture_output=True)
        if check and result.returncode:
            self.fail(result.stderr)
        return result

    def test_directions_zero_negative_convergence_and_segments(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "session.jsonl"
            self.run_cli(
                "init", "--file", str(path), "--metric", "latency", "--direction", "lower", "--max-runs", "5",
                "--convergence-runs", "2",
            )
            for metric, status in ((0, "keep"), (-1, "keep"), (-0.5, "discard"), (-0.25, "discard")):
                self.run_cli("record", "--file", str(path), "--metric", str(metric), "--status", status)
            summary = json.loads(self.run_cli("status", "--file", str(path)).stdout)
            self.assertEqual(summary["direction"], "lower")
            self.assertEqual(summary["baseline"], 0)
            self.assertEqual(summary["best"], -1)
            self.assertTrue(summary["converged"])
            self.assertEqual(summary["progress"]["bar"], "████████░░")

            self.run_cli(
                "record", "--file", str(path), "--metric", "10", "--metric-name", "throughput",
                "--direction", "higher", "--status", "keep",
            )
            segmented = json.loads(self.run_cli("status", "--file", str(path)).stdout)
            self.assertEqual(segmented["segment"], 1)
            self.assertEqual(segmented["direction"], "higher")
            self.assertEqual(segmented["best"], 10)
            self.assertEqual(segmented["budgets"]["runs"]["used"], 5)
            self.assertFalse(segmented["budgets"]["cost"]["exhausted"])

    def test_malformed_jsonl_and_compatibility_wrappers(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "session.jsonl"
            path.write_text("not json\n", encoding="utf-8")
            result = self.run_cli("status", "--file", str(path), check=False)
            self.assertEqual(result.returncode, 64)
            self.run_cli("init", "--file", str(path.with_name("good.jsonl")), "--metric", "score", "--direction", "higher")
            good = path.with_name("good.jsonl")
            for metric in (1, 2, 3):
                self.run_cli("record", "--file", str(good), "--metric", str(metric), "--status", "keep")
            for wrapper in ("summary.sh", "confidence.sh"):
                result = subprocess.run(["bash", str(SCRIPT.with_name(wrapper)), str(good)], text=True, capture_output=True)
                self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()

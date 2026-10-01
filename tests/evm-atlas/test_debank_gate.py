from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "skills" / "evm-atlas" / "scripts" / "debank-gate.py"
SPEC = importlib.util.spec_from_file_location("debank_gate", SCRIPT)
assert SPEC and SPEC.loader
GATE = importlib.util.module_from_spec(SPEC)
sys.modules["debank_gate"] = GATE
SPEC.loader.exec_module(GATE)


def fresh() -> dict:
    return {"queue": [], "lease": None, "cooldownUntil": 0}


class QueueTests(unittest.TestCase):
    def test_grants_in_fifo_order_one_lease_at_a_time(self) -> None:
        state = fresh()
        a = GATE.enqueue(state, "a", 1, 0)
        b = GATE.enqueue(state, "b", 1, 0)
        self.assertEqual(GATE.try_grant(state, b, 600, 1), "queued")
        self.assertEqual(GATE.try_grant(state, a, 600, 1), "granted")
        self.assertEqual(GATE.try_grant(state, a, 600, 2), "granted")
        self.assertEqual(GATE.try_grant(state, b, 600, 2), "queued")
        GATE.release(state, a, 3)
        self.assertEqual(GATE.try_grant(state, b, 600, 3), "granted")

    def test_expired_lease_frees_the_gate_and_cannot_be_renewed(self) -> None:
        state = fresh()
        a = GATE.enqueue(state, "a", 1, 0)
        b = GATE.enqueue(state, "b", 1, 0)
        self.assertEqual(GATE.try_grant(state, a, 10, 0), "granted")
        self.assertTrue(GATE.renew(state, a, 10, 5))
        self.assertEqual(GATE.try_grant(state, b, 600, 14), "queued")
        self.assertEqual(GATE.try_grant(state, b, 600, 15), "granted")
        self.assertFalse(GATE.renew(state, a, 10, 16))

    def test_abandoned_ticket_loses_its_place(self) -> None:
        state = fresh()
        abandoned = GATE.enqueue(state, "abandoned", 1, 0)
        waiting = GATE.enqueue(state, "waiting", 1, 0)
        state["lease"] = {"ticket": "holder", "label": "h", "profiles": 1, "grantedAt": 0, "expiresAt": 60}
        self.assertEqual(GATE.try_grant(state, waiting, 600, 100), "queued")
        self.assertEqual(GATE.try_grant(state, waiting, 600, GATE.STALE_TICKET_SECONDS + 1), "granted")
        self.assertEqual(GATE.try_grant(state, abandoned, 600, GATE.STALE_TICKET_SECONDS + 2), "unknown")

    def test_block_releases_the_holder_and_pauses_everyone(self) -> None:
        state = fresh()
        a = GATE.enqueue(state, "a", 25, 0)
        b = GATE.enqueue(state, "b", 1, 0)
        self.assertEqual(GATE.try_grant(state, a, 600, 0), "granted")
        GATE.block(state, a, 15, 10)
        self.assertIsNone(state["lease"])
        for now in range(10, 10 + 15 * 60, 60):
            self.assertEqual(GATE.try_grant(state, b, 600, now), "queued")
        self.assertEqual(GATE.try_grant(state, b, 600, 10 + 15 * 60), "granted")


class CliTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.env = {**os.environ, "DEBANK_GATE_DIR": self.tmp.name}

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def run_gate(self, *args: str) -> tuple[int, dict]:
        proc = subprocess.run(
            [sys.executable, str(SCRIPT), *args], env=self.env, capture_output=True, text=True, check=False
        )
        return proc.returncode, json.loads(proc.stdout) if proc.stdout else {}

    def test_second_agent_queues_then_resumes_its_ticket(self) -> None:
        code, first = self.run_gate("acquire", "--label", "bulk", "--profiles", "25", "--wait", "0")
        self.assertEqual((code, first["status"]), (0, "granted"))

        code, second = self.run_gate("acquire", "--label", "single", "--wait", "0")
        self.assertEqual(code, GATE.EXIT_QUEUED)
        self.assertEqual((second["status"], second["position"], second["holder"]["label"]), ("queued", 1, "bulk"))

        self.assertEqual(self.run_gate("release", "--ticket", first["ticket"])[0], 0)
        code, resumed = self.run_gate("acquire", "--label", "single", "--ticket", second["ticket"], "--wait", "0")
        self.assertEqual((code, resumed["status"], resumed["ticket"]), (0, "granted", second["ticket"]))

        code, lost = self.run_gate("renew", "--ticket", first["ticket"])
        self.assertEqual((code, lost["status"]), (GATE.EXIT_LOST, "lost"))

    def test_rejects_batches_over_the_profile_cap(self) -> None:
        code, _ = self.run_gate("acquire", "--label", "bulk", "--profiles", str(GATE.MAX_PROFILES + 1), "--wait", "0")
        self.assertNotEqual(code, 0)


if __name__ == "__main__":
    unittest.main()

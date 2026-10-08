from __future__ import annotations

import copy
import http.client
import importlib.util
import io
import json
import os
import signal
import subprocess
import sys
import unittest
import urllib.error
from pathlib import Path
from unittest.mock import Mock, patch


SCRIPT = Path(__file__).resolve().parents[2] / "skills/orchestration/scripts/select-model.py"
SPEC = importlib.util.spec_from_file_location("select_model", SCRIPT)
ROUTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ROUTER)
PAYLOAD = {
    "task": "Implement a bounded parser correction and its regression test.",
    "context": "Implementation. Native Codex. No external writes.",
    "candidates": [
        {"id": "routine", "model": "gpt-6-luna", "effort": "high", "description": "Routine work."},
        {"id": "involved", "model": "gpt-6.1-sol", "effort": "medium", "description": "Involved work."},
    ],
}
ANSWER = {"answers": {"configuration": {
    "type": "choice", "choice": "involved", "confidence": 0.8,
    "probabilities": {"routine": 0.1, "involved": 0.9},
}}}


class SelectModelTests(unittest.TestCase):
    def call(self, answer=ANSWER, status=200, error=None):
        response = Mock()
        response.status = status
        response.read.return_value = answer if isinstance(answer, bytes) else json.dumps(answer).encode()
        response.__enter__ = Mock(return_value=response)
        response.__exit__ = Mock(return_value=False)
        opener = Mock()
        opener.open.side_effect = error
        opener.open.return_value = response
        with patch.object(ROUTER.urllib.request, "build_opener", return_value=opener):
            result = ROUTER.select_configuration(PAYLOAD, "test-placeholder")
        return result, opener, response

    def test_success_uses_exact_atomic_candidate_and_evaluate_wire(self):
        result, opener, response = self.call()
        self.assertEqual(result, {"status": "selected", "id": "involved", "model": "gpt-6.1-sol",
                                  "effort": "medium", "confidence": 0.8})
        opener.open.assert_called_once()
        request = opener.open.call_args.args[0]
        self.assertEqual(request.full_url, "https://ai-gateway.vercel.sh/v1/evaluate")
        self.assertEqual(request.method, "POST")
        self.assertEqual(request.get_header("Authorization"), "Bearer test-placeholder")
        body = json.loads(request.data)
        self.assertEqual(body["model"], "typesafe-ai/jev")
        question = body["questions"]["configuration"]
        self.assertEqual(question["type"], "choice")
        self.assertEqual(set(question["criteria"]), {"routine", "involved"})
        self.assertIn("gpt-6.1-sol; effort: medium", question["criteria"]["involved"])
        response.read.assert_called_once_with(ROUTER.MAX_BYTES + 1)
        self.assertEqual(opener.open.call_args.kwargs["timeout"], ROUTER.TIMEOUT_SECONDS)

    def test_missing_auth_and_fixed_configuration_skip_network(self):
        with patch.object(ROUTER, "evaluate") as evaluate:
            self.assertEqual(ROUTER.select_configuration(PAYLOAD, None),
                             {"status": "fallback", "reason": "missing_auth"})
            fixed = copy.deepcopy(PAYLOAD)
            fixed["candidates"] = fixed["candidates"][:1]
            self.assertEqual(ROUTER.select_configuration(fixed, "test-placeholder")["reason"], "fixed_configuration")
            evaluate.assert_not_called()

    def test_invalid_inputs_do_not_send_task(self):
        cases = [{}, {**PAYLOAD, "task": ""}, {**PAYLOAD, "context": []},
                 {**PAYLOAD, "candidates": []}, {**PAYLOAD, "candidates": [None]}]
        duplicate = copy.deepcopy(PAYLOAD)
        duplicate["candidates"][1]["id"] = "routine"
        cases.append(duplicate)
        for payload in cases:
            with self.subTest(payload=payload), patch.object(ROUTER, "evaluate") as evaluate:
                self.assertEqual(ROUTER.select_configuration(payload, "test-placeholder")["reason"], "invalid_input")
                evaluate.assert_not_called()

    def test_malformed_or_oversized_wire_falls_back(self):
        cases = [b"{", b"\xff", b"[]", b'{"answers":{},"answers":{}}',
                 b"x" * (ROUTER.MAX_BYTES + 1), {}, {"answers": []},
                 {"answers": {"configuration": {"type": "text", "text": "involved"}}}]
        for answer in cases:
            with self.subTest(answer=str(answer)[:40]):
                self.assertEqual(self.call(answer)[0]["reason"], "invalid_response")

    def test_distributions_are_complete_finite_and_unambiguous(self):
        cases = [
            ({"routine": 0.2}, "involved", "invalid_response"),
            ({"routine": 0.1, "involved": 0.8, "other": 0.1}, "involved", "invalid_response"),
            ({"routine": -0.1, "involved": 1.1}, "involved", "invalid_response"),
            ({"routine": True, "involved": 0}, "routine", "invalid_response"),
            ({"routine": 0.1, "involved": 0.8}, "involved", "invalid_response"),
            ({"routine": float("nan"), "involved": 0.8}, "involved", "invalid_response"),
            ({"routine": float("inf"), "involved": 0.8}, "involved", "invalid_response"),
            ({"routine": 0.2, "involved": 0.8}, "other", "invalid_response"),
            ({"routine": 0.5, "involved": 0.5}, "involved", "ambiguous_choice"),
            ({"routine": 0.8, "involved": 0.2}, "involved", "ambiguous_choice"),
        ]
        for probabilities, choice, expected in cases:
            with self.subTest(probabilities=probabilities, choice=choice):
                answer = copy.deepcopy(ANSWER)
                answer["answers"]["configuration"].update(probabilities=probabilities, choice=choice)
                if expected == "ambiguous_choice":
                    answer["answers"]["configuration"]["confidence"] = 2 * max(probabilities.values()) - 1
                self.assertEqual(self.call(answer)[0]["reason"], expected)

    def test_confidence_floor_and_numeric_types(self):
        for confidence in (0.59, -1, 1.1, True, "0.9", None, float("nan")):
            with self.subTest(confidence=confidence):
                answer = copy.deepcopy(ANSWER)
                answer["answers"]["configuration"]["confidence"] = confidence
                if confidence == 0.59:
                    answer["answers"]["configuration"]["probabilities"] = {"routine": 0.205, "involved": 0.795}
                expected = "low_confidence" if confidence == 0.59 else "invalid_response"
                self.assertEqual(self.call(answer)[0]["reason"], expected)
        answer = copy.deepcopy(ANSWER)
        answer["answers"]["configuration"]["confidence"] = 0.6
        answer["answers"]["configuration"]["probabilities"] = {"routine": 0.2, "involved": 0.8}
        self.assertEqual(self.call(answer)[0]["status"], "selected")

    def test_confidence_must_match_choice_distribution(self):
        answer = copy.deepcopy(ANSWER)
        answer["answers"]["configuration"].update(
            confidence=1, probabilities={"routine": 0.49, "involved": 0.51})
        self.assertEqual(self.call(answer)[0]["reason"], "invalid_response")

    def test_api_failure_timeout_and_redirects_use_safe_codes(self):
        for error, expected in (
            (urllib.error.URLError("private diagnostics"), "api_unavailable"),
            (urllib.error.HTTPError(ROUTER.ENDPOINT, 401, "private body", {}, io.BytesIO(b"secret")), "api_unavailable"),
            (TimeoutError("private diagnostics"), "timeout"),
            (ROUTER.DeadlineExpired(), "timeout"),
            (urllib.error.URLError(TimeoutError()), "timeout"),
            (http.client.IncompleteRead(b"private partial body"), "api_unavailable"),
            (http.client.BadStatusLine("private status"), "api_unavailable"),
        ):
            with self.subTest(error=type(error).__name__):
                self.assertEqual(self.call(error=error)[0], {"status": "fallback", "reason": expected})
        self.assertEqual(self.call(status=503)[0]["reason"], "api_unavailable")
        self.assertIsNone(ROUTER.NoRedirect().redirect_request(None, None, 302, "", {}, "https://example.com"))

    def test_deadline_bounds_entire_request_and_restores_handler(self):
        previous_handler = signal.getsignal(signal.SIGALRM)
        with patch.object(ROUTER.signal, "setitimer") as timer:
            with ROUTER.deadline():
                handler = signal.getsignal(signal.SIGALRM)
                with self.assertRaises(ROUTER.DeadlineExpired):
                    handler(signal.SIGALRM, None)
            self.assertEqual(signal.getsignal(signal.SIGALRM), previous_handler)
        self.assertEqual(timer.call_args_list[0].args, (signal.ITIMER_REAL, ROUTER.TIMEOUT_SECONDS))
        self.assertEqual(timer.call_args_list[-1].args, (signal.ITIMER_REAL, 0))

    def test_unavailable_deadline_uses_fallback(self):
        with patch.object(ROUTER, "deadline", side_effect=NotImplementedError):
            self.assertEqual(self.call()[0]["reason"], "request_unavailable")

    def test_cli_emits_only_safe_json_and_no_secret_or_input(self):
        environment = dict(os.environ)
        environment.pop("AI_GATEWAY_API_KEY", None)
        for raw, reason in ((json.dumps(PAYLOAD), "missing_auth"), ("{private-input", "invalid_input"),
                            ("x" * (ROUTER.MAX_BYTES + 1), "invalid_input")):
            result = subprocess.run([sys.executable, str(SCRIPT)], input=raw, text=True, capture_output=True,
                                    env=environment, timeout=2)
            self.assertEqual(result.returncode, 0)
            self.assertEqual(json.loads(result.stdout), {"status": "fallback", "reason": reason})
            self.assertEqual(result.stderr, "")


if __name__ == "__main__":
    unittest.main()

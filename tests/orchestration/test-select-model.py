from __future__ import annotations

import copy
import http.client
import importlib.util
import io
import json
import os
import subprocess
import sys
import threading
import unittest
import urllib.error
from pathlib import Path
from unittest.mock import Mock, patch

SCRIPT = Path(__file__).resolve().parents[2] / "skills/orchestration/scripts/select-model.py"
SPEC = importlib.util.spec_from_file_location("select_model", SCRIPT)
ROUTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ROUTER)
BRIEF = {"id": "A1", "task": "Implement a bounded parser correction and its regression test.", "context": "Two files."}
PAYLOAD = {"route": "native-codex", "kind": "implementation", "briefs": [BRIEF]}


def demand_answer(levels, confidence=None):
    """Build a Score answer over the demand levels whose confidence follows Jev's formula unless overridden."""
    if confidence is None:
        confidence = ROUTER.score_confidence(levels, levels.index(max(levels)))[0]
    return {
        "answers": {
            "demand": {
                "type": "score",
                "confidence": confidence,
                "probabilities": {str(index): value for index, value in enumerate(levels)},
            }
        }
    }


ANSWER = demand_answer([0, 0.1, 0.85, 0.05, 0])


def payload_with(**changes):
    payload = copy.deepcopy(PAYLOAD)
    payload.update(changes)
    return payload


class SelectModelTests(unittest.TestCase):
    def call(self, answer=ANSWER, status=200, error=None, payload=PAYLOAD, key="test-placeholder"):
        response = Mock()
        response.status = status
        response.read.return_value = answer if isinstance(answer, bytes) else json.dumps(answer).encode()
        response.__enter__ = Mock(return_value=response)
        response.__exit__ = Mock(return_value=False)
        opener = Mock()
        opener.open.side_effect = error
        opener.open.return_value = response
        with patch.object(ROUTER.urllib.request, "build_opener", return_value=opener):
            results = ROUTER.route_briefs(payload, key)
        return results, opener, response

    def first(self, **kwargs):
        return self.call(**kwargs)[0][0]

    def test_success_rates_demand_rubric_on_evaluate_wire(self):
        results, opener, response = self.call()
        self.assertEqual(
            results,
            [
                {
                    "id": "A1",
                    "status": "selected",
                    "candidate": "sol_high",
                    "model": "gpt-6.1-sol",
                    "effort": "high",
                    "rule": "confidence",
                    "confidence": 0.88,
                    "record": "selection: jev; rule: confidence; confidence: 0.88",
                }
            ],
        )
        request = opener.open.call_args.args[0]
        self.assertEqual(request.full_url, "https://ai-gateway.vercel.sh/v1/evaluate")
        self.assertEqual(request.method, "POST")
        self.assertEqual(request.get_header("Authorization"), "Bearer test-placeholder")
        body = json.loads(request.data)
        self.assertEqual(body["model"], "typesafe-ai/jev")
        self.assertEqual(body["state"], BRIEF["task"] + "\n\nContext:\nImplementation. Two files.")
        question = body["questions"]["demand"]
        self.assertEqual((question["type"], question["criteria"]), ("score", list(ROUTER.DEMAND_LEVELS)))
        self.assertNotRegex(json.dumps(question), r"(?i)gpt|luna|sol\b|astra|sonnet|opus|effort:")
        response.read.assert_called_once_with(ROUTER.MAX_BYTES + 1)
        self.assertEqual(opener.open.call_args.kwargs["timeout"], ROUTER.TIMEOUT_SECONDS)

    def test_route_presets_cover_each_adapter_and_kind(self):
        expected = {
            ("native-codex", "research"): ["luna_high", "sol_medium", "sol_high"],
            ("native-codex", "implementation"): ["luna_high", "sol_medium", "sol_high", "sol_xhigh", "astra_xhigh"],
            ("codex-cli", "research"): ["luna_high", "sol_medium"],
            ("native-claude", "research"): ["sonnet_medium", "sonnet_high", "opus_medium", "opus_high"],
            ("claude-cli", "implementation"): [
                "sonnet_medium",
                "sonnet_high",
                "opus_medium",
                "opus_high",
                "opus_xhigh",
            ],
        }
        for (route, kind), ids in expected.items():
            with self.subTest(route=route, kind=kind):
                ladder = ROUTER.build_ladder({"route": route}, kind)
                self.assertEqual([candidate[0] for candidate in ladder], ids)

    def test_batch_keeps_input_order_and_routes_each_kind(self):
        payload = payload_with(
            briefs=[BRIEF, {"id": "R1", "kind": "research", "task": "Survey linter configuration across packages."}]
        )
        # All mass on the hardest level folds onto each ladder's strongest allowed candidate.
        results, opener, _ = self.call(answer=demand_answer([0, 0, 0, 0, 1]), payload=payload)
        self.assertEqual(
            [(result["id"], result["candidate"]) for result in results], [("A1", "astra_xhigh"), ("R1", "sol_high")]
        )
        self.assertEqual(opener.open.call_count, 2)
        states = sorted(json.loads(call.args[0].data)["state"] for call in opener.open.call_args_list)
        self.assertTrue(states[1].endswith("Context:\nResearch."))

    def test_filters_fold_distribution_onto_narrowed_ladder(self):
        payload = payload_with(route="native-claude", models=["opus"])
        result = self.first(answer=demand_answer([0.45, 0.4, 0.15, 0, 0]), payload=payload)
        self.assertEqual(
            (result["candidate"], result["rule"], result["confidence"]), ("opus_medium", "confidence", 1.0)
        )
        payload = payload_with(route="native-claude", efforts=["high"])
        # Level 2 is equidistant from sonnet_high (1) and opus_high (3), so the stronger candidate receives it.
        self.assertEqual(ROUTER.nearest_candidate(2, ROUTER.build_ladder(payload, "implementation")), 1)
        result = self.first(answer=demand_answer([0.1, 0.35, 0.4, 0.1, 0.05]), payload=payload)
        self.assertEqual((result["candidate"], result["effort"]), ("opus_high", "high"))

    def test_inherited_effort_merges_models(self):
        payload = payload_with(route="native-claude", inherit_effort=True)
        ladder = ROUTER.build_ladder(payload, "implementation")
        self.assertEqual(ladder, [("sonnet", "sonnet", "inherited", (0, 1)), ("opus", "opus", "inherited", (2, 3, 4))])
        result = self.first(answer=demand_answer([0.05, 0.25, 0.6, 0.1, 0]), payload=payload)
        self.assertEqual((result["candidate"], result["effort"]), ("opus", "inherited"))

    def test_custom_candidates_replace_presets(self):
        candidates = [
            {"id": "fable_medium", "model": "fable", "effort": "medium", "level": 1},
            {"id": "fable_high", "model": "fable", "effort": "high", "level": 2},
            {"id": "fable_xhigh", "model": "fable", "effort": "xhigh", "level": 3},
        ]
        payload = {"candidates": candidates, "kind": "implementation", "briefs": [BRIEF]}
        result = self.first(answer=demand_answer([0, 0, 0.1, 0.85, 0.05]), payload=payload)
        self.assertEqual((result["candidate"], result["model"]), ("fable_xhigh", "fable"))

    def test_fixed_missing_auth_and_invalid_input_skip_network(self):
        with patch.object(ROUTER, "request_demand") as request:
            fixed = ROUTER.route_briefs(payload_with(route="codex-cli", kind="research", efforts=["high"]), "key")
            self.assertEqual(
                fixed,
                [
                    {
                        "id": "A1",
                        "status": "fixed",
                        "candidate": "luna_high",
                        "model": "gpt-6-luna",
                        "effort": "high",
                        "record": "selection: fixed",
                    }
                ],
            )
            self.assertEqual(ROUTER.route_briefs(PAYLOAD, None)[0]["reason"], "missing_auth")
            cases = {
                "payload": [],
                "briefs": payload_with(briefs=[{**BRIEF, "id": str(index)} for index in range(ROUTER.MAX_BRIEFS + 1)]),
                "brief": payload_with(briefs=[BRIEF, BRIEF]),
                "kind": payload_with(kind="design"),
                "route": payload_with(route=["native-codex"]),
                "filters": payload_with(models="opus"),
                "no_candidates": payload_with(models=["opus"]),
                "candidates": {
                    "candidates": [{"id": "x", "model": "m", "effort": "e", "level": 9}],
                    "kind": "research",
                    "briefs": [BRIEF],
                },
            }
            for detail, payload in cases.items():
                with self.subTest(detail=detail):
                    self.assertEqual(
                        ROUTER.route_briefs(payload, "key"),
                        [
                            {
                                "status": "fallback",
                                "reason": "invalid_input",
                                "detail": detail,
                                "record": f"selection: fallback; reason: invalid_input; detail: {detail}",
                            }
                        ],
                    )
            request.assert_not_called()

    def test_malformed_or_oversized_wire_falls_back(self):
        cases = [
            b"{",
            b"\xff",
            b"[]",
            b'{"answers":{},"answers":{}}',
            b"x" * (ROUTER.MAX_BYTES + 1),
            {},
            {"answers": []},
            {"answers": {"demand": {"type": "text", "text": "involved"}}},
            {"answers": {"demand": {"type": "choice", "choice": "a", "confidence": 0.8, "probabilities": {"a": 1}}}},
        ]
        for answer in cases:
            with self.subTest(answer=str(answer)[:40]):
                self.assertEqual(self.first(answer=answer)["reason"], "invalid_response")

    def test_distributions_are_complete_and_finite(self):
        cases = [
            {"0": 0.2},
            {"0": 0.1, "1": 0.8, "2": 0.1, "3": 0, "4": 0, "5": 0},
            {"a": 0.1, "1": 0.9, "2": 0, "3": 0, "4": 0},
            {"0": -0.1, "1": 1.1, "2": 0, "3": 0, "4": 0},
            {"0": True, "1": 0, "2": 0, "3": 0, "4": 0},
            {"0": 0.1, "1": 0.8, "2": 0, "3": 0, "4": 0},
            {"0": float("nan"), "1": 0.8, "2": 0, "3": 0, "4": 0},
        ]
        for probabilities in cases:
            with self.subTest(probabilities=probabilities):
                answer = copy.deepcopy(ANSWER)
                answer["answers"]["demand"].update(probabilities=probabilities, confidence=0)
                self.assertEqual(self.first(answer=answer)["reason"], "invalid_response")

    def test_confidence_numeric_types_and_formula(self):
        for confidence in (-1, 1.1, True, "0.9", None, float("nan"), 0.5):
            with self.subTest(confidence=confidence):
                answer = copy.deepcopy(ANSWER)
                answer["answers"]["demand"]["confidence"] = confidence
                result = self.first(answer=answer)
                self.assertEqual(result["reason"], "invalid_response")
        self.assertEqual(
            self.first(answer=demand_answer([0, 0.1, 0.85, 0.05, 0], 0.7))["detail"], "confidence_consistency"
        )

    def test_live_score_distributions_match_formula_within_rounding(self):
        # Distributions and confidences observed from Jev's demand rubric, rounded independently to hundredths.
        cases = [
            ([0, 0.02, 0.13, 0.84, 0.01], 0.84, "sol_xhigh", "confidence"),
            ([0.96, 0.04, 0, 0, 0], 0.97, "luna_high", "confidence"),
            ([0, 0.42, 0.5, 0.08, 0], 0.58, "sol_high", "adjacent_tiers"),
        ]
        for levels, confidence, expected, rule in cases:
            with self.subTest(levels=levels):
                result = self.first(answer=demand_answer(levels, confidence))
                self.assertEqual((result["status"], result["candidate"], result["rule"]), ("selected", expected, rule))

    def test_tied_maximum_validates_and_abstains(self):
        result = self.first(answer=demand_answer([0, 0.5, 0.5, 0, 0]))
        self.assertEqual(result["reason"], "ambiguous_choice")

    def test_adjacent_tier_split_selects_higher_probability_tier(self):
        for levels, expected in (
            ([0.36, 0.47, 0.14, 0.03, 0], "sol_medium"),
            ([0, 0.02, 0.12, 0.5, 0.36], "sol_xhigh"),
        ):
            with self.subTest(levels=levels):
                result = self.first(answer=demand_answer(levels))
                self.assertEqual((result["candidate"], result["rule"]), (expected, "adjacent_tiers"))
                self.assertAlmostEqual(result["top_two_mass"], sum(sorted(levels)[-2:]), places=2)
                self.assertLess(result["confidence"], ROUTER.CONFIDENCE_FLOOR)

    def test_distant_or_thin_splits_remain_abstentions(self):
        for levels in ([0.36, 0.14, 0.35, 0.15, 0], [0.5, 0.29, 0.21, 0, 0], [0.07, 0.5, 0.16, 0.27, 0]):
            with self.subTest(levels=levels):
                result = self.first(answer=demand_answer(levels))
                self.assertEqual((result["status"], result["reason"]), ("fallback", "low_confidence"))
                self.assertTrue(
                    result["record"].startswith("selection: fallback; reason: low_confidence; confidence: ")
                )
                self.assertNotIn("candidate", result)

    def test_validation_diagnostics_never_echo_response_or_credential(self):
        private = "malicious-response-and-test-placeholder"
        for answer, detail in (
            ({"answers": {"demand": {"type": private}}}, "score_type"),
            ({"answers": {"demand": {"type": "score", "probabilities": {private: 1}}}}, "distribution_keys"),
            (private.encode(), None),
        ):
            with self.subTest(detail=detail):
                result = self.first(answer=answer)
                self.assertEqual(result["reason"], "invalid_response")
                self.assertEqual(result.get("detail"), detail)
                self.assertNotIn(private, json.dumps(result))
        self.assertEqual(
            ROUTER.classify(ROUTER.InvalidData(private)), {"status": "fallback", "reason": "invalid_response"}
        )

    def test_api_failure_timeout_and_redirects_use_safe_codes(self):
        for error, expected in (
            (urllib.error.URLError("private diagnostics"), "api_unavailable"),
            (
                urllib.error.HTTPError(ROUTER.ENDPOINT, 401, "private body", {}, io.BytesIO(b"secret")),
                "api_unavailable",
            ),
            (TimeoutError("private diagnostics"), "timeout"),
            (urllib.error.URLError(TimeoutError()), "timeout"),
            (http.client.IncompleteRead(b"private partial body"), "api_unavailable"),
            (http.client.BadStatusLine("private status"), "api_unavailable"),
        ):
            with self.subTest(error=type(error).__name__):
                result = self.first(error=error)
                self.assertEqual((result["status"], result["reason"]), ("fallback", expected))
                self.assertNotIn("private", json.dumps(result))
                if isinstance(error, urllib.error.HTTPError):
                    self.assertTrue(error.fp.closed)
        self.assertEqual(self.first(status=503)["reason"], "api_unavailable")
        self.assertIsNone(ROUTER.NoRedirect().redirect_request(None, None, 302, "", {}, "https://example.com"))

    def test_shared_deadline_bounds_blocked_requests(self):
        release = threading.Event()

        def blocked(brief, key):
            if brief["id"] == "slow":
                release.wait(5)
            return ANSWER

        payload = payload_with(briefs=[{**BRIEF, "id": "slow"}, BRIEF])
        with patch.object(ROUTER, "TIMEOUT_SECONDS", 0.2), patch.object(ROUTER, "request_demand", side_effect=blocked):
            results = ROUTER.route_briefs(payload, "key")
        release.set()
        self.assertEqual(
            [(result["id"], result["status"]) for result in results], [("slow", "fallback"), ("A1", "selected")]
        )
        self.assertEqual(results[0]["reason"], "timeout")

    def test_cli_emits_only_safe_json_lines_and_no_secret_or_input(self):
        environment = dict(os.environ)
        environment.pop("AI_GATEWAY_API_KEY", None)
        two_briefs = payload_with(briefs=[BRIEF, {**BRIEF, "id": "A2"}])
        for raw, expected in (
            (json.dumps(two_briefs), [("A1", "missing_auth"), ("A2", "missing_auth")]),
            ("{private-input", [(None, "invalid_input")]),
            ("x" * (ROUTER.MAX_BYTES + 1), [(None, "invalid_input")]),
        ):
            result = subprocess.run(
                [sys.executable, str(SCRIPT)],
                input=raw,
                check=False,
                text=True,
                capture_output=True,
                env=environment,
                timeout=5,
            )
            self.assertEqual(result.returncode, 0)
            lines = [json.loads(line) for line in result.stdout.splitlines()]
            self.assertEqual([(line.get("id"), line["reason"]) for line in lines], expected)
            self.assertNotIn("private", result.stdout)
            self.assertEqual(result.stderr, "")


if __name__ == "__main__":
    unittest.main()

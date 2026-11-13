from __future__ import annotations

import datetime as dt
import importlib.util
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "skills" / "evm-atlas" / "scripts" / "sweep-core.py"
SPEC = importlib.util.spec_from_file_location("sweep_core", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules["sweep_core"] = MODULE
SPEC.loader.exec_module(MODULE)

ADDRESS = "0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe"
BLOCK_HASH = "0x" + "a" * 64
TX_A = "0x" + "1" * 64
TX_B = "0x" + "2" * 64


def spec(profile: str = "general", model: str = "ethereum-eoa", quorum: int = 1, providers: int = 1) -> dict:
    return {
        "address": ADDRESS,
        "chain": {"id": 1, "name": "Ethereum", "accountActivityModel": model},
        "goal": "historical-activity",
        "profile": profile,
        "checkpoint": {
            "requestedAt": "2026-07-22T12:00:00Z",
            "resolutionKind": "finalized",
            "blockNumber": 100,
            "blockHash": BLOCK_HASH,
            "blockTimestamp": "2026-07-22T11:59:00Z",
            "observedAt": "2026-07-22T12:00:01Z",
        },
        "providers": [
            {
                "id": f"p{index}",
                "kind": "synthetic",
                "independenceGroup": f"index-{index}",
                "capabilities": ["txlist", "txlistinternal", "tokentx", "tokennfttx", "token1155tx"],
            }
            for index in range(1, providers + 1)
        ],
        "quorum": quorum,
    }


def state(nonce: str = "0x0", balance: str = "0x0") -> dict:
    return {
        "nonce": {"ok": True, "result": nonce, "blockHash": BLOCK_HASH},
        "native-balance": {"ok": True, "result": balance, "blockHash": BLOCK_HASH},
    }


def row(channel: str, transaction_hash: str = TX_A, block: int = 20, value: str = "0", **extra) -> dict:
    base = {
        "blockNumber": str(block),
        "hash": transaction_hash,
        "timeStamp": str(block * 10),
        "from": ADDRESS.lower() if channel == "txlist" else "0x" + "b" * 40,
        "to": ADDRESS.lower(),
        "value": value,
        "isError": "0",
        "txreceipt_status": "1",
        "errCode": "",
    }
    return base | extra


def responses(plan: dict, rows_by_provider: dict[str, dict[str, list[dict]]] | None = None) -> dict:
    rows_by_provider = rows_by_provider or {}
    provider_payload = {}
    for provider in plan["providers"]:
        channels = {}
        for channel in [item for item in plan["requiredChannels"] if item in MODULE.HISTORY_CHANNELS]:
            channels[channel] = {
                "ok": True,
                "complete": True,
                "rows": rows_by_provider.get(provider["id"], {}).get(channel, []),
            }
        provider_payload[provider["id"]] = {"indexedThrough": 100, "channels": channels}
    return {"state": state(), "providers": provider_payload}


class SweepCoreTests(unittest.TestCase):
    def test_etherscan_retirement_uses_execution_time_and_canonical_chain_id(self) -> None:
        request = spec()
        request["chain"] = {"id": 2741, "name": "Abstract", "accountActivityModel": "native-account-abstraction"}
        request["providers"][0]["kind"] = "etherscan"
        before = dt.datetime(2026, 11, 29, 23, 59, 59, tzinfo=dt.UTC)
        plan = MODULE.build_plan(request, now=before)
        self.assertEqual(len(plan["requests"]["history"]), 5)
        for execution_time in (
            dt.datetime(2026, 11, 30, tzinfo=dt.UTC),
            dt.datetime(2026, 12, 1, tzinfo=dt.UTC),
            dt.datetime(2026, 11, 30, 2, tzinfo=dt.timezone(dt.timedelta(hours=2))),
        ):
            with self.subTest(now=execution_time), self.assertRaisesRegex(MODULE.SweepError, "unsupported.*2026-11-30"):
                MODULE.build_plan(request, now=execution_time)
        # A historical requestedAt/observedAt and an omitted caller cutoff cannot restore current API access.
        execution_time = dt.datetime(2026, 12, 1, tzinfo=dt.UTC)
        with patch.object(MODULE.dt, "datetime", wraps=dt.datetime) as clock:
            clock.now.return_value = execution_time
            with self.assertRaisesRegex(MODULE.SweepError, "unsupported.*2026-11-30"):
                MODULE.build_plan(request)
            clock.now.assert_called_once_with(dt.UTC)
        # Evaluation of saved evidence does not perform a current-provider access check.
        with patch.object(MODULE.dt, "datetime", wraps=dt.datetime) as clock:
            clock.now.return_value = execution_time
            self.assertEqual(MODULE.evaluate(plan, responses(plan))["result"], "negative")
            clock.now.assert_not_called()

    def test_retirement_does_not_disable_other_chains_or_native_indexers(self) -> None:
        execution_time = dt.datetime(2026, 12, 1, tzinfo=dt.UTC)
        request = spec()
        request["providers"][0]["kind"] = "etherscan"
        self.assertTrue(MODULE.build_plan(request, now=execution_time)["requests"]["history"])
        request["chain"] = {"id": 2741, "name": "Abstract", "accountActivityModel": "native-account-abstraction"}
        request["providers"][0]["kind"] = "native-explorer"
        request["providers"][0]["capabilities"] = ["txlist", "tokentx", "tokennfttx"]
        plan = MODULE.build_plan(request, now=execution_time)
        payload = responses(plan)
        for channel in ("txlistinternal", "token1155tx"):
            del payload["providers"]["p1"]["channels"][channel]
        self.assertEqual(MODULE.evaluate(plan, payload)["result"], "unknown")
        payload["providers"] = {}
        self.assertEqual(MODULE.evaluate(plan, payload)["result"], "unknown")

    def test_plan_is_credential_free_and_profile_specific(self) -> None:
        plan = MODULE.build_plan(spec())
        self.assertEqual(plan["requiredChannels"][-1], "token1155tx")
        self.assertTrue(all(request["credentials"] == [] for request in plan["requests"]["history"]))
        self.assertTrue(all(request["bounds"]["endBlock"] == 100 for request in plan["requests"]["history"]))
        bootstrap = MODULE.build_plan(spec("bootstrap-discovery"))
        self.assertNotIn("token1155tx", bootstrap["requiredChannels"])

    def test_general_positive_earliest_and_state_positive(self) -> None:
        plan = MODULE.build_plan(spec())
        payload = responses(plan, {"p1": {"tokentx": [row("tokentx", TX_B, 30), row("tokentx", TX_A, 20)]}})
        result = MODULE.evaluate(plan, payload)
        self.assertEqual(result["result"], "positive")
        self.assertEqual(result["earliestQualifyingEvidence"]["transactionHash"], TX_A)
        payload["state"] = state(nonce="0x1")
        state_result = MODULE.evaluate(plan, payload)
        self.assertEqual(state_result["earliestQualifyingEvidence"]["channel"], "nonce")

    def test_bootstrap_zero_state_shortcut_and_eligibility(self) -> None:
        plan = MODULE.build_plan(spec("bootstrap-discovery"))
        payload = responses(plan)
        del payload["providers"]["p1"]["channels"]["txlist"]
        del payload["providers"]["p1"]["channels"]["txlistinternal"]
        result = MODULE.evaluate(plan, payload)
        self.assertEqual(result["result"], "negative")
        self.assertEqual({item["channel"] for item in result["omitted"]}, {"txlist", "txlistinternal"})

        ineligible_plan = MODULE.build_plan(spec("bootstrap-discovery", model="unknown"))
        ineligible = responses(ineligible_plan)
        del ineligible["providers"]["p1"]["channels"]["txlist"]
        self.assertEqual(MODULE.evaluate(ineligible_plan, ineligible)["result"], "unknown")
        native_plan = MODULE.build_plan(spec("bootstrap-discovery", model="native-account-abstraction"))
        native_payload = responses(native_plan)
        del native_payload["providers"]["p1"]["channels"]["txlistinternal"]
        self.assertEqual(MODULE.evaluate(native_plan, native_payload)["result"], "unknown")

    def test_bootstrap_filters_failed_and_noise_rows(self) -> None:
        plan = MODULE.build_plan(spec("bootstrap-discovery", model="unknown"))
        payload = responses(
            plan,
            {
                "p1": {
                    "txlist": [
                        row("txlist", value="0", **{"from": "0x" + "b" * 40}),
                        row("txlist", value="5", isError="1"),
                    ],
                    "txlistinternal": [row("txlistinternal", value="0")],
                }
            },
        )
        self.assertEqual(MODULE.evaluate(plan, payload)["result"], "negative")

    def test_missing_malformed_and_post_cutoff_channels_are_unknown(self) -> None:
        plan = MODULE.build_plan(spec())
        missing = responses(plan)
        del missing["providers"]["p1"]["channels"]["tokentx"]
        self.assertEqual(MODULE.evaluate(plan, missing)["result"], "unknown")
        malformed = responses(plan)
        malformed["providers"]["p1"]["channels"]["tokentx"]["rows"] = "bad"
        self.assertEqual(MODULE.evaluate(plan, malformed)["result"], "unknown")
        post_cutoff = responses(plan, {"p1": {"tokentx": [row("tokentx", block=101)]}})
        post_result = MODULE.evaluate(plan, post_cutoff)
        self.assertEqual(post_result["result"], "unknown")
        self.assertTrue(any("post-checkpoint" in gap["reason"] for gap in post_result["gaps"]))
        null_hash = responses(plan)
        null_hash["state"]["nonce"]["blockHash"] = None
        null_result = MODULE.evaluate(plan, null_hash)
        self.assertTrue(any("not bound to the checkpoint" in gap["reason"] for gap in null_result["gaps"]))

    def test_quorum_agreement_and_disagreement(self) -> None:
        plan = MODULE.build_plan(spec(quorum=2, providers=2))
        agree = responses(plan, {"p1": {"tokentx": [row("tokentx", TX_A)]}, "p2": {"tokentx": [row("tokentx", TX_A)]}})
        agreed = MODULE.evaluate(plan, agree)
        self.assertEqual(agreed["result"], "positive")
        self.assertTrue(agreed["quorum"]["agreement"])
        disagree = responses(
            plan, {"p1": {"tokentx": [row("tokentx", TX_A)]}, "p2": {"tokentx": [row("tokentx", TX_B)]}}
        )
        disagreed = MODULE.evaluate(plan, disagree)
        self.assertEqual(disagreed["result"], "unknown")
        self.assertFalse(disagreed["quorum"]["agreement"])

    def test_conformance_validators(self) -> None:
        fixture_dir = SCRIPT.parent.parent / "fixtures"
        import json

        counters = json.loads((fixture_dir / "blockscout-address-counters.json").read_text())
        topics = json.loads((fixture_dir / "etherscan-transfer-topic-response.json").read_text())
        self_transfer = json.loads((fixture_dir / "etherscan-transfer-topic-self-transfer.json").read_text())
        self.assertTrue(MODULE.validate_blockscout(counters)["valid"])
        self.assertTrue(MODULE.validate_transfer_topics(topics, ADDRESS)["valid"])
        self.assertFalse(MODULE.validate_transfer_topics(self_transfer, ADDRESS)["valid"])


if __name__ == "__main__":
    unittest.main()

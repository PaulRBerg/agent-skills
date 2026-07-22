#!/usr/bin/env python3
"""Plan, evaluate, and validate deterministic EVM address-sweep evidence."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import sys
from pathlib import Path
from typing import Any


ADDRESS_RE = re.compile(r"^0x[0-9a-fA-F]{40}$")
HASH_RE = re.compile(r"^0x[0-9a-fA-F]{64}$")
HEX_RE = re.compile(r"^0x(?:0|[1-9a-fA-F][0-9a-fA-F]*)$")
TRANSFER_TOPIC = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
HISTORY_CHANNELS = ("txlist", "txlistinternal", "tokentx", "tokennfttx", "token1155tx")
PROFILE_CHANNELS = {
    "general": ("nonce", "native-balance", *HISTORY_CHANNELS),
    "prb-finance-bootstrap": ("nonce", "native-balance", "txlist", "txlistinternal", "tokentx", "tokennfttx"),
}


class SweepError(ValueError):
    pass


def load_json(path: Path | None) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8") if path else sys.stdin.read())
    except (OSError, json.JSONDecodeError) as exc:
        raise SweepError(f"cannot read JSON: {exc}") from exc


def iso_time(value: Any, field: str) -> dt.datetime:
    if not isinstance(value, str):
        raise SweepError(f"{field} must be an ISO-8601 string")
    try:
        parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise SweepError(f"{field} must be an ISO-8601 string") from exc
    if parsed.tzinfo is None:
        raise SweepError(f"{field} must include a timezone")
    return parsed


def normalize_profile(value: Any) -> str:
    aliases = {"bootstrap": "prb-finance-bootstrap", "prb-finance bootstrap": "prb-finance-bootstrap"}
    normalized = aliases.get(value, value)
    if normalized not in PROFILE_CHANNELS:
        raise SweepError(f"unsupported profile: {value}")
    return normalized


def validate_checkpoint(checkpoint: Any) -> dict[str, Any]:
    if not isinstance(checkpoint, dict):
        raise SweepError("checkpoint must be an object")
    required = ("requestedAt", "resolutionKind", "blockNumber", "blockHash", "blockTimestamp", "observedAt")
    missing = [field for field in required if field not in checkpoint]
    if missing:
        raise SweepError(f"checkpoint is missing: {', '.join(missing)}")
    if checkpoint["resolutionKind"] not in {"finalized", "verified"}:
        raise SweepError("checkpoint resolutionKind must be finalized or verified")
    if not isinstance(checkpoint["blockNumber"], int) or checkpoint["blockNumber"] < 0:
        raise SweepError("checkpoint blockNumber must be a non-negative integer")
    if not isinstance(checkpoint["blockHash"], str) or not HASH_RE.fullmatch(checkpoint["blockHash"]):
        raise SweepError("checkpoint blockHash must contain 32 hex bytes")
    requested = iso_time(checkpoint["requestedAt"], "checkpoint requestedAt")
    block_time = iso_time(checkpoint["blockTimestamp"], "checkpoint blockTimestamp")
    observed = iso_time(checkpoint["observedAt"], "checkpoint observedAt")
    if block_time > requested:
        raise SweepError("checkpoint blockTimestamp is after requestedAt")
    if observed < block_time:
        raise SweepError("checkpoint observedAt is before blockTimestamp")
    return {**checkpoint, "blockHash": checkpoint["blockHash"].lower()}


def build_plan(spec: Any) -> dict[str, Any]:
    if not isinstance(spec, dict):
        raise SweepError("plan input must be an object")
    address = spec.get("address")
    if not isinstance(address, str) or not ADDRESS_RE.fullmatch(address):
        raise SweepError("address must contain exactly 20 hex bytes")
    chain = spec.get("chain")
    if not isinstance(chain, dict) or not isinstance(chain.get("id"), int) or chain["id"] <= 0:
        raise SweepError("chain.id must be a positive integer")
    model = chain.get("accountActivityModel")
    if not isinstance(model, str) or not model:
        raise SweepError("chain.accountActivityModel must be a non-empty string")
    goal = spec.get("goal", "historical-activity")
    if goal not in {"historical-activity", "bootstrap-discovery"}:
        raise SweepError(f"unsupported goal: {goal}")
    profile = normalize_profile(spec.get("profile", "general"))
    checkpoint = validate_checkpoint(spec.get("checkpoint"))
    providers = spec.get("providers")
    if not isinstance(providers, list) or not providers:
        raise SweepError("providers must be a non-empty array selected by the agent")
    normalized_providers: list[dict[str, Any]] = []
    provider_ids: set[str] = set()
    for provider in providers:
        if not isinstance(provider, dict) or not isinstance(provider.get("id"), str) or not provider["id"]:
            raise SweepError("every provider must have a non-empty id")
        if provider["id"] in provider_ids:
            raise SweepError(f"duplicate provider id: {provider['id']}")
        provider_ids.add(provider["id"])
        capabilities = provider.get("capabilities")
        if not isinstance(capabilities, list) or not all(channel in HISTORY_CHANNELS for channel in capabilities):
            raise SweepError(f"provider {provider['id']} has invalid capabilities")
        normalized_providers.append(
            {
                "id": provider["id"],
                "kind": provider.get("kind", "selected-indexer"),
                "independenceGroup": provider.get("independenceGroup", provider["id"]),
                "capabilities": list(dict.fromkeys(capabilities)),
            }
        )
    quorum = spec.get("quorum", 1)
    if isinstance(quorum, dict):
        quorum = quorum.get("required", 1) if quorum.get("enabled", False) else 1
    if not isinstance(quorum, int) or quorum < 1:
        raise SweepError("quorum must be a positive integer")
    if quorum > len({provider["independenceGroup"] for provider in normalized_providers}):
        raise SweepError("quorum exceeds independent provider groups")
    required = list(PROFILE_CHANNELS[profile])
    state_requests = [
        {
            "requestId": f"state:{channel}",
            "channel": channel,
            "transport": "json-rpc",
            "method": method,
            "params": [address.lower(), {"blockHash": checkpoint["blockHash"], "requireCanonical": True}],
            "credentials": [],
        }
        for channel, method in (("nonce", "eth_getTransactionCount"), ("native-balance", "eth_getBalance"))
    ]
    history_requests = []
    for provider in normalized_providers:
        for channel in required:
            if channel not in HISTORY_CHANNELS or channel not in provider["capabilities"]:
                continue
            history_requests.append(
                {
                    "requestId": f"history:{provider['id']}:{channel}",
                    "providerId": provider["id"],
                    "channel": channel,
                    "transport": "indexed-history",
                    "operation": {"providerKind": provider["kind"], "action": channel},
                    "bounds": {"startBlock": 0, "endBlock": checkpoint["blockNumber"]},
                    "order": "asc" if quorum > 1 else "desc",
                    "credentials": [],
                }
            )
    return {
        "schemaVersion": 1,
        "address": address.lower(),
        "chain": chain,
        "goal": goal,
        "profile": profile,
        "checkpoint": checkpoint,
        "quorumRequirement": quorum,
        "requiredChannels": required,
        "providers": normalized_providers,
        "requests": {"state": state_requests, "history": history_requests},
    }


def parse_quantity(value: Any, context: str) -> int:
    if isinstance(value, int) and not isinstance(value, bool) and value >= 0:
        return value
    if not isinstance(value, str):
        raise SweepError(f"{context} must be a non-negative integer or quantity string")
    if HEX_RE.fullmatch(value):
        return int(value, 16)
    if re.fullmatch(r"0|[1-9]\d*", value):
        return int(value)
    raise SweepError(f"{context} must be a non-negative integer or quantity string")


def row_block(row: dict[str, Any]) -> int:
    value = row.get("blockNumber", row.get("block_number"))
    return parse_quantity(value, "row block number")


def touches(row: dict[str, Any], address: str) -> bool:
    values = [row.get(key) for key in ("from", "to", "contractAddress", "created_contract")]
    return any(isinstance(value, str) and value.lower() == address for value in values)


def failed_row(row: dict[str, Any]) -> bool:
    return str(row.get("isError", row.get("is_error", "0"))) == "1" or str(row.get("txreceipt_status", "1")) == "0" or bool(str(row.get("errCode", row.get("error", ""))).strip())


def qualifies(profile: str, channel: str, row: dict[str, Any], address: str) -> bool:
    if failed_row(row) or not touches(row, address):
        return False
    if profile == "general" or channel in {"tokentx", "tokennfttx", "token1155tx"}:
        return True
    value = parse_quantity(row.get("value", 0), f"{channel} row value")
    if channel == "txlist":
        outgoing = isinstance(row.get("from"), str) and row["from"].lower() == address
        return outgoing or value > 0
    if channel == "txlistinternal":
        return value > 0
    return True


def evidence(provider_id: str, channel: str, row: dict[str, Any]) -> dict[str, Any]:
    transaction_hash = row.get("hash", row.get("transactionHash", row.get("transaction_hash")))
    if not isinstance(transaction_hash, str) or not HASH_RE.fullmatch(transaction_hash):
        raise SweepError(f"qualifying {channel} row has no valid transaction hash")
    timestamp = row.get("timeStamp", row.get("timestamp"))
    return {
        "providerId": provider_id,
        "channel": channel,
        "transactionHash": transaction_hash.lower(),
        "blockNumber": row_block(row),
        "timestamp": timestamp,
    }


def evidence_key(item: dict[str, Any]) -> tuple[Any, ...]:
    return (item["blockNumber"], str(item.get("timestamp") or ""), item["transactionHash"], item["channel"])


def evaluate(plan: Any, responses: Any) -> dict[str, Any]:
    if not isinstance(plan, dict) or plan.get("schemaVersion") != 1:
        raise SweepError("plan schemaVersion must be 1")
    if not isinstance(responses, dict):
        raise SweepError("responses must be an object")
    checkpoint = validate_checkpoint(plan.get("checkpoint"))
    address = plan.get("address")
    if not isinstance(address, str) or not ADDRESS_RE.fullmatch(address):
        raise SweepError("plan address is invalid")
    profile = normalize_profile(plan.get("profile"))
    required = plan.get("requiredChannels")
    if required != list(PROFILE_CHANNELS[profile]):
        raise SweepError("plan requiredChannels do not match the profile")
    checked: list[dict[str, Any]] = []
    omitted: list[dict[str, str]] = []
    gaps: list[dict[str, str]] = []
    state = responses.get("state") or {}
    if not isinstance(state, dict):
        raise SweepError("responses.state must be an object")
    state_values: dict[str, int] = {}
    state_positive: dict[str, Any] | None = None
    for channel in ("nonce", "native-balance"):
        response = state.get(channel)
        if not isinstance(response, dict) or response.get("ok") is not True:
            gaps.append({"channel": channel, "reason": "missing or failed checkpoint-bound state response"})
            continue
        if response.get("blockHash", "").lower() != checkpoint["blockHash"] and response.get("checkpointBound") is not True:
            gaps.append({"channel": channel, "reason": "state response is not bound to the checkpoint hash"})
            continue
        try:
            quantity = parse_quantity(response.get("result"), f"state {channel}")
        except SweepError as exc:
            gaps.append({"channel": channel, "reason": str(exc)})
            continue
        state_values[channel] = quantity
        checked.append({"channel": channel, "source": "state-rpc", "result": "positive" if quantity else "negative"})
        if quantity and state_positive is None:
            state_positive = {"source": "state-rpc", "channel": channel, "value": quantity, "blockNumber": checkpoint["blockNumber"], "blockHash": checkpoint["blockHash"]}

    zero_shortcut = (
        profile == "prb-finance-bootstrap"
        and plan.get("chain", {}).get("accountActivityModel") == "ethereum-eoa"
        and state_values.get("nonce") == 0
        and state_values.get("native-balance") == 0
    )
    history_required = [channel for channel in required if channel in HISTORY_CHANNELS]
    if zero_shortcut:
        for channel in ("txlist", "txlistinternal"):
            history_required.remove(channel)
            omitted.append({"channel": channel, "reason": "ethereum-eoa zero-state invariant"})

    provider_responses = responses.get("providers") or {}
    if not isinstance(provider_responses, dict):
        raise SweepError("responses.providers must be an object")
    provider_results: list[dict[str, Any]] = []
    for provider in plan.get("providers", []):
        provider_id = provider["id"]
        response = provider_responses.get(provider_id)
        if not isinstance(response, dict):
            provider_results.append({"providerId": provider_id, "complete": False, "earliest": None, "positive": False})
            continue
        try:
            indexed_through = parse_quantity(response.get("indexedThrough"), f"provider {provider_id} indexedThrough")
        except SweepError as exc:
            gaps.append({"channel": "history", "reason": str(exc)})
            provider_results.append({"providerId": provider_id, "complete": False, "earliest": None, "positive": False})
            continue
        if indexed_through < checkpoint["blockNumber"]:
            gaps.append({"channel": "history", "reason": f"provider {provider_id} is indexed only through {indexed_through}"})
            provider_results.append({"providerId": provider_id, "complete": False, "earliest": None, "positive": False})
            continue
        channels = response.get("channels")
        if not isinstance(channels, dict):
            gaps.append({"channel": "history", "reason": f"provider {provider_id} channels are malformed"})
            provider_results.append({"providerId": provider_id, "complete": False, "earliest": None, "positive": False})
            continue
        provider_complete = True
        provider_evidence: list[dict[str, Any]] = []
        for channel in history_required:
            channel_response = channels.get(channel)
            if not isinstance(channel_response, dict) or channel_response.get("ok") is not True:
                provider_complete = False
                gaps.append({"channel": channel, "reason": f"provider {provider_id} response is missing or failed"})
                continue
            rows = channel_response.get("rows")
            if not isinstance(rows, list) or not all(isinstance(row, dict) for row in rows):
                provider_complete = False
                gaps.append({"channel": channel, "reason": f"provider {provider_id} rows are malformed"})
                continue
            try:
                bounded = all(row_block(row) <= checkpoint["blockNumber"] for row in rows)
            except SweepError as exc:
                provider_complete = False
                gaps.append({"channel": channel, "reason": f"provider {provider_id}: {exc}"})
                continue
            if not bounded:
                provider_complete = False
                gaps.append({"channel": channel, "reason": f"provider {provider_id} returned a post-checkpoint row"})
                continue
            try:
                qualifying = [row for row in rows if qualifies(profile, channel, row, address.lower())]
                channel_evidence = [evidence(provider_id, channel, row) for row in qualifying]
            except SweepError as exc:
                provider_complete = False
                gaps.append({"channel": channel, "reason": f"provider {provider_id}: {exc}"})
                continue
            provider_evidence.extend(channel_evidence)
            complete = channel_response.get("complete") is True
            if not channel_evidence and not complete:
                provider_complete = False
                gaps.append({"channel": channel, "reason": f"provider {provider_id} did not exhaust the bounded channel"})
            elif channel_evidence and not complete:
                provider_complete = False
                gaps.append({"channel": channel, "reason": f"provider {provider_id} positive response is not complete enough for earliest evidence"})
            checked.append(
                {
                    "channel": channel,
                    "source": provider_id,
                    "result": "positive" if channel_evidence else "negative" if complete else "unknown",
                }
            )
        earliest = min(provider_evidence, key=evidence_key) if provider_evidence else None
        provider_results.append(
            {
                "providerId": provider_id,
                "independenceGroup": provider["independenceGroup"],
                "complete": provider_complete,
                "positive": bool(provider_evidence),
                "earliest": earliest,
            }
        )

    quorum = plan.get("quorumRequirement", 1)
    agreement: bool | None = None
    earliest: dict[str, Any] | None = state_positive
    result = "unknown"
    if state_positive:
        result = "positive"
    elif quorum == 1:
        usable = next((provider for provider in provider_results if provider["positive"] or provider["complete"]), None)
        if usable and usable["positive"]:
            result, earliest = "positive", usable["earliest"]
        elif usable and usable["complete"] and not gaps:
            result = "negative"
        agreement = None
    else:
        votes = [provider for provider in provider_results if provider.get("complete")]
        distinct: list[dict[str, Any]] = []
        seen_groups: set[str] = set()
        for vote in votes:
            if vote["independenceGroup"] not in seen_groups:
                distinct.append(vote)
                seen_groups.add(vote["independenceGroup"])
        if len(distinct) >= quorum:
            selected = distinct[:quorum]
            if all(vote["positive"] for vote in selected):
                keys = {evidence_key(vote["earliest"]) for vote in selected}
                agreement = len(keys) == 1
                if agreement:
                    result, earliest = "positive", selected[0]["earliest"]
            elif all(vote["complete"] and not vote["positive"] for vote in selected):
                agreement, result = True, "negative"
            else:
                agreement = False
        if agreement is False:
            gaps.append({"channel": "quorum", "reason": "independent history providers disagree"})

    coverage = "complete" if result == "negative" and not gaps else "partial" if checked or omitted else "unknown"
    if result == "positive" and earliest:
        coverage = "complete" if not gaps else "partial"
    return {
        "schemaVersion": 1,
        "result": result,
        "coverage": coverage,
        "profile": profile,
        "checkpoint": checkpoint,
        "checked": checked,
        "omitted": omitted,
        "gaps": gaps,
        "earliestQualifyingEvidence": earliest,
        "quorum": {"required": quorum, "agreement": agreement, "providers": provider_results},
    }


def validate_blockscout(payload: Any) -> dict[str, Any]:
    fields = ("transactions_count", "token_transfers_count", "gas_usage_count", "validations_count")
    valid = isinstance(payload, dict) and all(isinstance(payload.get(field), str) and re.fullmatch(r"0|[1-9]\d*", payload[field]) for field in fields)
    return {"schemaVersion": 1, "validator": "blockscout-address-counters", "valid": valid, "counts": {field: payload.get(field) for field in fields} if isinstance(payload, dict) else {}}


def validate_transfer_topics(payload: Any, address: str) -> dict[str, Any]:
    if not ADDRESS_RE.fullmatch(address):
        raise SweepError("address must contain exactly 20 hex bytes")
    address_topic = "0x" + "0" * 24 + address[2:].lower()
    rows = payload.get("result") if isinstance(payload, dict) and payload.get("status") == "1" else None
    conformant_rows = []
    if isinstance(rows, list):
        for row in rows:
            topics = row.get("topics") if isinstance(row, dict) else None
            if not isinstance(topics, list) or len(topics) < 3 or not all(isinstance(topic, str) for topic in topics[:3]):
                conformant_rows = []
                break
            lowered = [topic.lower() for topic in topics]
            if lowered[0] != TRANSFER_TOPIC or address_topic not in lowered[1:3]:
                conformant_rows = []
                break
            conformant_rows.append(lowered)
    outbound = sum(row[1] == address_topic and row[2] != address_topic for row in conformant_rows)
    inbound = sum(row[2] == address_topic and row[1] != address_topic for row in conformant_rows)
    return {
        "schemaVersion": 1,
        "validator": "etherscan-transfer-topics",
        "valid": bool(conformant_rows and outbound and inbound),
        "outboundOnlyResults": outbound,
        "inboundOnlyResults": inbound,
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    plan = subparsers.add_parser("plan")
    plan.add_argument("--input", required=True, type=Path)
    evaluate_parser = subparsers.add_parser("evaluate")
    evaluate_parser.add_argument("--plan", required=True, type=Path)
    evaluate_parser.add_argument("--responses", required=True, type=Path)
    validate_parser = subparsers.add_parser("validate")
    validate_parser.add_argument("validator", choices=("blockscout-address-counters", "etherscan-transfer-topics"))
    validate_parser.add_argument("--input", type=Path)
    validate_parser.add_argument("--address")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    try:
        if args.command == "plan":
            result = build_plan(load_json(args.input))
        elif args.command == "evaluate":
            result = evaluate(load_json(args.plan), load_json(args.responses))
        else:
            payload = load_json(args.input)
            if args.validator == "blockscout-address-counters":
                result = validate_blockscout(payload)
            else:
                if not args.address:
                    raise SweepError("etherscan-transfer-topics requires --address")
                result = validate_transfer_topics(payload, args.address)
    except SweepError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 64
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0 if result.get("valid", True) else 1


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Maintain and summarize an append-only autoresearch session."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import math
import os
import statistics
import sys
from pathlib import Path
from typing import Any


STATUSES = {"keep", "discard", "crash", "checks_failed"}
DIRECTIONS = {"higher", "lower"}


class SessionError(ValueError):
    pass


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")


def read_records(path: Path) -> list[dict[str, Any]]:
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        raise SessionError(str(exc)) from exc
    records: list[dict[str, Any]] = []
    for line_number, line in enumerate(lines, start=1):
        if not line.strip():
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError as exc:
            raise SessionError(f"line {line_number}: malformed JSON: {exc.msg}") from exc
        if not isinstance(record, dict):
            raise SessionError(f"line {line_number}: record must be an object")
        records.append(record)
    validate_records(records)
    return records


def validate_records(records: list[dict[str, Any]]) -> None:
    if not records or records[0].get("type") != "config":
        raise SessionError("first record must be a config record")
    configs: dict[int, dict[str, Any]] = {}
    last_run = 0
    for index, record in enumerate(records, start=1):
        if record.get("type") == "config":
            segment = record.get("segment")
            if not isinstance(segment, int) or segment < 0 or segment in configs:
                raise SessionError(f"record {index}: invalid or duplicate config segment")
            if record.get("direction") not in DIRECTIONS:
                raise SessionError(f"record {index}: direction must be higher or lower")
            if not isinstance(record.get("metricName"), str) or not record["metricName"]:
                raise SessionError(f"record {index}: metricName must be non-empty")
            configs[segment] = record
            continue
        status = record.get("status")
        if status not in STATUSES:
            raise SessionError(f"record {index}: invalid status")
        if not isinstance(record.get("run"), int) or record["run"] != last_run + 1:
            raise SessionError(f"record {index}: run numbers must be consecutive from 1")
        last_run = record["run"]
        segment = record.get("segment")
        if segment not in configs:
            raise SessionError(f"record {index}: run references an unknown segment")
        metric = record.get("metric")
        if isinstance(metric, bool) or not isinstance(metric, (int, float)) or not math.isfinite(metric):
            raise SessionError(f"record {index}: metric must be a finite number")
        for field in ("elapsedSeconds", "estimatedCost"):
            value = record.get(field, 0)
            if isinstance(value, bool) or not isinstance(value, (int, float)) or value < 0 or not math.isfinite(value):
                raise SessionError(f"record {index}: {field} must be a non-negative number")


def append_record(path: Path, record: dict[str, Any]) -> None:
    encoded = (json.dumps(record, separators=(",", ":"), ensure_ascii=False) + "\n").encode()
    descriptor = os.open(path, os.O_WRONLY | os.O_APPEND)
    try:
        os.write(descriptor, encoded)
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def current_config(records: list[dict[str, Any]]) -> dict[str, Any]:
    return max((record for record in records if record.get("type") == "config"), key=lambda item: item["segment"])


def summarize(records: list[dict[str, Any]]) -> dict[str, Any]:
    config = current_config(records)
    segment = config["segment"]
    runs = [record for record in records if record.get("status") in STATUSES and record["segment"] == segment]
    valid = [record for record in runs if record["status"] in {"keep", "discard"}]
    kept = [record for record in valid if record["status"] == "keep"]
    baseline = valid[0] if valid else None
    candidates = kept or ([baseline] if baseline else [])
    direction = config["direction"]
    best = (max if direction == "higher" else min)(candidates, key=lambda item: item["metric"]) if candidates else None
    metrics = [record["metric"] for record in valid]
    mad = None
    if len(metrics) >= 3:
        median = statistics.median(metrics)
        mad = statistics.median(abs(value - median) for value in metrics)
    delta = None if not baseline or not best else best["metric"] - baseline["metric"]
    improvement = None if delta is None else (delta if direction == "higher" else -delta)
    confidence = None if improvement is None or mad in (None, 0) else improvement / mad
    confidence_level = None
    if confidence is not None:
        confidence_level = "likely_real" if confidence >= 2 else "marginal" if confidence >= 1 else "within_noise"

    consecutive = 0
    best_so_far: float | None = None
    for record in valid:
        if record["status"] == "keep" and (
            best_so_far is None
            or (direction == "higher" and record["metric"] > best_so_far)
            or (direction == "lower" and record["metric"] < best_so_far)
        ):
            best_so_far = record["metric"]
            consecutive = 0
        else:
            consecutive += 1

    total_runs = len([record for record in records if record.get("status") in STATUSES])
    max_runs = config.get("maxRuns")
    elapsed = sum(float(record.get("elapsedSeconds", 0)) for record in runs)
    cost = sum(float(record.get("estimatedCost", 0)) for record in runs)
    convergence_runs = config.get("convergenceRuns")
    filled = None if not max_runs else min(10, math.floor(10 * total_runs / max_runs + 0.5))
    counts = {status: sum(record["status"] == status for record in runs) for status in sorted(STATUSES)}
    return {
        "schemaVersion": 1,
        "segment": segment,
        "metricName": config["metricName"],
        "direction": direction,
        "runsCompleted": total_runs,
        "segmentRuns": len(runs),
        "counts": counts,
        "baseline": baseline["metric"] if baseline else None,
        "best": best["metric"] if best else None,
        "bestRun": best["run"] if best else None,
        "delta": delta,
        "mad": mad,
        "confidence": confidence,
        "confidenceLevel": confidence_level,
        "consecutiveValidWithoutBest": consecutive,
        "converged": bool(convergence_runs is not None and consecutive >= convergence_runs),
        "budgets": {
            "runs": {"used": total_runs, "limit": max_runs, "exhausted": bool(max_runs is not None and total_runs >= max_runs)},
            "runtimeSeconds": {
                "used": elapsed,
                "limit": config.get("maxRuntimeSeconds"),
                "exhausted": bool(config.get("maxRuntimeSeconds") is not None and elapsed >= config["maxRuntimeSeconds"]),
            },
            "cost": {
                "used": cost,
                "limit": config.get("maxCost"),
                "exhausted": bool(config.get("maxCost") is not None and cost >= config["maxCost"]),
            },
        },
        "progress": None if filled is None else {"filled": filled, "empty": 10 - filled, "bar": "█" * filled + "░" * (10 - filled)},
        "runs": runs,
    }


def render_summary(summary: dict[str, Any]) -> str:
    counts = summary["counts"]
    progress = f" [{summary['progress']['bar']}]" if summary["progress"] else ""
    lines = [
        f"AUTORESEARCH SUMMARY{progress}",
        f"Metric: {summary['metricName']} ({summary['direction']} is better), segment {summary['segment']}",
        (
            f"Runs: {summary['segmentRuns']} segment | {counts['keep']} kept | {counts['discard']} discarded | "
            f"{counts['crash']} crashed | {counts['checks_failed']} checks_failed"
        ),
        f"Baseline: {summary['baseline']}",
        f"Best: {summary['best']}",
    ]
    if summary["confidence"] is None:
        reason = "zero noise" if summary["mad"] == 0 else "insufficient valid runs"
        lines.append(f"Confidence: N/A ({reason})")
    else:
        lines.append(f"Confidence: {summary['confidence']:.2f}x ({summary['confidenceLevel'].upper()})")
    return "\n".join(lines) + "\n"


def render_confidence(summary: dict[str, Any]) -> tuple[str, int]:
    valid_count = sum(summary["counts"][status] for status in ("keep", "discard"))
    if valid_count < 3:
        return f"Insufficient valid data in current segment: {valid_count} runs (need >= 3)\n", 1
    if summary["mad"] == 0:
        return "MAD is zero — no measurable noise in the data\n", 0
    if summary["confidence"] is None:
        return "No kept result available for confidence calculation\n", 0
    lines = [
        f"Confidence: {summary['confidence']:.2f}x ({summary['confidenceLevel'].upper()})",
        f"Baseline:   {summary['baseline']}",
        f"Best:       {summary['best']} ({summary['direction']} is better)",
        f"Delta:      {summary['delta']}",
        f"MAD:        {summary['mad']}",
    ]
    return "\n".join(lines) + "\n", 0


def init_command(args: argparse.Namespace) -> int:
    if args.file.exists():
        raise SessionError(f"session file already exists: {args.file}")
    args.file.parent.mkdir(parents=True, exist_ok=True)
    config = {
        "type": "config",
        "schemaVersion": 1,
        "segment": 0,
        "metricName": args.metric,
        "direction": args.direction,
        "maxRuns": args.max_runs,
        "maxRuntimeSeconds": args.max_runtime_seconds,
        "maxCost": args.max_cost,
        "convergenceRuns": args.convergence_runs,
        "createdAt": utc_now(),
    }
    descriptor = os.open(args.file, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        os.write(descriptor, (json.dumps(config, separators=(",", ":")) + "\n").encode())
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    print(json.dumps(config, indent=2))
    return 0


def record_command(args: argparse.Namespace) -> int:
    records = read_records(args.file)
    config = current_config(records)
    metric_name = args.metric_name or config["metricName"]
    if metric_name != config["metricName"]:
        if args.direction is None:
            raise SessionError("--direction is required when --metric-name starts a new segment")
        config = {
            **{key: config.get(key) for key in ("maxRuns", "maxRuntimeSeconds", "maxCost", "convergenceRuns")},
            "type": "config",
            "schemaVersion": 1,
            "segment": config["segment"] + 1,
            "metricName": metric_name,
            "direction": args.direction,
            "createdAt": utc_now(),
        }
        append_record(args.file, config)
        records.append(config)
    elif args.direction is not None and args.direction != config["direction"]:
        raise SessionError("direction cannot change without a new metric segment")
    run_number = 1 + sum(record.get("status") in STATUSES for record in records)
    record = {
        "type": "run",
        "run": run_number,
        "segment": config["segment"],
        "metric": args.metric,
        "status": args.status,
        "commit": args.commit,
        "description": args.description,
        "elapsedSeconds": args.elapsed_seconds,
        "estimatedCost": args.estimated_cost,
        "recordedAt": utc_now(),
    }
    append_record(args.file, record)
    print(json.dumps(record, indent=2))
    return 0


def status_command(args: argparse.Namespace) -> int:
    summary = summarize(read_records(args.file))
    if args.format == "json":
        print(json.dumps(summary, indent=2, ensure_ascii=False))
        return 0
    if not summary["runs"]:
        print("No experiment results found.")
        return 1
    if args.format == "summary":
        print(render_summary(summary), end="")
        return 0
    output, returncode = render_confidence(summary)
    print(output, end="")
    return returncode


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    init = subparsers.add_parser("init")
    init.add_argument("--file", type=Path, default=Path("autoresearch.jsonl"))
    init.add_argument("--metric", required=True)
    init.add_argument("--direction", choices=sorted(DIRECTIONS), required=True)
    init.add_argument("--max-runs", type=int, default=20)
    init.add_argument("--max-runtime-seconds", type=float, default=7200)
    init.add_argument("--max-cost", type=float, default=0)
    init.add_argument("--convergence-runs", type=int, default=5)
    init.set_defaults(handler=init_command)

    record = subparsers.add_parser("record")
    record.add_argument("--file", type=Path, default=Path("autoresearch.jsonl"))
    record.add_argument("--metric", type=float, required=True)
    record.add_argument("--metric-name")
    record.add_argument("--direction", choices=sorted(DIRECTIONS))
    record.add_argument("--status", choices=sorted(STATUSES), required=True)
    record.add_argument("--commit")
    record.add_argument("--description", default="")
    record.add_argument("--elapsed-seconds", type=float, default=0)
    record.add_argument("--estimated-cost", type=float, default=0)
    record.set_defaults(handler=record_command)

    status = subparsers.add_parser("status")
    status.add_argument("--file", type=Path, default=Path("autoresearch.jsonl"))
    status.add_argument("--format", choices=("json", "summary", "confidence"), default="json")
    status.set_defaults(handler=status_command)
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    try:
        if getattr(args, "max_runs", 1) is not None and getattr(args, "max_runs", 1) <= 0:
            raise SessionError("--max-runs must be positive")
        if getattr(args, "convergence_runs", 1) is not None and getattr(args, "convergence_runs", 1) <= 0:
            raise SessionError("--convergence-runs must be positive")
        return args.handler(args)
    except SessionError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 64


if __name__ == "__main__":
    raise SystemExit(main())

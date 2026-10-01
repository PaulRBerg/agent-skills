#!/usr/bin/env python3
"""Host-wide FIFO lease queue for DeBank access by concurrent agents.

DeBank's WAF rate-limits the whole browser, so one agent's burst blocks every other agent. Each agent takes a lease
here before touching debank.com and releases it afterwards; a reported block pauses the queue for everyone.
"""

from __future__ import annotations

import argparse
import contextlib
import fcntl
import json
import os
import signal
import sys
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path

DEFAULT_TTL = 600
DEFAULT_WAIT = 240
DEFAULT_BLOCK_MINUTES = 15
MAX_PROFILES = 25
# A queued agent must poll again within this window or lose its place, so an abandoned ticket cannot stall the queue.
STALE_TICKET_SECONDS = 120
POLL_SECONDS = 1.0

EXIT_QUEUED = 3
EXIT_LOST = 4


def state_dir() -> Path:
    override = os.environ.get("DEBANK_GATE_DIR")
    if override:
        return Path(override)
    base = os.environ.get("XDG_STATE_HOME") or str(Path.home() / ".local" / "state")
    return Path(base) / "evm-atlas" / "debank-gate"


def iso(ts: float) -> str | None:
    return datetime.fromtimestamp(ts, timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ") if ts else None


@contextlib.contextmanager
def locked_state():
    directory = state_dir()
    directory.mkdir(parents=True, exist_ok=True)
    with open(directory / "gate.lock", "w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        path = directory / "state.json"
        try:
            state = json.loads(path.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            state = {"queue": [], "lease": None, "cooldownUntil": 0}
        yield state
        tmp = path.with_suffix(".tmp")
        tmp.write_text(json.dumps(state, indent=2) + "\n")
        os.replace(tmp, path)


def prune(state: dict, now: float) -> None:
    lease = state["lease"]
    if lease and lease["expiresAt"] <= now:
        state["lease"] = None
    state["queue"] = [t for t in state["queue"] if now - t["seenAt"] < STALE_TICKET_SECONDS]


def enqueue(state: dict, label: str, profiles: int, now: float) -> str:
    ticket = {"id": uuid.uuid4().hex[:12], "label": label, "profiles": profiles, "enqueuedAt": now, "seenAt": now}
    state["queue"].append(ticket)
    return ticket["id"]


def try_grant(state: dict, ticket_id: str, ttl: int, now: float) -> str:
    """Return "granted", "queued", or "unknown" (never issued, or dropped as stale)."""
    prune(state, now)
    lease = state["lease"]
    if lease and lease["ticket"] == ticket_id:
        return "granted"
    ticket = next((t for t in state["queue"] if t["id"] == ticket_id), None)
    if ticket is None:
        return "unknown"
    ticket["seenAt"] = now
    if lease or now < state["cooldownUntil"] or state["queue"][0]["id"] != ticket_id:
        return "queued"
    state["queue"].pop(0)
    state["lease"] = {
        "ticket": ticket_id,
        "label": ticket["label"],
        "profiles": ticket["profiles"],
        "grantedAt": now,
        "expiresAt": now + ttl,
    }
    return "granted"


def renew(state: dict, ticket_id: str, ttl: int, now: float) -> bool:
    prune(state, now)
    lease = state["lease"]
    if not lease or lease["ticket"] != ticket_id:
        return False
    lease["expiresAt"] = now + ttl
    return True


def release(state: dict, ticket_id: str, now: float) -> None:
    prune(state, now)
    if state["lease"] and state["lease"]["ticket"] == ticket_id:
        state["lease"] = None
    state["queue"] = [t for t in state["queue"] if t["id"] != ticket_id]


def block(state: dict, ticket_id: str | None, minutes: float, now: float) -> None:
    state["cooldownUntil"] = max(state["cooldownUntil"], now + minutes * 60)
    if ticket_id:
        release(state, ticket_id, now)


def summary(state: dict, now: float, ticket_id: str | None = None) -> dict:
    lease = state["lease"]
    ids = [t["id"] for t in state["queue"]]
    return {
        "now": iso(now),
        "holder": {"label": lease["label"], "profiles": lease["profiles"], "expiresAt": iso(lease["expiresAt"])}
        if lease
        else None,
        "cooldownUntil": iso(state["cooldownUntil"]) if state["cooldownUntil"] > now else None,
        "queued": len(ids),
        **({"position": ids.index(ticket_id) + 1} if ticket_id in ids else {}),
    }


def emit(payload: dict) -> None:
    print(json.dumps(payload))


def cmd_acquire(args: argparse.Namespace) -> int:
    if not 1 <= args.profiles <= MAX_PROFILES:
        sys.exit(f"--profiles must be between 1 and {MAX_PROFILES}; split larger runs into batches")
    deadline = time.time() + args.wait
    ticket_id = args.ticket
    pending = True

    def cancel(signum, frame):
        raise SystemExit(128 + signum)

    signal.signal(signal.SIGTERM, cancel)
    try:
        while True:
            now = time.time()
            with locked_state() as state:
                result = try_grant(state, ticket_id, args.ttl, now) if ticket_id else "unknown"
                if result == "unknown":
                    ticket_id = enqueue(state, args.label, args.profiles, now)
                    result = try_grant(state, ticket_id, args.ttl, now)
                info = summary(state, now, ticket_id)
            if result == "granted":
                pending = False
                emit({"status": "granted", "ticket": ticket_id, "expiresAt": info["holder"]["expiresAt"]})
                return 0
            if now >= deadline:
                pending = False
                emit({"status": "queued", "ticket": ticket_id, **info})
                return EXIT_QUEUED
            time.sleep(min(POLL_SECONDS, deadline - now))
    finally:
        # An interrupted wait gives up its place instead of stalling the head of the queue.
        if pending and ticket_id:
            with locked_state() as state:
                release(state, ticket_id, time.time())


def cmd_renew(args: argparse.Namespace) -> int:
    now = time.time()
    with locked_state() as state:
        ok = renew(state, args.ticket, args.ttl, now)
        info = summary(state, now)
    if not ok:
        emit({"status": "lost", "ticket": args.ticket, **info})
        return EXIT_LOST
    emit({"status": "renewed", "ticket": args.ticket, "expiresAt": info["holder"]["expiresAt"]})
    return 0


def cmd_release(args: argparse.Namespace) -> int:
    now = time.time()
    with locked_state() as state:
        release(state, args.ticket, now)
        info = summary(state, now)
    emit({"status": "released", "ticket": args.ticket, **info})
    return 0


def cmd_block(args: argparse.Namespace) -> int:
    now = time.time()
    with locked_state() as state:
        block(state, args.ticket, args.minutes, now)
        info = summary(state, now)
    emit({"status": "cooldown", **info})
    return 0


def cmd_status(args: argparse.Namespace) -> int:
    now = time.time()
    with locked_state() as state:
        prune(state, now)
        info = summary(state, now)
        info["queue"] = [{"label": t["label"], "profiles": t["profiles"]} for t in state["queue"]]
    emit(info)
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    acquire = sub.add_parser("acquire", help="join the queue and wait for the lease")
    acquire.add_argument("--label", required=True, help="who is asking, e.g. 'sweep-eth holdings check'")
    acquire.add_argument("--profiles", type=int, default=1, help=f"profiles this lease covers (max {MAX_PROFILES})")
    acquire.add_argument("--ticket", help="resume a queued ticket and keep its place")
    acquire.add_argument("--wait", type=float, default=DEFAULT_WAIT, help="seconds to wait before returning queued")
    acquire.add_argument("--ttl", type=int, default=DEFAULT_TTL, help="lease lifetime in seconds")
    acquire.set_defaults(func=cmd_acquire)

    renew_parser = sub.add_parser("renew", help="extend a held lease")
    renew_parser.add_argument("--ticket", required=True)
    renew_parser.add_argument("--ttl", type=int, default=DEFAULT_TTL)
    renew_parser.set_defaults(func=cmd_renew)

    release_parser = sub.add_parser("release", help="free a lease or leave the queue")
    release_parser.add_argument("--ticket", required=True)
    release_parser.set_defaults(func=cmd_release)

    block_parser = sub.add_parser("block", help="report a WAF block: pause the queue and release the lease")
    block_parser.add_argument("--ticket")
    block_parser.add_argument("--minutes", type=float, default=DEFAULT_BLOCK_MINUTES)
    block_parser.set_defaults(func=cmd_block)

    sub.add_parser("status", help="show the holder, cooldown, and queue").set_defaults(func=cmd_status)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())

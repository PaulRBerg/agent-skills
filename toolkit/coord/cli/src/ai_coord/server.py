"""Local HTTP API for the coordination dashboard."""

from __future__ import annotations

import json
import time
from collections.abc import Callable
from datetime import UTC, datetime
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from threading import Lock
from typing import Any
from urllib.parse import urlparse

from ai_coord.coordinator import Coordinator
from ai_coord.store import Store

CACHE_SECONDS = 2
HEARTBEAT_SECONDS = 20
POLL_SECONDS = 1


def _coordinator() -> Coordinator:
    return Coordinator(Store())


def sse_snapshot_frame(payload: dict[str, Any]) -> bytes:
    """Encode one snapshot as a Server-Sent Event frame."""
    data = json.dumps(payload, separators=(",", ":"), sort_keys=True)
    return f"event: snapshot\ndata: {data}\n\n".encode()


class SnapshotService:
    """Build and share rate-limited machine-wide dashboard snapshots."""

    def __init__(
        self,
        coordinator_factory: Callable[[], Coordinator] = _coordinator,
        *,
        monotonic: Callable[[], float] = time.monotonic,
        utcnow: Callable[[], datetime] = lambda: datetime.now(UTC),
    ) -> None:
        self._coordinator_factory = coordinator_factory
        self._monotonic = monotonic
        self._utcnow = utcnow
        self._cache: tuple[float, dict[str, Any]] | None = None
        self._lock = Lock()

    def snapshot(self) -> dict[str, Any]:
        """Return a cached snapshot, refreshing no more than once every two seconds."""
        with self._lock:
            now = self._monotonic()
            if self._cache is not None and now - self._cache[0] < CACHE_SECONDS:
                return self._cache[1]
            payload = self._build_snapshot()
            self._cache = (now, payload)
            return payload

    def generation(self) -> int:
        """Read the change counter without refreshing provider inventory."""
        coordinator = self._coordinator_factory()
        try:
            return coordinator.store.generation()
        finally:
            coordinator.store.close()

    def _build_snapshot(self) -> dict[str, Any]:
        coordinator = self._coordinator_factory()
        try:
            payload = coordinator.snapshot(machine_wide=True).as_dict()
            payload["messages"] = coordinator.store.all_messages()
            payload["generated_at"] = self._utcnow().isoformat()
            payload["generation"] = coordinator.store.generation()
            return payload
        finally:
            coordinator.store.close()


class ApiRequestHandler(BaseHTTPRequestHandler):
    """Serve dashboard API requests from one shared SnapshotService."""

    service: SnapshotService

    def do_GET(self) -> None:
        path = urlparse(self.path).path
        if path == "/api/snapshot":
            try:
                payload = self.service.snapshot()
            except Exception as error:  # noqa: BLE001
                self._send_json(500, {"error": str(error)})
                return
            self._send_json(200, payload)
        elif path == "/api/events":
            self._serve_events()
        else:
            self._send_json(404, {"error": "not found"})

    def _send_json(self, status: int, payload: dict[str, Any]) -> None:
        body = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _serve_events(self) -> None:
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "keep-alive")
        self.end_headers()

        last_generation: int | None = None
        last_heartbeat = 0.0
        try:
            while True:
                generation = self.service.generation()
                now = time.monotonic()
                if generation != last_generation or now - last_heartbeat >= HEARTBEAT_SECONDS:
                    self.wfile.write(sse_snapshot_frame(self.service.snapshot()))
                    self.wfile.flush()
                    last_generation = generation
                    last_heartbeat = now
                time.sleep(POLL_SECONDS)
        except (BrokenPipeError, ConnectionResetError):
            return

    def log_message(self, format: str, *args: object) -> None:
        """Keep the local server quiet unless the caller elects to log it."""


def create_server(
    host: str = "127.0.0.1", port: int = 4477, service: SnapshotService | None = None
) -> ThreadingHTTPServer:
    """Create the local dashboard server without starting its request loop."""
    handler = type(
        "DashboardApiRequestHandler",
        (ApiRequestHandler,),
        {"service": service or SnapshotService()},
    )
    return ThreadingHTTPServer((host, port), handler)

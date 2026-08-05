"""Atomic claim acquisition and active-scope replacement."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from sqlite3 import Connection
from typing import TYPE_CHECKING, Any

from ai_coord.identity import Identity
from ai_coord.store import Store
from ai_coord.util import (
    MAX_MESSAGE_CHARS,
    UNHASHABLE_BLOB_HASH,
    any_overlap,
    benign_dirt_scopes,
    git_blob_hash,
    now_ts,
    overlapping_paths,
    overlaps_outside_coverage,
    paths_overlap,
    relevant_dirty,
    sanitize,
    scopes_cover,
)

if TYPE_CHECKING:
    from ai_coord.providers import InventoryResult

DIRT_HOLD_SECONDS = 90


@dataclass(frozen=True, slots=True)
class Outcome:
    kind: str
    code: int
    detail: str = ""
    paths: tuple[str, ...] = ()
    holders: tuple[str, ...] = ()
    broad_paths: tuple[str, ...] = ()

    def line(self) -> str:
        fields = [self.kind]
        if self.detail:
            fields.append(self.detail)
        fields.extend(self.paths)
        return "\t".join(fields)


@dataclass
class ClaimArbiter:
    """Coordinate one session's atomic path claim through a small interface."""

    store: Store
    refresh_inventory: Callable[[], InventoryResult]
    observe_git_dirt: Callable[[Path, float], tuple[tuple[str, ...], dict[str, dict[str, Any]]]]
    identity_display: Callable[[str, str], str]
    current_time: Callable[[], float] = now_ts

    def start(
        self,
        identity: Identity,
        root: Path,
        label: str,
        paths: tuple[str, ...],
        existing: dict[str, Any] | None,
    ) -> Outcome:
        if not paths:
            return self._save_intent(identity, root, label, existing)
        if existing and existing["state"] == "active":
            return self._update_active(identity, root, label, paths, existing)

        timestamp = self.current_time()
        inventory = self.refresh_inventory()
        all_dirty, observations = self.observe_git_dirt(root, timestamp)
        dirty = relevant_dirty(paths, all_dirty)
        benign_scopes = benign_dirt_scopes(root)
        residual_owners = self.store.residual_owners(str(root))
        existing_paths = tuple(existing["paths"]) if existing else ()
        created_at = (
            float(existing["created_at"])
            if existing and existing["state"] == "queued" and scopes_cover(existing_paths, paths)
            else timestamp
        )

        with self.store.transaction() as connection:
            claims = self.store.claims(str(root))
            active_blockers = [
                claim
                for claim in claims
                if claim["state"] == "active"
                and not self._same_identity(claim, identity)
                and any_overlap(paths, tuple(claim["paths"]))
            ]
            earlier_waiters = [
                claim
                for claim in claims
                if claim["state"] == "queued"
                and not self._same_identity(claim, identity)
                and float(claim["created_at"]) < created_at
                and claim.get("blocked_reason") != "legacy-pattern"
                and any_overlap(paths, tuple(claim["paths"]))
            ]
            unattributed_dirty = self._unattributed_dirty(dirty, claims)
            fresh_dirty, advisory_dirty = self._partition_dirty(
                unattributed_dirty,
                observations,
                residual_owners,
                benign_scopes,
                identity,
                timestamp,
            )

            if not inventory.complete:
                state, reason = "queued", "coverage"
                outcome = Outcome("UNKNOWN", 2, "coverage")
            elif fresh_dirty:
                state, reason = "queued", "dirty"
                outcome = Outcome("UNKNOWN", 2, f"dirty-settling:{','.join(fresh_dirty)}")
            elif active_blockers or earlier_waiters:
                state = "queued"
                reason = "overlap" if active_blockers else "waiter"
                outcome = self._blocked_outcome(paths, active_blockers or earlier_waiters)
            else:
                state, reason = "active", None
                detail = f"stale-dirt:{','.join(advisory_dirty)}" if advisory_dirty else ""
                outcome = Outcome("READY", 0, detail, paths)

            should_notify_holders = (
                existing is None
                or existing.get("blocked_reason") != "overlap"
                or set(existing_paths) != set(paths)
            )
            self.store.save_claim(
                connection,
                identity,
                repo_root=str(root),
                label=label,
                state=state,
                paths=paths,
                blocked_reason=reason,
                created_at=created_at,
                updated_at=timestamp,
            )
            if should_notify_holders:
                for blocker in active_blockers:
                    self.store.add_message(
                        connection,
                        identity,
                        Identity(str(blocker["client"]), str(blocker["session_id"])),
                        self._blocked_message(label, paths, blocker),
                        str(root),
                        timestamp,
                    )
        if outcome.kind == "READY" and advisory_dirty:
            baselines = {
                path: oid
                for path in advisory_dirty
                if (oid := git_blob_hash(root, path, write=True)) != UNHASHABLE_BLOB_HASH
            }
            self.store.replace_baselines(identity, baselines)
        return outcome

    def _update_active(
        self,
        identity: Identity,
        root: Path,
        label: str,
        paths: tuple[str, ...],
        existing: dict[str, Any],
    ) -> Outcome:
        existing_paths = tuple(existing["paths"])
        timestamp = self.current_time()
        if set(existing_paths) == set(paths):
            if existing["label"] != label:
                with self.store.transaction() as connection:
                    self.store.save_claim(
                        connection,
                        identity,
                        repo_root=str(root),
                        label=label,
                        state="active",
                        paths=paths,
                        blocked_reason=None,
                        created_at=float(existing["created_at"]),
                        updated_at=timestamp,
                    )
            return Outcome("READY", 0, paths=paths)

        all_dirty, observations = self.observe_git_dirt(root, timestamp)
        if scopes_cover(existing_paths, paths):
            with self.store.transaction() as connection:
                self._save_active_scope(
                    connection,
                    identity,
                    root,
                    label,
                    paths,
                    existing,
                    all_dirty,
                    (),
                    timestamp,
                )
            return Outcome("READY", 0, paths=paths)

        inventory = self.refresh_inventory()
        dirty = relevant_dirty(paths, all_dirty)
        benign_scopes = benign_dirt_scopes(root)
        residual_owners = self.store.residual_owners(str(root))
        with self.store.transaction() as connection:
            claims = self.store.claims(str(root))
            active_blockers = [
                claim
                for claim in claims
                if claim["state"] == "active"
                and not self._same_identity(claim, identity)
                and overlaps_outside_coverage(paths, tuple(claim["paths"]), existing_paths)
            ]
            queued_blockers = [
                claim
                for claim in claims
                if claim["state"] == "queued"
                and not self._same_identity(claim, identity)
                and claim.get("blocked_reason") != "legacy-pattern"
                and overlaps_outside_coverage(paths, tuple(claim["paths"]), existing_paths)
            ]
            unattributed_dirty = self._unattributed_dirty(dirty, claims)
            fresh_dirty, advisory_dirty = self._partition_dirty(
                unattributed_dirty,
                observations,
                residual_owners,
                benign_scopes,
                identity,
                timestamp,
            )

            if not inventory.complete:
                return Outcome("ACTIVE", 3, "update-unknown:coverage", existing_paths)
            if fresh_dirty:
                return Outcome(
                    "ACTIVE",
                    3,
                    f"update-unknown:dirty-settling:{','.join(fresh_dirty)}",
                    existing_paths,
                )
            blockers = [*active_blockers, *queued_blockers]
            if blockers:
                holders = tuple(
                    self.identity_display(str(claim["client"]), str(claim["session_id"]))
                    for claim in blockers
                )
                return Outcome(
                    "ACTIVE",
                    3,
                    f"update-blocked:{','.join(holders)}",
                    existing_paths,
                    holders,
                    self._broad_requested_paths(paths, blockers),
                )

            self._save_active_scope(
                connection,
                identity,
                root,
                label,
                paths,
                existing,
                all_dirty,
                advisory_dirty,
                timestamp,
            )
        return Outcome("READY", 0, paths=paths)

    def _save_active_scope(
        self,
        connection: Connection,
        identity: Identity,
        root: Path,
        label: str,
        paths: tuple[str, ...],
        existing: dict[str, Any],
        all_dirty: tuple[str, ...],
        advisory_dirty: tuple[str, ...],
        timestamp: float,
    ) -> None:
        existing_paths = tuple(existing["paths"])
        current = self.store.claim(identity)
        if (
            current is None
            or current["state"] != "active"
            or set(current["paths"]) != set(existing_paths)
        ):
            raise RuntimeError("active claim changed during scope update")
        released_dirty = tuple(
            path
            for path in relevant_dirty(existing_paths, all_dirty)
            if not relevant_dirty(paths, (path,))
        )
        baselines = {
            str(row["path"]): str(row["oid"])
            for row in self.store.baselines(identity)
            if relevant_dirty(paths, (str(row["path"]),))
        }
        baselines.update(
            {
                path: oid
                for path in advisory_dirty
                if (oid := git_blob_hash(root, path, write=True)) != UNHASHABLE_BLOB_HASH
            }
        )
        waiters = [
            claim
            for claim in self.store.claims(str(root))
            if claim["state"] == "queued"
            and claim.get("blocked_reason") != "legacy-pattern"
            and any_overlap(existing_paths, tuple(claim["paths"]))
            and not any_overlap(paths, tuple(claim["paths"]))
        ]
        self.store.save_claim(
            connection,
            identity,
            repo_root=str(root),
            label=label,
            state="active",
            paths=paths,
            blocked_reason=None,
            created_at=float(existing["created_at"]),
            updated_at=timestamp,
            baselines=baselines,
            residual_paths=released_dirty,
        )
        message = sanitize(
            f"Narrowed claim '{existing['label']}'; your queued claim may now be ready.",
            MAX_MESSAGE_CHARS,
        )
        for waiter in waiters:
            self.store.add_message(
                connection,
                identity,
                Identity(str(waiter["client"]), str(waiter["session_id"])),
                message,
                str(root),
                timestamp,
            )

    def _blocked_outcome(self, paths: tuple[str, ...], blockers: list[dict[str, Any]]) -> Outcome:
        holders = tuple(
            self.identity_display(str(claim["client"]), str(claim["session_id"]))
            for claim in blockers
        )
        overlaps = tuple(
            sorted(
                {
                    path
                    for claim in blockers
                    for path in overlapping_paths(paths, tuple(claim["paths"]))
                }
            )
        )
        return Outcome(
            "BLOCKED",
            3,
            ",".join(holders),
            overlaps,
            holders,
            self._broad_requested_paths(paths, blockers),
        )

    @staticmethod
    def _broad_requested_paths(
        paths: tuple[str, ...], blockers: list[dict[str, Any]]
    ) -> tuple[str, ...]:
        return tuple(
            sorted(
                {
                    requested
                    for blocker in blockers
                    for requested in paths
                    for owned in blocker["paths"]
                    if requested == "." or str(owned).startswith(f"{requested}/")
                }
            )
        )

    @staticmethod
    def _blocked_message(label: str, paths: tuple[str, ...], blocker: dict[str, Any]) -> str:
        blocker_paths = tuple(str(path) for path in blocker["paths"])
        overlaps = overlapping_paths(paths, blocker_paths)
        broad = tuple(
            sorted(
                {
                    owned
                    for owned in blocker_paths
                    for requested in paths
                    if owned == "." or requested.startswith(f"{owned}/")
                }
            )
        )
        if broad:
            text = (
                f"Narrow broad claim {', '.join(broad)} with ai-coord start if unrelated; "
                f"queued work '{label}' overlaps: {', '.join(overlaps)}."
            )
        else:
            text = f"Queued behind your claim: {label}; overlaps: {', '.join(overlaps)}."
        return sanitize(text, MAX_MESSAGE_CHARS)

    def _save_intent(
        self,
        identity: Identity,
        root: Path,
        label: str,
        existing: dict[str, Any] | None,
    ) -> Outcome:
        timestamp = self.current_time()
        state = str(existing["state"]) if existing else "intent"
        paths = tuple(existing["paths"]) if existing else ()
        with self.store.transaction() as connection:
            self.store.save_claim(
                connection,
                identity,
                repo_root=str(root),
                label=label,
                state=state,
                paths=paths,
                blocked_reason=existing.get("blocked_reason") if existing else None,
                created_at=float(existing["created_at"]) if existing else timestamp,
                updated_at=timestamp,
            )
        if state == "active":
            return Outcome("READY", 0, paths=paths)
        if state == "queued":
            return Outcome("BLOCKED", 3, "intent updated", paths)
        return Outcome("INTENT", 0, label)

    @staticmethod
    def _partition_dirty(
        dirty: tuple[str, ...],
        observations: dict[str, dict[str, Any]],
        residual_owners: dict[str, dict[str, Any]],
        benign_scopes: tuple[str, ...],
        identity: Identity,
        current: float,
    ) -> tuple[tuple[str, ...], tuple[str, ...]]:
        fresh: list[str] = []
        advisory: list[str] = []
        for path in dirty:
            residual = residual_owners.get(path)
            benign = any(paths_overlap(path, scope) for scope in benign_scopes)
            residual_own = residual is not None and (
                residual["client"],
                residual["session_id"],
            ) == (identity.client, identity.session_id)
            observation = observations[path]
            stale = current - float(observation["first_seen"]) >= DIRT_HOLD_SECONDS
            (advisory if benign or residual_own or stale else fresh).append(path)
        return tuple(fresh), tuple(advisory)

    @staticmethod
    def _same_identity(claim: dict[str, Any], identity: Identity) -> bool:
        return claim["client"] == identity.client and claim["session_id"] == identity.session_id

    @staticmethod
    def _unattributed_dirty(
        dirty: tuple[str, ...], claims: list[dict[str, Any]]
    ) -> tuple[str, ...]:
        owned_scopes = [
            scope for claim in claims if claim["state"] == "active" for scope in claim["paths"]
        ]
        return tuple(
            path for path in dirty if not any(paths_overlap(path, scope) for scope in owned_scopes)
        )

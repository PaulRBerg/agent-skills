"""Pure helpers for paths, text, time, and Git state."""

from __future__ import annotations

import os
import re
import secrets
import subprocess
import time
import tomllib
import unicodedata
from pathlib import Path

MAX_CALLSIGN_CODEPOINTS = 40
MAX_LABEL_CHARS = 80
MAX_MESSAGE_CHARS = 240
MAX_PRESENCE_CHARS = 200
MAX_SCOPE_CHARS = 120
_GLOB_CHARS = frozenset("*?[]")
UNHASHABLE_BLOB_HASH = "<deleted-or-unhashable>"
_EMOJI_VARIATION_SELECTORS = frozenset(("\ufe0e", "\ufe0f"))
_ZERO_WIDTH_JOINER = "\u200d"


def now_ts() -> float:
    """Return the current UTC Unix timestamp."""
    return time.time()


def sanitize(text: str, limit: int) -> str:
    """Collapse non-printable or repeated whitespace and cap the result."""
    printable = "".join(char if char.isprintable() else " " for char in text)
    collapsed = " ".join(printable.split())
    if len(collapsed) > limit:
        return collapsed[: limit - 1].rstrip() + "…"
    return collapsed


def callsign_key(text: str) -> str:
    """Return the machine-wide comparison key for a callsign or target."""
    folded = unicodedata.normalize("NFC", " ".join(text.split())).casefold()
    normalized = unicodedata.normalize("NFC", folded)
    return "".join(char for char in normalized if char not in _EMOJI_VARIATION_SELECTORS)


def normalize_callsign(text: str) -> str:
    """Normalize and validate one emoji-bearing session callsign."""
    normalized = unicodedata.normalize("NFC", text)
    if any(
        not char.isprintable()
        and not char.isspace()
        and char != _ZERO_WIDTH_JOINER
        and char not in _EMOJI_VARIATION_SELECTORS
        for char in normalized
    ):
        raise ValueError("callsign contains unsupported control characters")
    normalized = " ".join(normalized.split())
    if not normalized:
        raise ValueError("callsign must contain text")
    if len(normalized) > MAX_CALLSIGN_CODEPOINTS:
        raise ValueError(f"callsign exceeds {MAX_CALLSIGN_CODEPOINTS} Unicode code points")
    if not any(unicodedata.category(char)[0] in {"L", "N"} for char in normalized):
        raise ValueError("callsign must contain at least one letter or number")
    if not any(_is_emoji_code_point(ord(char)) for char in normalized):
        raise ValueError("callsign must contain at least one emoji")
    return normalized


def _is_emoji_code_point(code_point: int) -> bool:
    return (
        0x1F000 <= code_point <= 0x1FAFF
        or 0x2600 <= code_point <= 0x26FF
        or 0x2700 <= code_point <= 0x27BF
    )


def new_id() -> str:
    """Return a short collision-resistant local identifier."""
    return secrets.token_hex(4)


def git_root(cwd: Path) -> Path | None:
    """Resolve the Git worktree root containing cwd."""
    try:
        result = subprocess.run(
            ["git", "-C", str(cwd), "rev-parse", "--show-toplevel"],
            capture_output=True,
            check=False,
            text=True,
            timeout=5,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if result.returncode != 0:
        return None
    value = result.stdout.strip()
    return Path(value).resolve() if value else None


def normalize_scopes(scopes: tuple[str, ...], cwd: Path, root: Path) -> tuple[str, ...]:
    """Normalize literal file or directory scopes beneath root."""
    normalized: list[str] = []
    resolved_root = root.resolve()
    for raw_scope in scopes:
        if not raw_scope or any(char in raw_scope for char in _GLOB_CHARS):
            raise ValueError(f"invalid literal scope: {raw_scope!r}")
        candidate = Path(raw_scope).expanduser()
        if not candidate.is_absolute():
            candidate = cwd / candidate
        try:
            resolved_candidate = (
                candidate.parent.resolve(strict=False) / candidate.name
                if candidate.is_symlink()
                else candidate.resolve(strict=False)
            )
            relative = resolved_candidate.relative_to(resolved_root)
        except (OSError, RuntimeError, ValueError) as error:
            raise ValueError(f"scope is outside repository: {raw_scope}") from error
        value = relative.as_posix().removeprefix("./").rstrip("/") or "."
        if not all(char.isprintable() for char in value):
            raise ValueError(f"scope contains non-printable characters: {raw_scope!r}")
        if len(value) > MAX_SCOPE_CHARS:
            raise ValueError(f"scope exceeds {MAX_SCOPE_CHARS} characters: {raw_scope!r}")
        if value not in normalized:
            normalized.append(value)
    return tuple(normalized)


def paths_overlap(left: str, right: str) -> bool:
    """Return whether two normalized literal scopes overlap by ancestry."""
    if left == "." or right == ".":
        return True
    return left == right or left.startswith(f"{right}/") or right.startswith(f"{left}/")


def any_overlap(left: tuple[str, ...], right: tuple[str, ...]) -> bool:
    return any(paths_overlap(a, b) for a in left for b in right)


def overlapping_paths(left: tuple[str, ...], right: tuple[str, ...]) -> tuple[str, ...]:
    values = {a if len(a) >= len(b) else b for a in left for b in right if paths_overlap(a, b)}
    return tuple(sorted(values))


def git_dirty_paths(root: Path) -> tuple[str, ...]:
    """Return tracked, untracked, and both sides of renamed dirty paths."""
    try:
        result = subprocess.run(
            ["git", "-C", str(root), "status", "--porcelain=v1", "-z", "--untracked-files=all"],
            capture_output=True,
            check=False,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise RuntimeError(f"could not inspect Git dirt: {error}") from error
    if result.returncode != 0:
        detail = result.stderr.decode(errors="replace").strip() or f"exit {result.returncode}"
        raise RuntimeError(f"could not inspect Git dirt: {detail}")

    parts = result.stdout.split(b"\0")
    dirty: list[str] = []
    index = 0
    while index < len(parts):
        entry = parts[index]
        index += 1
        if not entry:
            continue
        decoded = entry.decode(errors="replace")
        if len(decoded) < 4:
            continue
        status = decoded[:2]
        path = decoded[3:]
        if path and path not in dirty:
            dirty.append(path)
        if ("R" in status or "C" in status) and index < len(parts):
            other = parts[index].decode(errors="replace")
            index += 1
            if other and other not in dirty:
                dirty.append(other)
    return tuple(dirty)


def git_blob_hash(root: Path, path: str, *, write: bool = False) -> str:
    """Hash one worktree path, returning a stable sentinel when it cannot be hashed."""
    command = ["git", "-C", str(root), "hash-object"]
    if write:
        command.append("-w")
    command.extend(("--", path))
    try:
        result = subprocess.run(
            command,
            capture_output=True,
            check=False,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError):
        return UNHASHABLE_BLOB_HASH
    value = result.stdout.strip()
    if result.returncode != 0 or not value:
        return UNHASHABLE_BLOB_HASH
    return value


def benign_dirt_scopes(root: Path) -> tuple[str, ...]:
    """Read validated literal benign-dirt prefixes from the repository config."""
    try:
        with (root / ".ai-coord.toml").open("rb") as stream:
            data = tomllib.load(stream)
        dirt = data.get("dirt")
        values = dirt.get("benign") if isinstance(dirt, dict) else None
        if not isinstance(values, list) or not all(isinstance(value, str) for value in values):
            return ()
        if any(Path(value).is_absolute() for value in values):
            return ()
        return normalize_scopes(tuple(values), root, root)
    except (OSError, tomllib.TOMLDecodeError, TypeError, ValueError):
        return ()


def relevant_dirty(scopes: tuple[str, ...], dirty_paths: tuple[str, ...]) -> tuple[str, ...]:
    return tuple(
        path for path in dirty_paths if any(paths_overlap(scope, path) for scope in scopes)
    )


def age_label(timestamp: float, current: float | None = None) -> str:
    reference = now_ts() if current is None else current
    seconds = max(0, int(reference - timestamp))
    if seconds < 60:
        return f"{seconds}s"
    if seconds < 3600:
        return f"{seconds // 60}m"
    if seconds < 86400:
        return f"{seconds // 3600}h"
    return f"{seconds // 86400}d"


def first_heading(markdown: str) -> str | None:
    """Return the first Markdown H1, sanitized as a claim label."""
    for line in markdown.splitlines():
        match = re.match(r"^#\s+(.+?)\s*$", line)
        if match:
            value = sanitize(match.group(1), MAX_LABEL_CHARS)
            return value or None
    return None


def private_state_dir() -> Path:
    """Resolve and create the private ai-coord state directory."""
    override = os.environ.get("AI_COORD_STATE_DIR")
    if override:
        directory = Path(override).expanduser()
    else:
        xdg = os.environ.get("XDG_STATE_HOME")
        base = Path(xdg).expanduser() if xdg else Path.home() / ".local" / "state"
        directory = base / "ai-coord"
    directory.mkdir(parents=True, exist_ok=True, mode=0o700)
    directory.chmod(0o700)
    return directory

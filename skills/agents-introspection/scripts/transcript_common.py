"""Shared redaction, pattern, and reader helpers for transcript mining scripts."""

from __future__ import annotations

import json
import os
import re
from collections import Counter, deque
from pathlib import Path
from typing import Any, Iterable


EMAIL_RE = re.compile(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b")
API_KEY_RE = re.compile(r"\b(?:sk|pk|rk|ghp|github_pat|xox[baprs])-[A-Za-z0-9_\-]{16,}\b")
GENERIC_SECRET_RE = re.compile(
    r"(?i)\b(?:api[_-]?key|access[_-]?token|secret(?:[_-]?key)?|private[_-]?key|password)\b"
    r"\s*[:=]\s*[\"']?(?![$<{[])([A-Za-z0-9_./+=-]{16,})"
)
EVM_ADDRESS_RE = re.compile(r"(?<![0-9a-fA-F])0x[0-9a-fA-F]{40}(?![0-9a-fA-F])")
HEX_64_RE = re.compile(r"(?<![0-9a-fA-F])0x[0-9a-fA-F]{64}(?![0-9a-fA-F])")
LONG_SECRETISH_RE = re.compile(r"(?<![A-Za-z0-9_/-])[A-Za-z0-9_+=/-]{48,}(?![A-Za-z0-9_/-])")

CORRECTION_PATTERNS = {
    "user-correction": re.compile(r"(?i)\b(actually|wrong|instead|i asked|not what|do not|don't|stop|you should)\b"),
    "instruction-reminder": re.compile(r"(?i)\b(AGENTS\.md|instructions?|follow .*rules?|violat(?:e|ed|ion))\b"),
}
VERIFICATION_PATTERNS = {
    "tests": re.compile(r"(?i)\b(test|pytest|vitest|unit tests?|integration tests?)\b"),
    "lint-format": re.compile(r"(?i)\b(lint|format|mdformat|prettier|ruff|eslint)\b"),
    "repo-check": re.compile(r"(?i)\b(just |uv run|cargo check|npm run|pnpm |bun test|verified|verification|passes?|passed)\b"),
}
THEME_PATTERNS = {
    "agent-skills": re.compile(r"(?i)\b(SKILL\.md|openai\.yaml|frontmatter|allow_implicit|agent skills?|skill catalog)\b"),
    "transcripts": re.compile(r"(?i)\b(transcript|session_index|sessions/|claude/projects|history\.jsonl)\b"),
    "markdown": re.compile(r"(?i)\b(markdown|mdformat|README\.md|AGENTS\.md)\b"),
    "git": re.compile(r"(?i)\b(git status|git diff|commit|branch|worktree|staged)\b"),
    "shell-tooling": re.compile(r"(?i)\b(zsh|bash|just|uv run|rg |fd |jq )\b"),
    "privacy": re.compile(r"(?i)\b(secret|redact|private key|api key|token|wallet|address)\b"),
}
CONTEXT_PREFIXES = (
    "# AGENTS.md instructions for",
    "<skill>",
    "<environment_context>",
    "<permissions instructions>",
    "<collaboration_mode>",
    "<turn_aborted>",
    "<command-message>",
    "Base directory for this skill:",
)
FAILURE_STATUSES = {"error", "failed", "failure", "cancelled", "canceled", "timed_out", "timeout"}
MAX_FULL_SESSION_BYTES = 2_000_000
SESSION_HEAD_RECORDS = 250
SESSION_TAIL_RECORDS = 750
METADATA_HEAD_RECORDS = 100


def redact_text(value: str | None) -> str:
    if not value:
        return ""
    text = EMAIL_RE.sub("<email>", value)
    text = API_KEY_RE.sub("<api-key>", text)
    text = GENERIC_SECRET_RE.sub(lambda match: match.group(0).split(match.group(1), 1)[0] + "<secret>", text)
    text = HEX_64_RE.sub("<tx-or-key-hash>", text)
    text = EVM_ADDRESS_RE.sub("<evm-address>", text)
    return LONG_SECRETISH_RE.sub("<secret-like-token>", text)


def truncate(value: str, limit: int = 160) -> str:
    return value if len(value) <= limit else value[: limit - 1].rstrip() + "..."


def count_privacy_gaps(strings: list[str]) -> Counter[str]:
    counter: Counter[str] = Counter()
    for text in strings:
        if EMAIL_RE.search(text):
            counter["email"] += 1
        if API_KEY_RE.search(text) or GENERIC_SECRET_RE.search(text):
            counter["api-key-or-secret"] += 1
        if HEX_64_RE.search(text):
            counter["private-key-or-tx-hash"] += 1
        if EVM_ADDRESS_RE.search(text):
            counter["evm-address"] += 1
        if LONG_SECRETISH_RE.search(text):
            counter["long-secret-like-token"] += 1
    return counter


def count_patterns(strings: list[str], patterns: dict[str, re.Pattern[str]]) -> Counter[str]:
    counter: Counter[str] = Counter()
    for text in strings:
        for name, pattern in patterns.items():
            if pattern.search(text):
                counter[name] += 1
    return counter


def keyword_alternatives(keyword: str) -> list[str]:
    return [alternative.strip() for alternative in keyword.split("|") if alternative.strip()]


def count_keywords(strings: list[str], keywords: list[str]) -> Counter[str]:
    counter: Counter[str] = Counter()
    lowered = [text.lower() for text in strings]
    for keyword in keywords:
        alternatives = [alternative.lower() for alternative in keyword_alternatives(keyword)]
        if not alternatives:
            continue
        matches = sum(1 for text in lowered if any(alternative in text for alternative in alternatives))
        if matches:
            counter[redact_text(keyword)] = matches
    return counter


def read_jsonl(path: Path) -> Iterable[Any]:
    try:
        with path.open("r", encoding="utf-8", errors="replace") as handle:
            for line in handle:
                item = parse_jsonl_line(line)
                if item is not None:
                    yield item
    except OSError:
        return


def read_jsonl_head(path: Path) -> Iterable[Any]:
    try:
        with path.open("r", encoding="utf-8", errors="replace") as handle:
            for _, line in zip(range(METADATA_HEAD_RECORDS), handle):
                item = parse_jsonl_line(line)
                if item is not None:
                    yield item
    except OSError:
        return


def read_jsonl_sampled(path: Path) -> Iterable[Any]:
    try:
        if path.stat().st_size <= MAX_FULL_SESSION_BYTES:
            yield from read_jsonl(path)
            return
    except OSError:
        return
    tail: deque[str] = deque(maxlen=SESSION_TAIL_RECORDS)
    try:
        with path.open("r", encoding="utf-8", errors="replace") as handle:
            for line_number, line in enumerate(handle):
                if line_number < SESSION_HEAD_RECORDS:
                    item = parse_jsonl_line(line)
                    if item is not None:
                        yield item
                else:
                    tail.append(line)
    except OSError:
        return
    for line in tail:
        item = parse_jsonl_line(line)
        if item is not None:
            yield item


def read_jsonl_sampled_with_lines(path: Path) -> Iterable[tuple[int, Any]]:
    try:
        if path.stat().st_size <= MAX_FULL_SESSION_BYTES:
            with path.open("r", encoding="utf-8", errors="replace") as handle:
                for line_number, line in enumerate(handle):
                    item = parse_jsonl_line(line)
                    if item is not None:
                        yield line_number, item
            return
    except OSError:
        return
    tail: deque[tuple[int, str]] = deque(maxlen=SESSION_TAIL_RECORDS)
    try:
        with path.open("r", encoding="utf-8", errors="replace") as handle:
            for line_number, line in enumerate(handle):
                if line_number < SESSION_HEAD_RECORDS:
                    item = parse_jsonl_line(line)
                    if item is not None:
                        yield line_number, item
                else:
                    tail.append((line_number, line))
    except OSError:
        return
    for line_number, line in tail:
        item = parse_jsonl_line(line)
        if item is not None:
            yield line_number, item


def read_raw_lines_sampled(path: Path) -> Iterable[str]:
    try:
        if path.stat().st_size <= MAX_FULL_SESSION_BYTES:
            with path.open("r", encoding="utf-8", errors="replace") as handle:
                yield from handle
            return
    except OSError:
        return
    tail: deque[str] = deque(maxlen=SESSION_TAIL_RECORDS)
    try:
        with path.open("r", encoding="utf-8", errors="replace") as handle:
            for line_number, line in enumerate(handle):
                if line_number < SESSION_HEAD_RECORDS:
                    yield line
                else:
                    tail.append(line)
    except OSError:
        return
    yield from tail


def parse_jsonl_line(line: str) -> Any | None:
    try:
        return json.loads(line) if line.strip() else None
    except json.JSONDecodeError:
        return None


def parse_content_blocks(content: Any) -> tuple[list[str], Counter[str], int]:
    texts: list[str] = []
    tools: Counter[str] = Counter()
    failures = 0
    if isinstance(content, str):
        texts.append(content)
    elif isinstance(content, list):
        for block in content:
            if isinstance(block, str):
                texts.append(block)
                continue
            if not isinstance(block, dict):
                continue
            block_type = str(block.get("type", ""))
            if block_type in {"text", "input_text", "output_text"} and isinstance(block.get("text"), str):
                texts.append(block["text"])
            elif block_type in {"tool_use", "tool_call", "function_call"}:
                name = block.get("name")
                if isinstance(name, str):
                    tools[redact_text(name)] += 1
            elif block_type in {"tool_result", "function_call_output"}:
                failures += int(has_structured_failure(block))
    elif isinstance(content, dict):
        nested_texts, nested_tools, nested_failures = parse_content_blocks([content])
        texts.extend(nested_texts)
        tools.update(nested_tools)
        failures += nested_failures
    return texts, tools, failures


def extract_record_channels(item: Any) -> dict[str, Any]:
    result: dict[str, Any] = {"user": [], "assistant": [], "context": [], "tools": Counter(), "failures": 0}
    if not isinstance(item, dict):
        return result
    payload = item.get("payload") if isinstance(item.get("payload"), dict) else None
    candidate = payload or item
    candidate_type = str(candidate.get("type", item.get("type", "")))
    role = candidate.get("role")
    message = candidate.get("message")
    if isinstance(message, dict):
        role = message.get("role", role)
        content = message.get("content")
    else:
        content = candidate.get("content")
        if content is None and isinstance(message, str):
            content = message

    if candidate_type == "user_message" and role is None:
        role = "user"
        content = candidate.get("message")
    texts, tools, failures = parse_content_blocks(content)
    result["tools"].update(tools)
    collect_structured_tools(candidate, result["tools"])
    result["failures"] += max(failures, int(has_structured_failure(candidate)))

    if role in {"system", "developer"} or item.get("type") in {"system", "developer"}:
        result["context"].extend(texts)
    elif role == "user" or item.get("type") == "user" or candidate_type == "user_message":
        for text in texts:
            if is_context_envelope(text):
                result["context"].append(text)
            else:
                result["user"].append(text)
    elif role == "assistant" or item.get("type") == "assistant":
        result["assistant"].extend(texts)
    return result


def collect_structured_tools(value: Any, counter: Counter[str]) -> None:
    if not isinstance(value, dict):
        return
    value_type = str(value.get("type", ""))
    if value_type in {"function_call", "tool_use", "tool_call"}:
        name = value.get("name") or value.get("tool_name")
        if isinstance(name, str):
            counter[redact_text(name)] += 1


def has_structured_failure(value: Any, depth: int = 0) -> bool:
    if depth > 6 or not isinstance(value, (dict, list)):
        return False
    if isinstance(value, list):
        return any(has_structured_failure(child, depth + 1) for child in value)
    if value.get("is_error") is True or value.get("isError") is True:
        return True
    exit_code = value.get("exit_code", value.get("exitCode"))
    if isinstance(exit_code, int) and exit_code != 0:
        return True
    status = value.get("status")
    if isinstance(status, str) and status.lower() in FAILURE_STATUSES:
        return True
    for key, child in value.items():
        if key in {"input", "arguments", "command", "text"}:
            continue
        if isinstance(child, (dict, list)) and has_structured_failure(child, depth + 1):
            return True
        if key in {"output", "content"} and isinstance(child, str):
            parsed = parse_json_value(child)
            if parsed is not None and has_structured_failure(parsed, depth + 1):
                return True
    return False


def parse_json_value(value: str) -> Any | None:
    value = value.strip()
    if not value.startswith(("{", "[")):
        return None
    try:
        return json.loads(value)
    except json.JSONDecodeError:
        return None


def extract_tool_failures(item: Any) -> list[dict[str, str | None]]:
    """Return failure details (tool, status, text) for tool_result/function_call_output blocks in a record."""
    if not isinstance(item, dict):
        return []
    payload = item.get("payload") if isinstance(item.get("payload"), dict) else None
    candidate = payload or item
    message = candidate.get("message")
    content = message.get("content") if isinstance(message, dict) else candidate.get("content")
    results: list[dict[str, str | None]] = []
    candidate_type = str(candidate.get("type", item.get("type", "")))
    if candidate_type in {"function_call_output", "tool_result"}:
        detail = tool_failure_detail(candidate)
        if detail is not None:
            results.append(detail)
    if isinstance(content, list):
        for block in content:
            if isinstance(block, dict) and str(block.get("type", "")) in {"function_call_output", "tool_result"}:
                detail = tool_failure_detail(block)
                if detail is not None:
                    results.append(detail)
    return results


def tool_failure_detail(block: dict[str, Any]) -> dict[str, str | None] | None:
    if not has_structured_failure(block):
        return None
    name = block.get("name") or block.get("tool_name")
    status, text = tool_failure_status_and_text(block)
    return {"tool": redact_text(name) if isinstance(name, str) else None, "status": status, "text": redact_text(text)}


def tool_failure_status_and_text(value: Any) -> tuple[str | None, str]:
    if not isinstance(value, dict):
        return None, ""
    exit_code = value.get("exit_code", value.get("exitCode"))
    status = value.get("status")
    if isinstance(status, str) and isinstance(exit_code, int):
        status_text: str | None = f"{status} (exit_code={exit_code})"
    elif isinstance(status, str):
        status_text = status
    elif isinstance(exit_code, int):
        status_text = f"exit_code={exit_code}"
    elif value.get("is_error") is True or value.get("isError") is True:
        status_text = "error"
    else:
        status_text = None
    for key in ("output", "content", "text"):
        raw = value.get(key)
        if isinstance(raw, str):
            parsed = parse_json_value(raw)
            if isinstance(parsed, dict):
                nested_status, nested_text = tool_failure_status_and_text(parsed)
                return status_text or nested_status, nested_text or raw
            return status_text, raw
        if isinstance(raw, list):
            texts = [entry.get("text") for entry in raw if isinstance(entry, dict) and isinstance(entry.get("text"), str)]
            if texts:
                return status_text, " ".join(texts)
    return status_text, ""


def is_context_envelope(text: str) -> bool:
    stripped = text.lstrip()
    return any(stripped.startswith(prefix) for prefix in CONTEXT_PREFIXES)


def normalize_path(value: str) -> Path:
    return Path(os.path.expanduser(value)).resolve(strict=False)


def is_within(candidate: Path, root: Path) -> bool:
    try:
        candidate.relative_to(root)
        return True
    except ValueError:
        return False


def first_string_shallow(item: Any, keys: tuple[str, ...]) -> str | None:
    if not isinstance(item, dict):
        return None
    for key in keys:
        value = item.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()
    return None


def deduplicate_messages(messages: Iterable[str]) -> list[str]:
    result: list[str] = []
    seen: set[str] = set()
    for message in messages:
        normalized = " ".join(message.split())
        if normalized and normalized not in seen:
            seen.add(normalized)
            result.append(message.strip())
    return result


def extract_strings(value: Any, depth: int = 0) -> Iterable[str]:
    if depth > 8:
        return
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for child in value.values():
            yield from extract_strings(child, depth + 1)
    elif isinstance(value, list):
        for child in value:
            yield from extract_strings(child, depth + 1)


def source_title(item: Any) -> str | None:
    if not isinstance(item, dict):
        return None
    for key in ("title", "summary", "task"):
        value = item.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()
    return None

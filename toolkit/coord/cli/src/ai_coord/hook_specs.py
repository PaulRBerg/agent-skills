"""Canonical lightweight hook definitions shared by runtime and installers."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class HookSpec:
    event: str
    command: str
    matcher: str | None = None
    timeout: int | None = None
    additional_context_limit: int | None = None
    if_filter: str | None = None
    async_: bool | None = None
    async_rewake: bool | None = None


CODEX_HOOK_SPECS = (
    HookSpec(
        "SessionStart",
        "ai-coord hook codex",
        matcher="startup|resume|clear",
        timeout=5,
    ),
    HookSpec(
        "UserPromptSubmit",
        "ai-coord hook codex",
        timeout=5,
        additional_context_limit=200,
    ),
    HookSpec("Stop", "ai-coord hook codex", timeout=5),
    HookSpec("SessionEnd", "ai-coord hook codex", timeout=3),
    HookSpec("SubagentStart", "ai-coord hook codex", timeout=5),
    HookSpec("SubagentStop", "ai-coord hook codex", timeout=5),
    HookSpec("PostToolUse", "ai-coord hook codex", timeout=5),
)
CLAUDE_HOOK_SPECS = (
    HookSpec("SessionStart", "ai-coord hook claude", timeout=5),
    HookSpec("UserPromptSubmit", "ai-coord hook claude", timeout=5),
    HookSpec("Stop", "ai-coord hook claude", timeout=5),
    HookSpec("SessionEnd", "ai-coord hook claude", timeout=3),
    HookSpec("SubagentStart", "ai-coord hook claude", timeout=5),
    HookSpec("SubagentStop", "ai-coord hook claude", timeout=5),
    HookSpec("PostToolUse", "ai-coord hook claude", matcher="ExitPlanMode", timeout=5),
    HookSpec("PostToolBatch", "ai-coord hook claude", timeout=5),
    HookSpec(
        "PostToolUseFailure",
        "ai-coord waker claude",
        matcher="Bash",
        timeout=3600,
        if_filter="Bash(ai-coord start *)",
        async_=True,
        async_rewake=True,
    ),
)


def hook_specs(client: str) -> tuple[HookSpec, ...]:
    if client == "codex":
        return CODEX_HOOK_SPECS
    if client == "claude":
        return CLAUDE_HOOK_SPECS
    raise ValueError(f"unsupported client: {client}")

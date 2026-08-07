---
compatibility: Requires macOS /usr/bin/pbcopy and local Claude Code or Codex CLI transcript storage.
coordination: exempt
disable-model-invocation: true
metadata:
  install-targets: claude-code codex
name: copy-transcript-path
user-invocable: true
description: Copy the active Claude Code or Codex CLI chat transcript path to the macOS clipboard.
---

# Copy Transcript Path

Copy the active chat's transcript path to the clipboard.

This skill is coordination-exempt: skip the ai-coord gate (`git status` / `ai-coord status` / `ai-coord start`) for this
skill's own work.

## Supported Chat Hosts

Before doing any work, identify the current chat host. If it is not Claude Code or Codex CLI, stop with this error:
`This skill only works in Claude Code or Codex CLI.`

## Workflow

Resolve `scripts/copy-transcript-path.sh` relative to this `SKILL.md` and run it with no arguments:

```sh
bash <skill-dir>/scripts/copy-transcript-path.sh
```

Return the helper's successful stdout verbatim and stop. Do not infer a transcript from the current project, select a
recent session, or add explanation.

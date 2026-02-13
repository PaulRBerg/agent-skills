# Codex CLI Configuration Options

## Model Selection (`-m` / `--model`)

| Model           | Description                          |
| --------------- | ------------------------------------ |
| `gpt-5.3-codex` | Latest frontier agentic coding model |

## Reasoning Effort (`-c model_reasoning_effort=`)

Configured via `-c model_reasoning_effort=<level>` or in `~/.codex/config.toml`.

| Level    | Description                       | When to Use                                      |
| -------- | --------------------------------- | ------------------------------------------------ |
| `low`    | Fast responses, minimal reasoning | Single file review, syntax check, quick question |
| `medium` | Balanced speed and depth          | Multi-file review, focused feature planning      |
| `high`   | Deeper analysis, slower responses | Architecture analysis, cross-cutting concerns    |
| `xhigh`  | Maximum reasoning depth           | Large codebase planning, comprehensive audit     |

**Selection**: Choose effort level based on task complexity (see SKILL.md Reasoning Effort Guidelines).

## Sandbox Modes (`-s` / `--sandbox`)

| Mode                 | Description                   | Use Case                     |
| -------------------- | ----------------------------- | ---------------------------- |
| `read-only`          | No file modifications allowed | **Planning and code review** |
| `workspace-write`    | Can modify files in workspace | Implementation tasks         |
| `danger-full-access` | Full system access            | System-level operations      |

## Global Flags

These flags work with all Codex commands:

| Flag                      | Description                                                 |
| ------------------------- | ----------------------------------------------------------- |
| `-C <dir>` / `--cd <dir>` | Set working directory                                       |
| `--search`                | Enable live web search (native Responses `web_search` tool) |
| `--add-dir <DIR>`         | Additional directories that should be writable              |
| `--full-auto`             | Shorthand for workspace-write with auto-approval            |

## Exec Subcommand Flags

These flags are specific to `codex exec`:

| Flag                     | Description                       |
| ------------------------ | --------------------------------- |
| `-o <file>`              | Write output to file              |
| `--json`                 | Output in JSONL event format      |
| `--output-schema <FILE>` | Structured output schema          |
| `--skip-git-repo-check`  | Bypass git repository requirement |

## Timeout Guidelines

Bash tool timeout caps at 600000ms (10 minutes). Choose based on reasoning effort:

| Effort           | Timeout   |
| ---------------- | --------- |
| `low` / `medium` | 300000ms  |
| `high` / `xhigh` | 600000ms  |

For `xhigh` tasks that may exceed 10 minutes, use `run_in_background: true` on the Bash tool call and set `CODEX_OUTPUT` so the wrapper writes to a known file you can read later.

## Example Commands

### Planning Query via Wrapper (high effort)

```bash
CODEX_OUTPUT="/tmp/codex-output.txt" \
EFFORT="high" \
scripts/run-codex-exec.sh <<'EOF'
Analyze this codebase and design an implementation plan for [feature].
EOF
```

### Code Review via Wrapper (medium effort)

```bash
EFFORT="medium" \
scripts/run-codex-exec.sh <<'EOF'
Review the recent changes for security vulnerabilities, focusing on SQL injection and XSS.
Include file paths and line references for each finding.
EOF
```

### Query with Web Search via Wrapper

```bash
CODEX_SEARCH=1 \
EFFORT="medium" \
scripts/run-codex-exec.sh <<'EOF'
Research current best practices for [topic] and recommend an approach.
EOF
```

## User Configuration

Override defaults by specifying in the prompt:

- "Use medium reasoning effort"
- "With high reasoning"
- "Enable web search"

**Model selection is restricted to the allowlist above.** User requests for unlisted models (e.g., "use o3") must be rejected.

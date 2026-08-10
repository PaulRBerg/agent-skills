# Context

`ai-notify` is a macOS Rust CLI that sends `terminal-notifier` alerts for Claude Code hooks and Codex CLI's `notify`
callback. Keep notification delivery macOS-specific while keeping pure logic and tests platform-independent; CI runs on
Ubuntu with nightly Rust.

## Upstream Documentation

- OpenAI Codex CLI `notify` callback: <https://learn.chatgpt.com/docs/config-file/config-advanced#notifications>
- Claude Code hook configuration and event schemas: <https://code.claude.com/docs/en/hooks>

## Development Workflow

- Bootstrap with the rolling nightly toolchain declared in `rust-toolchain.toml`; use the checked-in `Cargo.lock` for
  every Cargo build, test, and install.
- Run the checkout with `cargo run --locked -- ...`; `just install-cli` only installs the executable and never links an
  integration.
- Prefer the `justfile`: `just check` runs the release gate, `just test [cargo-test args]` runs tests, and `just fmt`
  formats Rust source. Use `just prettier-check` for a focused documentation/configuration check.
- Do not run `just install-cli` for ordinary verification; it writes to the user's Cargo installation directory.

## Architecture and Invariants

- Claude Code commands under `ai-notify event` read hook JSON from stdin. The `ai-notify codex` callback accepts Codex's
  JSON as its final argument or via `--stdin`; it does not create a tracked SQLite session.
- `integrations::HOOK_SPECS` is the source of truth for installed Claude hooks. The integration inspector derives its
  required event set from that list so `link claude` and `check` stay aligned.
- Preserve unrelated settings and hooks when changing integration writers. `link codex` must continue to refuse a
  different root `notify` value unless forced; profile names resolve to sibling `<profile>.config.toml` files.
- Configuration respects `XDG_CONFIG_HOME` and defaults to `~/.config/ai-notify`. Runtime configuration is cached for
  the life of the process.
- Claude `Stop` defers completion while `background_tasks` or `session_crons` are present. `StopFailure` alerts only in
  `all` mode and bypasses duration and prompt filters. Codex payloads lack duration, so Codex filtering applies only
  notification mode and prompt-prefix exclusions.
- SQLite uses WAL mode with `synchronous=NORMAL`; session data is intentionally transient rather than strictly durable.

## Testing

- Keep focused unit tests beside their Rust modules and CLI contract tests in `tests/cli.rs`; use `just test <filter>`
  for targeted verification.
- Isolate configuration and database paths with temporary directories; tests must not write to the user's actual XDG
  configuration directory.
- Inject or mock the macOS platform check, `terminal-notifier` discovery, and subprocess calls. Linux CI must not
  require the real notifier.
- For hook or Codex configuration changes, cover idempotence, preservation of unrelated configuration, and conflict
  behavior.

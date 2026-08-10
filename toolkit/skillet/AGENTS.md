# ai-skillet contributor guidance

Keep the CLI synchronous and library-owned: `src/main.rs` parses process arguments and maps
errors to exit codes, while behavior belongs in `src/lib.rs` and focused modules.

The supported public surface is `map`, `doctor`, `doctor --dependencies-only`, and `--version`.
Preserve the documented exit codes and deterministic text, JSON, and DOT output contracts.

Use the nightly minimal Rust toolchain configured at the monorepo root. Before proposing a change,
run the narrowest relevant locked Cargo check. Keep macOS and Linux compatibility; do not add
runtime services, plugin systems, config files, shell hooks, or completion generation without an
explicit product decision.

From the monorepo root, `cargo test -p ai-skillet --locked` is the focused package gate and
`just rust-check` is the aggregate Rust gate. `just install-cli` installs every workspace binary
under `~/.local`; do not run it for ordinary verification.

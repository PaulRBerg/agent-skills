# ai-skillet contributor guidance

Keep the CLI synchronous and library-owned: `src/main.rs` parses process arguments and maps
errors to exit codes, while behavior belongs in `src/lib.rs` and focused modules.

The supported public surface is `map`, `doctor`, `doctor --dependencies-only`, and `--version`.
Preserve machine-readable JSON and DOT output as later implementations fill in command behavior.

Use the stable minimal Rust toolchain configured in `rust-toolchain.toml`. Before proposing a
change, run the narrowest relevant locked Cargo check. Keep macOS and Linux compatibility; do not
add runtime services, plugin systems, config files, shell hooks, or completion generation without
an explicit product decision.

`just install-cli` is the only local installation path and must keep installing under
`${CARGO_INSTALL_ROOT:-$HOME/.local}`.

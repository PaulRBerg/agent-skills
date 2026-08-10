set default-list := true
set shell := ["bash", "-euo", "pipefail", "-c"]

# Build every Rust workspace package.
@build:
    cargo build --workspace --locked

# Run every Rust workspace test.
@test:
    cargo test --workspace --locked

# Lint every Rust target and reject warnings.
@lint:
    cargo clippy --workspace --all-targets --locked -- --deny warnings

# Format Rust source files.
@fmt:
    cargo fmt --all

# Check Rust formatting.
@fmt-check:
    cargo fmt --all -- --check

# Run the Rust validation gate.
@rust-check: fmt-check lint test build

# Run the coordination dashboard validation gate.
@coord-dashboard-check:
    just --justfile apps/coord-dashboard/justfile --working-directory apps/coord-dashboard check

# Start the coordination dashboard development server.
@coord-dashboard-dev:
    just --justfile apps/coord-dashboard/justfile --working-directory apps/coord-dashboard dev

# Run the handoffs validation gate.
@handoffs-check:
    just --justfile apps/handoffs/justfile --working-directory apps/handoffs check

# Start the handoffs development server.
@handoffs-dev:
    just --justfile apps/handoffs/justfile --working-directory apps/handoffs dev

# Run all workspace and application checks.
@check: rust-check coord-dashboard-check handoffs-check

# Install all local CLI packages under ~/.local.
@install-cli:
    cargo install --path commit --locked --force --root "$HOME/.local"
    cargo install --path coord --locked --force --root "$HOME/.local"
    cargo install --path notify --locked --force --root "$HOME/.local"
    cargo install --path skillet --locked --force --root "$HOME/.local"

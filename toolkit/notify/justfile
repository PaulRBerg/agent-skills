set shell := ["bash", "-euo", "pipefail", "-c"]
prettier_version := "3.5.3"

# Install only the CLI; integrations are configured explicitly with `ai-notify link`.
@install-cli:
    cargo install --path . --locked --force --root "$HOME/.local"
alias ic := install-cli

# Build the debug binary with the checked-in dependency graph.
@build:
    cargo build --locked

# Build the release binary with the checked-in dependency graph.
@build-release:
    cargo build --release --locked

# Run Rust tests. Pass a test filter or other cargo-test arguments as needed.
@test *args:
    cargo test --locked {{ args }}
alias t := test

# Format Rust source files.
@fmt:
    cargo fmt

# Check Rust formatting.
@fmt-check:
    cargo fmt --check

# Lint every Rust target, including tests, without allowing warnings.
@clippy:
    cargo clippy --all-targets --locked -- -D warnings

# Check Markdown and JSON with a pinned Prettier release.
@prettier-check:
    npx --yes prettier@{{ prettier_version }} --check '**/*.{json,jsonc,md}'
alias pc := prettier-check

# Format Markdown and JSON with the pinned Prettier release.
@prettier-write:
    npx --yes prettier@{{ prettier_version }} --write --log-level warn '**/*.{json,jsonc,md}'
alias pw := prettier-write

# Run every release-gate check.
@check:
    cargo fmt --check
    cargo clippy --all-targets --locked -- -D warnings
    cargo test --locked
    npx --yes prettier@{{ prettier_version }} --check '**/*.{json,jsonc,md}'
alias fc := check

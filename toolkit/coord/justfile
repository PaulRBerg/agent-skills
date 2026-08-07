# See https://just.systems/man/en/settings.html
set allow-duplicate-recipes
set allow-duplicate-variables
set default-list := true
set shell := ["bash", "-euo", "pipefail", "-c"]

# ---------------------------------------------------------------------------- #
#                                   PACKAGES                                   #
# ---------------------------------------------------------------------------- #

# Vite dashboard package recipes
mod dashboard

# ---------------------------------------------------------------------------- #
#                                   COMMANDS                                   #
# ---------------------------------------------------------------------------- #

# Install CLI globally and link host hooks
@install-cli:
    cargo install --locked --quiet --path . --force --root "${CARGO_INSTALL_ROOT:-$HOME/.local}"
    ai-coord link all
alias ic := install-cli

# Run the local dashboard API and Vite development server
[script("bash")]
dev:
    set -euo pipefail

    (
        cd dashboard
        exec bun run dev
    ) &
    dashboard_pid=$!

    cleanup() {
        kill "$dashboard_pid" 2>/dev/null || true
        wait "$dashboard_pid" 2>/dev/null || true
    }

    trap cleanup EXIT INT TERM
    cargo run --locked -- serve
alias d := dev

# Run Rust tests
@test *args:
    cargo test --locked {{ args }}
alias t := test

# ---------------------------------------------------------------------------- #
#                                    CHECKS                                    #
# ---------------------------------------------------------------------------- #

# Run all local checks and tests
[group("checks")]
@check:
    just _run-with-status cargo-fmt-check
    just _run-with-status cargo-clippy-check
    just _run-with-status test
    just _run-with-status prettier-check
    just _run-with-status dashboard::tsc-check
    just _run-with-status dashboard::test
    just _run-with-status dashboard::build
    echo ""
    echo -e '{{ GREEN }}All local checks passed!{{ NORMAL }}'
alias c := check

# Run all code checks
[group("checks")]
@full-check:
    just _run-with-status cargo-fmt-check
    just _run-with-status cargo-clippy-check
    just _run-with-status prettier-check
    just _run-with-status dashboard::tsc-check
    echo ""
    echo -e '{{ GREEN }}All code checks passed!{{ NORMAL }}'
alias fc := full-check

# Run all code fixes
[group("checks")]
@full-write:
    just _run-with-status cargo-fmt-write
    just _run-with-status prettier-write
    echo ""
    echo -e '{{ GREEN }}All code fixes applied!{{ NORMAL }}'
alias fw := full-write

# Check Rust formatting
[group("checks")]
@cargo-fmt-check:
    cargo fmt --all -- --check
alias cfc := cargo-fmt-check

# Format Rust sources
[group("checks")]
@cargo-fmt-write:
    cargo fmt --all
alias cfw := cargo-fmt-write

# Lint all Rust targets and reject warnings
[group("checks")]
@cargo-clippy-check:
    cargo clippy --all-targets --locked -- -D warnings
alias ccc := cargo-clippy-check

# Check Markdown, JSON, and dashboard source formatting with Prettier
[group("checks")]
@prettier-check:
    npx --yes prettier@3.9.6 --check "**/*.{json,jsonc,md,ts,tsx,css}"
alias pc := prettier-check

# Format Markdown, JSON, and dashboard sources with Prettier
[group("checks")]
@prettier-write:
    npx --yes prettier@3.9.6 --write --log-level warn "**/*.{json,jsonc,md,ts,tsx,css}"
alias pw := prettier-write

# ---------------------------------------------------------------------------- #
#                                   UTILITIES                                  #
# ---------------------------------------------------------------------------- #

[no-cd]
@_run-with-status recipe:
    echo ""
    echo -e '{{ CYAN }}→ Running {{ recipe }}...{{ NORMAL }}'
    just {{ recipe }}
    echo -e '{{ GREEN }}✓ {{ recipe }} completed{{ NORMAL }}'

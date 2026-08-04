# See https://just.systems/man/en/settings.html
set allow-duplicate-recipes
set allow-duplicate-variables
set default-list := true
set shell := ["bash", "-euo", "pipefail", "-c"]

# ---------------------------------------------------------------------------- #
#                                   PACKAGES                                   #
# ---------------------------------------------------------------------------- #

# Python CLI package recipes
mod cli

# Vite dashboard package recipes
mod dashboard

# ---------------------------------------------------------------------------- #
#                                   COMMANDS                                   #
# ---------------------------------------------------------------------------- #

# Install CLI globally and link host hooks
@install-cli:
    uv tool install --force ./cli
    ai-coord link all
alias ic := install-cli

# Run the local dashboard API and Vite development server
@dev:
    just dashboard::dev
alias d := dev

# Run CLI tests with pytest
@test *args:
    just cli::test {{ args }}
alias t := test

# ---------------------------------------------------------------------------- #
#                                    CHECKS                                    #
# ---------------------------------------------------------------------------- #

# Run all local checks and tests
[group("checks")]
@check:
    just _run-with-status full-check
    just _run-with-status cli::test
    just _run-with-status dashboard::test
    just _run-with-status dashboard::build
    echo ""
    echo -e '{{ GREEN }}All local checks passed!{{ NORMAL }}'
alias c := check

# Run all code checks
[group("checks")]
@full-check:
    just _run-with-status prettier-check
    just _run-with-status cli::ruff-check
    just _run-with-status cli::pyright-check
    just _run-with-status dashboard::tsc-check
    echo ""
    echo -e '{{ GREEN }}All code checks passed!{{ NORMAL }}'
alias fc := full-check

# Run all code fixes
[group("checks")]
@full-write:
    just _run-with-status prettier-write
    just _run-with-status cli::ruff-write
    echo ""
    echo -e '{{ GREEN }}All code fixes applied!{{ NORMAL }}'
alias fw := full-write

# Check Markdown, JSON, and dashboard source formatting with Prettier
[group("checks")]
@prettier-check:
    npx prettier --check "**/*.{json,jsonc,md,ts,tsx,css}"
alias pc := prettier-check

# Format Markdown, JSON, and dashboard sources with Prettier
[group("checks")]
@prettier-write:
    npx prettier --write --log-level warn "**/*.{json,jsonc,md,ts,tsx,css}"
alias pw := prettier-write

alias rc := cli::ruff-check
alias rw := cli::ruff-write
alias pyc := cli::pyright-check

# ---------------------------------------------------------------------------- #
#                                   UTILITIES                                  #
# ---------------------------------------------------------------------------- #

[no-cd]
@_run-with-status recipe:
    echo ""
    echo -e '{{ CYAN }}→ Running {{ recipe }}...{{ NORMAL }}'
    just {{ recipe }}
    echo -e '{{ GREEN }}✓ {{ recipe }} completed{{ NORMAL }}'

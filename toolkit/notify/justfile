# See https://just.systems/man/en/settings.html
set allow-duplicate-recipes
set allow-duplicate-variables
set shell := ["bash", "-euo", "pipefail", "-c"]
set unstable

# ---------------------------------------------------------------------------- #
#                                   COMMANDS                                   #
# ---------------------------------------------------------------------------- #

# Install CLI globally
@install-cli:
    uv tool install --force .
alias ic := install-cli

# Run tests with pytest
@test *args:
    uv run pytest {{ args }}
alias t := test

# ---------------------------------------------------------------------------- #
#                                    CHECKS                                    #
# ---------------------------------------------------------------------------- #

# Run all code checks
[group("checks")]
@full-check:
    just _run-with-status prettier-check
    just _run-with-status ruff-check
    just _run-with-status pyright-check
    echo ""
    echo -e '{{ GREEN }}All code checks passed!{{ NORMAL }}'
alias fc := full-check

# Run all code fixes
[group("checks")]
@full-write:
    just _run-with-status prettier-write
    just _run-with-status ruff-write
    echo ""
    echo -e '{{ GREEN }}All code fixes applied!{{ NORMAL }}'
alias fw := full-write

# Check Python formatting and linting with ruff
[group("checks")]
@ruff-check:
    uv run ruff check .
    uv run ruff format --check .
alias rc := ruff-check

# Auto-fix Python formatting and linting with ruff
[group("checks")]
@ruff-write:
    uv run ruff check --fix .
    uv run ruff format .
alias rw := ruff-write

# Check types with pyright
[group("checks")]
@pyright-check:
    uv run pyright
alias pyc := pyright-check

# Check Markdown formatting with prettier (readonly)
[group("checks")]
@prettier-check:
    npx prettier --check "**/*.{json,jsonc,md}"
alias pc := prettier-check

# Auto-fix Markdown formatting with prettier
[group("checks")]
@prettier-write:
    npx prettier --write --log-level warn "**/*.{json,jsonc,md}"
alias pw := prettier-write

# ---------------------------------------------------------------------------- #
#                                   UTILITIES                                  #
# ---------------------------------------------------------------------------- #

# Private recipe to run a check with formatted output
[no-cd]
@_run-with-status recipe:
    echo ""
    echo -e '{{ CYAN }}→ Running {{ recipe }}...{{ NORMAL }}'
    just {{ recipe }}
    echo -e '{{ GREEN }}✓ {{ recipe }} completed{{ NORMAL }}'
alias rws := _run-with-status

set allow-duplicate-variables
set allow-duplicate-recipes
set shell := ["bash", "-euo", "pipefail", "-c"]
set unstable

default:
    @just --list

install-deps: install-uv install-mdformat

install-mdformat:
    uv tool install mdformat --with mdformat-gfm --with mdformat-frontmatter

install-uv:
    curl -LsSf https://astral.sh/uv/install.sh | sh

mdformat-check:
    uvx --with mdformat-gfm --with mdformat-frontmatter mdformat --check .
alias mc := mdformat-check

mdformat-write:
    uvx --with mdformat-gfm --with mdformat-frontmatter mdformat .
alias mw := mdformat-write

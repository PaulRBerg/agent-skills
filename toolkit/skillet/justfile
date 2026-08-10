default:
    @just --list

test:
    cargo test --locked

conformance:
    cargo test --locked --test cli_surface --test catalog --test map --test doctor --test conformance

cargo-fmt-check:
    cargo fmt --all -- --check

cargo-clippy-check:
    cargo clippy --all-targets --locked -- -D warnings

prettier-check:
    bunx --bun prettier@3.9.6 --check .

check: cargo-fmt-check cargo-clippy-check test prettier-check

full-check: check

full-write:
    cargo fmt --all
    bunx --bun prettier@3.9.6 --write .

install-cli:
    cargo_install_root="${CARGO_INSTALL_ROOT:-$HOME/.local}"; cargo install --path . --locked --root "$cargo_install_root"

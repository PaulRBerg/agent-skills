# ai-skillet

`ai-skillet` inspects and maintains catalogs of agent skills.

## Status

Version 0.1.0 provides synchronous, no-network `map` and `doctor` engines. JSON reports use the
clean Rust schema version 1. The schema preserves the Python tools' consumer contracts, but output
is not byte-compatible with the Python implementation.

## Commands

```text
ai-skillet map [OPTIONS]
ai-skillet doctor [OPTIONS]
ai-skillet --version
```

`doctor --dependencies-only` limits diagnostics to skill-dependency declarations.

## Conformance contract

| Area         | Required contract                                                                                                                                          | Intentional version 1 behavior                                                                          |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| CLI          | Usage and operational errors exit 2; doctor findings exit 1; safe-fix failures exit 3                                                                      | Operational errors are emitted once with no generic duplicate                                           |
| Map output   | Deterministic text, JSON, and DOT; skills, roots, edges, duplicates, unresolved references, hashes, and portfolio exposures remain available               | Declared and inferred evidence remain independent; missing filters warn while returning an empty report |
| Discovery    | Explicit roots, broad-root exclusions, portfolio roots, ignored entries requested directly, symlink exposures, and paths containing newlines are supported | Local dependencies resolve across every scanned root                                                    |
| Streaming    | Large files and newline-free lines are scanned with bounded buffers; snippets are bounded match text                                                       | No ripgrep child process or cancellation lifecycle is required                                          |
| Doctor       | Every metadata, dependency, coordination, resource, README, prompt-hygiene, and CLI-version finding family is retained                                     | YAML and OpenAI policy diagnostics are structural; safe fixes are isolated and atomic                   |
| Dependencies | Bare and external identifiers, uniqueness, self-reference, resolution, and target-name ordering are validated                                              | External owner/repository case is preserved; repository names ending in `.git` are rejected             |

The integration tests in `tests/conformance.rs`, `tests/map.rs`, `tests/doctor.rs`, and
`tests/catalog.rs` are the executable contract. Python captures are migration evidence, not golden
output fixtures.

## Development

`rust-toolchain.toml` selects the stable Rust channel with the minimal profile plus `clippy` and
`rustfmt`.

```sh
just check
just test
```

Install a release build locally with `just install-cli`. It installs only under
`${CARGO_INSTALL_ROOT:-$HOME/.local}`.

## License

MIT. See [LICENSE.md](LICENSE.md).

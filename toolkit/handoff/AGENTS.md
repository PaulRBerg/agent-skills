# Development Instructions

`ai-handoff` is a stateless Rust workspace member. It shells out to native `git` for repository discovery and ignore
checks; do not introduce libgit2 or persisted CLI state.

## Architecture

- `src/cli.rs` defines the public command line; `src/error.rs` and `src/main.rs` map failures to stable exit classes.
- `src/create.rs` validates handoff metadata, publishes new handoffs atomically, and verifies clipboard commands.
- `src/archive.rs` moves completed handoffs into the user archive.
- `src/git.rs` is the only subprocess boundary for Git operations.

## Invariants

- Frontmatter has exactly the six documented keys in contract order.
- Creation never overwrites a target or traverses symlinked handoff directories.
- Failed creation removes its staged file, published target, and any directories created by that invocation.
- Tests isolate repositories, `HOME`, clipboard commands, and archive storage in temporary directories.

## Validation

From the monorepo root, run the narrowest relevant `cargo test -p ai-handoff` filter first, then `just rust-check` for
the aggregate Rust gate.

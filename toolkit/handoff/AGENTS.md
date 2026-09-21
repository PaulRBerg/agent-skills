# ai-handoff

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

## CLI reference

`ai-handoff` creates immutable task-handoff Markdown files, emits the exact Codex launch command for them, archives
completed handoffs without changing the rest of a document.

### Installation

Requires Git, Cargo, and the rolling Rust nightly toolchain:

```sh
cargo install --git https://github.com/PaulRBerg/agent-toolkit ai-handoff --locked --root "$HOME/.local"
```

For local development, install the current checkout instead:

```sh
cargo install --path . --locked --force --root "$HOME/.local"
```

### Commands

```text
ai-handoff create [--check] --repo <dir>... [--launch-repo <dir>]
                  --category <category> --task <task> [--draft <body.md>]
                  [--before-work-skill <dir>] [--no-clipboard] <FILENAME.md>
ai-handoff archive <handoff-path>
```

`create` canonicalizes and deduplicates Git worktrees. A single-repository handoff is published below that
repository's ignored `.ai/task-handoffs/` directory. A cross-repository handoff is published below
`$HOME/Desktop/.ai/task-handoffs/` and requires an explicit launch repository plus a `## Repository order` section.
Publication descends through no-follow directory handles, is no-overwrite and atomic, and clipboard commands are copied
through `pbcopy` and verified through `pbpaste` unless `--no-clipboard` is passed. `--draft` is required except with
`--check`, which validates placement without reading a draft or writing files. `--before-work-skill` requires an
absolute directory with a readable `SKILL.md` and appends an instruction to load it before any task work to the
generated Codex prompt. Generated handoff files abbreviate every occurrence of the active home directory as `~`;
reported paths and launch commands remain absolute.

`archive` moves a handoff to `$HOME/.local/share/task-handoffs/archive/<origin>/`, adding a UTC timestamp when the
name is occupied.

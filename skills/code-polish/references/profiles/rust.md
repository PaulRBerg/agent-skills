# Rust Profile

Load when the diff touches Rust source, Cargo manifests or workspaces, build scripts, toolchain configuration,
unsafe/FFI, async or concurrent code, or tests.

## Checks

- `RS-001` Recoverable panic (`HIGH`): `unwrap`, `expect`, indexing, `panic!`, or `unreachable!` is reachable through
  valid input or fallible external state rather than a proven invariant.
- `RS-002` Unsafe contract breach (`CRITICAL`): unsafe blocks or implementations, raw pointers, FFI, pinning, or layout
  code fail to uphold aliasing, lifetime, alignment, initialization, ownership, unwind, or `Send`/`Sync` invariants.
- `RS-003` Error contract loss (`HIGH`): errors are discarded, flattened into the wrong class or exit behavior, stripped
  of actionable context, or masked by cleanup.
- `RS-004` Async/concurrency lifecycle (`HIGH`): blocking work or lock guards cross an await, spawned work lacks
  ownership, or cancellation, panics, and channel closure can cause hangs, deadlocks, or lost failures.
- `RS-005` State/cleanup atomicity (`HIGH`): filesystem or process updates, temporary files, locks, or guards can leave
  partial state, remove resources they do not own, or prevent safe retry after interruption.
- `RS-006` OS/process boundary mismatch (`HIGH`): exit status, environment, or working directory assumptions go
  unchecked; path or byte handling forces UTF-8; or normalization and containment assumptions accept or reject the wrong
  target.
- `RS-007` Cargo/toolchain drift (`MEDIUM`): manifests, lockfiles, features, workspace membership, resolver, edition,
  MSRV, toolchain, or build outputs change inconsistently or make dependency resolution irreproducible.
- `RS-008` Test blind spot (`MEDIUM`): changed error paths, CLI or integration behavior, features, workspace members,
  targets, platforms, or concurrency behavior lack coverage.

## Evidence Expectations

- Show the triggering input, error path, schedule, or interruption point. For unsafe code, name the required invariant
  and the safe caller that can violate it.
- Do not flag panic-capable syntax alone; prove that the path is reachable without a programmer bug or broken invariant.
- Use repository-provided validation when present. Otherwise name the narrow applicable command, such as
  `cargo fmt --all --check`, `cargo clippy --all-targets --locked -- --deny warnings`, or
  `cargo test --locked <filter>`; add workspace, feature, or target coverage only when the changed contract requires it.

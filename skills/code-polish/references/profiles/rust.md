# Rust Profile

Load when the diff touches Rust source, Cargo manifests or workspaces, build scripts, toolchain configuration,
unsafe/FFI, async or concurrent code, or tests.

## Checks

- `RS-001` Recoverable panic (`HIGH`): `unwrap`, `expect`, indexing, `panic!`, or `unreachable!` is reachable through
  valid input or fallible external state rather than a proven invariant.
- `RS-002` Unsafe contract breach (`CRITICAL`): Unsafe blocks or implementations, raw pointers, FFI, pinning, or layout
  code violate aliasing, lifetime, alignment, initialization, ownership, unwind, or `Send`/`Sync` invariants.
- `RS-003` Error contract loss (`HIGH`): Code discards errors, flattens them into the wrong class or exit behavior,
  removes actionable context, or masks them during cleanup.
- `RS-004` Async/concurrency lifecycle (`HIGH`): Blocking work or lock guards cross an await, spawned work lacks
  ownership, or cancellation, panics, and channel closure can cause hangs, deadlocks, or lost failures.
- `RS-005` State/cleanup atomicity (`HIGH`): Filesystem or process updates, temporary files, locks, or guards can leave
  partial state, remove resources they do not own, or prevent safe retry after interruption.
- `RS-006` OS/process boundary mismatch (`HIGH`): Code does not check exit status, environment, or working directory
  assumptions, forces UTF-8 in path or byte handling, or accepts or rejects the wrong target through normalization and
  containment assumptions.
- `RS-007` Cargo/toolchain drift (`MEDIUM`): Manifests, lockfiles, features, workspace membership, resolver, edition,
  MSRV, toolchain, or build outputs change inconsistently or make dependency resolution irreproducible.
- `RS-008` Test blind spot (`MEDIUM`): Changed error paths, CLI or integration behavior, features, workspace members,
  targets, platforms, or concurrency behavior lack coverage.

## Evidence Expectations

- Show the triggering input, error path, schedule, or interruption point. For unsafe code, name the required invariant
  and the safe caller that can violate it.
- Do not flag panic-capable syntax alone. Prove that the path is reachable without a programmer bug or broken invariant.
- Use repository-provided validation when present. Otherwise name the narrow applicable command, such as
  `cargo fmt --all --check`, `cargo clippy --all-targets --locked -- --deny warnings`, or
  `cargo test --locked <filter>`. Add workspace, feature, or target coverage only when the changed contract requires it.

# Agent Toolkit

## Workspace boundaries

- The nightly Rust workspace contains the `ai-commit`, `ai-coord`, `ai-handoff`, `ai-notify`, and `ai-skillet` crates. Keep shared Rust configuration at the root and crate behavior within its crate.
- `apps/coord-dashboard` and `apps/handoffs` are independent Bun packages with separate locks and package-local validation. Do not combine their dependencies, scripts, or build outputs with the Rust workspace or each other.
- Context is source-owned: root files describe workspace-wide behavior; each package's README.md and AGENTS.md own its product and local workflow. Update the owning package rather than duplicating package guidance at the root.

## Validation

Use the root justfile for workspace changes: run the narrowest relevant check first, then `just check` when a change spans the workspace. Root `just check` runs the Rust gate and both Bun application gates; it does not install CLI binaries.

## Compatibility and safety

- Preserve each tool's documented command, output, and persisted-data contracts. Do not add compatibility paths, migrations, aliases, or dual formats unless explicitly requested; reject incompatible persisted versions with actionable errors.
- Changes that can invalidate live ai-coord agents, state, hooks, or CLI installation require explicit authorization and an isolated `AI_COORD_STATE_DIR` for development and validation. Never reset coordination state or replace global hooks from this checkout implicitly.
- Updating the installed CLIs (`just install-cli`) is pre-authorized when the committed changes preserve the documented command, output, and persisted-data contracts, or when `ai-coord` shows no other live agent sessions on this machine. Otherwise ask the user to close the other agents before installing.
- Keep both Bun applications local-only. In particular, handoffs must remain read-only and bound to loopback; the coordination dashboard must not make external requests.

# Toolkit

This workspace lives at `toolkit/` inside the agent-skills catalog repository. The catalog skill rules in the root
`AGENTS.md` govern `skills/`, not the toolkit; the toolkit follows this file and its package-local AGENTS.md files.
Toolkit commits use the catalog's natural-language commit format.

## Workspace boundaries

- The nightly Rust workspace contains the `ai-commit`, `ai-coord`, `ai-handoff`, `ai-notify`, and `ai-skillet` crates.
  Keep shared Rust configuration at the workspace root (`toolkit/`) and crate behavior within its crate.
- `apps/coord-dashboard` and `apps/handoffs` are independent Bun packages with separate locks and package-local
  validation. Do not combine their dependencies, scripts, or build outputs with the Rust workspace or each other.
- Context is source-owned: this file describes workspace-wide behavior; package-local AGENTS.md files own product and
  workflow guidance. Update the owning package rather than duplicating its guidance here.

## Validation

Use `toolkit/justfile` for workspace changes (`just <recipe>` inside `toolkit/`, or `just toolkit::<recipe>` from the
catalog root): run the narrowest relevant check first, then `just check` when a change spans the workspace. `just check`
runs the Rust gate and both Bun application gates; it does not install CLI binaries.

## Compatibility and safety

- Preserve each tool's documented command, output, and persisted-data contracts. Do not add compatibility paths,
  migrations, aliases, or dual formats unless explicitly requested; reject incompatible persisted versions with
  actionable errors.
- Changes that can invalidate live ai-coord agents, state, hooks, or CLI installation require explicit authorization and
  an isolated `AI_COORD_STATE_DIR` for development and validation. Never reset coordination state or replace global
  hooks from this checkout implicitly.
- Updating the installed CLIs (`just install-cli`, or `just toolkit::install-cli` from the catalog root) is
  pre-authorized when the committed changes preserve the documented command, output, and persisted-data contracts, or
  when `ai-coord` shows no other live agent sessions on this machine. Otherwise ask the user to close the other agents
  before installing.
- Keep both Bun applications local-only. In particular, handoffs must remain read-only and bound to loopback; the
  coordination dashboard must not make external requests.

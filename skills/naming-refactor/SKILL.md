---
compatibility: Requires Git, uv, and local command and edit access.
disable-model-invocation: true
name: naming-refactor
user-invocable: true
description: Refactor naming and repository structure exhaustively while preserving behavior and external contracts.
---

# Naming Refactor

Make every name in the current repository communicate one coherent domain model, regardless of refactor cost.

## Contract

- Cover the complete current Git repository. Do not narrow the run to selected files or stop after a candidate cap.
- Preserve features, runtime behavior, side effects, performance-sensitive characteristics, and externally consumed
  contracts. Large diffs, path moves, and repository-controlled interface migrations are allowed.
- Treat an exported surface as repository-controlled only when every consumer is in scope and can migrate atomically.
  Preserve other API names, CLI flags, environment variables, configuration keys, wire fields, routes, database names,
  and import paths unless the user explicitly authorizes a breaking migration.
- Fix a bug discovered during the refactor only when the defect and intended behavior are clear and a regression check
  can prove the fix. Keep bug fixes in a distinct change wave and report them separately.
- Preserve pre-existing work. Local edits, file moves, directory restructuring, and non-destructive validation are
  authorized. Do not commit, push, publish, or write externally unless the user or repository instructions require it.
- A verified no-op is valid only after exhaustive coverage. Do not rename a clear, conventional name merely to create
  churn, but do not retain a weak name to minimize diff size.

## Coverage Ledger

Resolve `scripts/naming-ledger.py` relative to this `SKILL.md` and create its ledger outside the repository:

```sh
uv run "<skill-dir>/scripts/naming-ledger.py" init --root <repo> --ledger <scratch.json>
```

The helper maps every tracked and non-ignored untracked path and records pre-existing worktree state. Account for a path
only after inspecting its name, relevant contents, and role:

```sh
uv run "<skill-dir>/scripts/naming-ledger.py" mark \
  --ledger <scratch.json> --status <pending|retained|renamed|excluded|blocked> \
  --path <path> [--path <path>...] [--reason <text>]
```

Use `retained` when the current name is justified, `renamed` when the path or its contents joined a verified rename,
`excluded` for generated, vendored, binary, or bulk artifacts validated through their source or invariant, and `blocked`
when behavior or contract safety cannot be established. `excluded` and `blocked` require reasons.

```sh
uv run "<skill-dir>/scripts/naming-ledger.py" pending --ledger <scratch.json> [--limit <n>]
uv run "<skill-dir>/scripts/naming-ledger.py" refresh --ledger <scratch.json>
uv run "<skill-dir>/scripts/naming-ledger.py" summary --ledger <scratch.json>
```

Refresh after path moves and before final validation. New paths become pending; removed paths remain in the ledger and
must be accounted as renamed, excluded, or blocked. The run is complete only when the helper reports no pending or
blocked paths.

## Ground the Refactor

1. Read applicable repository instructions. Record the repository root, starting commit and status, build and check
   commands, generated sources, and ownership boundary for pre-existing changes.
2. Initialize the ledger and establish baseline format, lint, type, test, build, codegen, API-snapshot, or smoke checks
   appropriate to the repository. Attribute existing failures before editing.
3. Identify external contracts, repository-controlled consumers, reflection and serialization surfaces, dynamic imports,
   case-insensitive filesystem constraints, and language-aware rename tooling.
4. Derive canonical domain vocabulary from behavior, types, data flow, documentation, tests, and relevant history.
   History resolves unclear intent; it does not override the current design.

Completion of this phase requires a recorded baseline, explicit contract boundaries, and a ledger covering the entire
repository.

## Build the Rename Map

Inspect every ledger path and build an evidence-backed map before changing each coherent domain slice. Cover
directories, files, packages, modules, namespaces, exports, types, classes, functions, methods, parameters, variables,
booleans, constants, tests, fixtures, documentation, configuration, scripts, and CI.

Apply these rules together:

- Give one concept one canonical term; give distinct concepts distinct terms.
- Name by domain role, behavior, ownership, lifecycle, units, and polarity rather than incidental implementation.
- Replace misleading, overloaded, contextless, or generic names such as `data`, `info`, `item`, `manager`, `process`,
  `handle`, and `utils` when a specific name is supported by evidence.
- Align directory, filename, primary export, and module responsibility. Move paths when the current structure obscures
  ownership or forces names to compensate for poor context.
- Preserve required language, framework, protocol, and ecosystem idioms. Do not perform a repository-wide casing or
  synonym rewrite when existing terminology is already coherent.

For every rename group, record the old and new concept, rationale, contract classification, affected consumers,
collision risks, dynamic string references, migration order, and proving checks. Resolve ambiguity through symbol and
reference inspection before choosing a name. Never use blind global replacement for an overloaded term.

Completion of this phase requires every non-excluded path to be retained with a reasoned naming model, assigned to a
validated rename group, or marked blocked with concrete evidence.

## Execute in Verified Waves

Apply rename groups in coherent dependency waves. Parallelize independent read-only analysis when useful, but serialize
overlapping edits and shared manifests, exports, schemas, and entrypoints.

- Prefer language-server, compiler, or AST-aware rename support for symbols. Use exact text replacement only after
  proving each occurrence has the same meaning.
- Move a tracked path with a repository-safe mechanism that does not stage unrelated work. Use an intermediate path for
  case-only renames on case-insensitive filesystems.
- Update definitions, consumers, imports, re-exports, tests, fixtures, docs, examples, configuration, CI, selectors,
  reflection, serialization, and generated sources in the same wave.
- Change generated output through its generator or schema, then regenerate and verify it. Do not edit vendored sources.
- Run the narrowest proving checks after each wave. Fix attributable failures before continuing; if parity cannot be
  established, revert only that wave's edits without repository-wide reset, clean, checkout, or stash commands.
- Mark ledger paths only after the wave is verified. Refresh the ledger after moves so new paths enter coverage.

Continue until every planned rename is applied or blocked; refactor cost, diff size, and elapsed time are not stopping
criteria.

## Final Verification and Report

Refresh the ledger, inspect every new path, and repeat the semantic naming pass until it finds no material naming issue.
Search for stale old names and paths, including case variants and non-code literals. Run aggregate repository checks and
compare them with baseline; no new unexplained failure is acceptable. Verify stable external contracts through available
API snapshots, schemas, CLI help, import surfaces, or focused smoke tests.

Lead success with `### ✅ Naming refactor complete — <rename groups> groups · <accounted>/<mapped> paths accounted`.
Report exact file and directory moves, every public or exported rename, compact local-identifier group counts,
baseline-versus-final checks, intentional retained external names, incidental bug fixes, and residual risks. Keep
commands, paths, names, diagnostics, and contract identifiers exact and undecorated.

If the ledger is incomplete, behavior parity is unproven, or an external contract would require unapproved breakage,
lead with `### ⛔ Naming refactor incomplete` and report the blocking evidence and required decision. Do not describe
the run as complete while any path remains pending or blocked.

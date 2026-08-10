# ai-skillet

`ai-skillet` inspects and maintains catalogs of agent skills.

## Status

Version 1.0.0 provides synchronous, no-network `map` and `doctor` engines. JSON reports use the
clean Rust schema version 1. The schema preserves the Python tools' consumer contracts, but output
is not byte-compatible with the Python implementation.

## Commands

```text
ai-skillet map [OPTIONS]
ai-skillet doctor [OPTIONS]
ai-skillet --version
```

`doctor --dependencies-only` limits diagnostics to skill-dependency declarations.

A catalog root that exposes `skills/` must provide a `README.md` with an exact `## Skills`
section and a Markdown table. The required first column is `Skill` and lists every active skill
name; additional columns are optional and ignored by the inventory validator. Conventional
installed roots named `.agents`, `.claude`, or `.codex` do not require a catalog README inventory.

## Doctor validation contract

`ai-skillet doctor --root <skill-or-catalog-root>` is the canonical deterministic, offline local
validator for the supported extended skill dialect. It accepts this one top-level field union:

- Portable [Agent Skills](https://agentskills.io/specification): `name`, `description`, `license`,
  `compatibility`, `metadata`, and `allowed-tools`.
- [Claude Code extensions](https://code.claude.com/docs/en/skills#frontmatter-reference):
  `when_to_use`, `argument-hint`, `arguments`, `disable-model-invocation`, `user-invocable`,
  `disallowed-tools`, `model`, `effort`, `context`, `agent`, `background`, `hooks`, `paths`, and
  `shell`.
- Repository extensions: `coordination` and `skill-dependencies`.

Unknown top-level fields are errors. `metadata` must be a string-to-string mapping;
`metadata.install-targets` additionally accepts only `claude-code`, `codex`, or
`claude-code codex`. Tool, argument, and path fields accept a string or a list of strings, while
`hooks` must be a mapping. Claude Boolean fields accept `true`/`false`, `yes`/`no`, `on`/`off`,
or `1`/`0`; other YAML shapes are not coerced. `context` accepts only `fork`, `effort` accepts
`low`, `medium`, `high`, `xhigh`, or `max`, and `shell` accepts `bash` or `powershell`. `agent`
and `background` require `context: fork`.

`coordination: exempt` requires this exact sentence in ordinary Markdown body prose:

```text
This skill is coordination-exempt: skip the ai-coord gate for its declared work.
```

Inline code, fenced or indented code, blockquotes, and clearly headed `Example` or `Examples`
sections do not count as declarations and do not trigger missing-frontmatter errors.

New schema diagnostics use these stable codes:

- Unknown field: `FRONTMATTER_UNKNOWN_FIELD`.
- Invalid types: `LICENSE_INVALID_TYPE`, `ALLOWED_TOOLS_INVALID_TYPE`,
  `WHEN_TO_USE_INVALID_TYPE`, `ARGUMENTS_INVALID_TYPE`, `DISALLOWED_TOOLS_INVALID_TYPE`,
  `MODEL_INVALID_TYPE`, `EFFORT_INVALID_TYPE`, `BACKGROUND_INVALID_TYPE`,
  `HOOKS_INVALID_TYPE`, `PATHS_INVALID_TYPE`, `SHELL_INVALID_TYPE`, and
  `METADATA_VALUE_INVALID_TYPE`. Existing field-specific type codes remain unchanged.
- Invalid values: `EFFORT_INVALID_VALUE` and `SHELL_INVALID_VALUE`, alongside the retained
  compatibility, context, coordination, and install-target codes.
- Cross-field errors: `AGENT_CONTEXT_REQUIRED` and `BACKGROUND_CONTEXT_REQUIRED`.
- Redundant defaults: `DISABLE_MODEL_INVOCATION_REDUNDANT_DEFAULT` warns on explicit `false`,
  and `USER_INVOCABLE_REDUNDANT_DEFAULT` warns on explicit `true`. Omission preserves those
  effective defaults.

These diagnostics add findings without changing JSON schema version 1, deterministic finding
order, exit codes, or the existing `--fix-safe` boundary.

## Conformance contract

| Area         | Required contract                                                                                                                                          | Intentional version 1 behavior                                                                          |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| CLI          | Usage and operational errors exit 2; doctor findings exit 1; safe-fix failures exit 3                                                                      | Operational errors are emitted once with no generic duplicate                                           |
| Map output   | Deterministic text, JSON, and DOT; skills, roots, edges, duplicates, unresolved references, hashes, and portfolio exposures remain available               | Declared and inferred evidence remain independent; missing filters warn while returning an empty report |
| Discovery    | Explicit roots, broad-root exclusions, portfolio roots, ignored entries requested directly, symlink exposures, and paths containing newlines are supported | Local dependencies resolve across every scanned root                                                    |
| Streaming    | Large files and newline-free lines are scanned with bounded buffers; snippets are bounded match text                                                       | No ripgrep child process or cancellation lifecycle is required                                          |
| Doctor       | The complete supported frontmatter union plus every metadata, dependency, coordination, resource, README, prompt-hygiene, and CLI-version finding family is validated | YAML and OpenAI policy diagnostics are structural; safe fixes are isolated and atomic                   |
| Dependencies | Bare and external identifiers, uniqueness, self-reference, resolution, and target-name ordering are validated                                              | External owner/repository case is preserved; repository names ending in `.git` are rejected             |

The integration tests in `tests/conformance.rs`, `tests/map.rs`, `tests/doctor.rs`, and
`tests/catalog.rs` are the executable contract. Python captures are migration evidence, not golden
output fixtures.

## Development

The monorepo selects nightly Rust with the minimal profile plus `clippy` and `rustfmt`.

```sh
cargo test -p ai-skillet --locked
just rust-check
```

From the monorepo root, `just install-cli` installs all four workspace binaries under `~/.local`.

## License

MIT. See [LICENSE.md](../LICENSE.md).

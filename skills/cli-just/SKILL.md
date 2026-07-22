---
disable-model-invocation: false
name: cli-just
user-invocable: false
description:
  "Use for just/justfile task automation: create justfiles, write recipes, configure settings, add modules/attributes,
  or set up command-runner workflows."
---

# Just Command Runner

Create readable task automation that matches the repository's installed Just version and existing justfile conventions.

## Defaults

- Inspect `just --version`, the existing justfile/module tree, and `just --list` before editing. Installed capabilities
  and existing syntax are authoritative over this skill's examples.
- Preserve bespoke formatting. Do not rewrite a justfile with `just --fmt`; use `just --fmt --check` only when the
  repository explicitly treats the built-in formatter as authoritative. `just --dump` is inspection output, not a
  formatting source.
- Prefer explicit, small recipes; `require()` for tool dependencies; private helpers; check/write recipe pairs; and
  aliases after their target recipes.
- Make recipes quiet by default: prefix the recipe name with `@` unless echoing commands has clear value. Recipe-level
  `@` inverts per-line `@`, so never also prefix lines inside a `@recipe`. Never add `@` to a `[script]` recipe: scripts
  are already quiet, and `@` un-mutes them. See "Quiet Recipes and Command Prefixes" in `references/recipes.md`.
- On stock macOS, assume `/bin/bash` 3.2 unless the recipe explicitly selects a newer shell.
- Use the user's section-banner style when creating a new standalone justfile; existing repository organization
  overrides it.

## Workflow

1. Determine the requested recipe behavior, inputs, outputs, cwd, shell, environment, dependencies, and success
   condition.

2. Use the installed manual (`just --help`, `just --man`, or <https://just.systems/man/en/>) for version-sensitive
   syntax. Read only the task-specific reference:

   | Task                                       | Reference                      |
   | ------------------------------------------ | ------------------------------ |
   | Recipes, parameters, dependencies, cache   | `references/recipes.md`        |
   | Settings, dotenv, lists, modules           | `references/settings.md`       |
   | Expressions, functions, constants          | `references/syntax.md`         |
   | Shell or script recipes                    | `references/inline-scripts.md` |
   | Check/write, status, aliases, organization | `references/patterns.md`       |

3. Make the smallest recipe or setting change. Do not enable unstable features unless the requested design needs them
   and the installed version supports them.

4. Validate parsing with `just --list` or `just --summary`, then execute the narrowest safe recipe path. For
   state-changing recipes, use an existing dry-run/check mode or inspect the expanded command first.

## Opinionated Organization

For a new standalone file, prefer dependency declarations, constants, public recipes, checks, then private helpers. Use
`set default-list := true` when no single default action makes sense and the installed version supports it. Example
starting points live in `examples/standalone.just` and `examples/devkit.just`; load one only when creating that shape.

Completion requires a parsable justfile, the requested recipe behavior, and execution or dry-run evidence appropriate to
its side effects. Finish with `### ✅ Just workflow ready`, a compact recipe/alias/purpose table when several entries
changed, and `### 🧪 Verification` with exact commands and outcomes. Keep confirmation prompts, recipe data output,
compiler output, and `just --list` output undecorated; preserve the repository's own banner vocabulary.

# Just Settings & Modules Reference

Configuration and module system for Just command runner.

## Settings

Settings configure global behavior and must appear at the top of the justfile.

### Boolean Settings

Enable with `set NAME` or `set NAME := true`:

| Setting                     | Description                                                                       |
| --------------------------- | --------------------------------------------------------------------------------- |
| `allow-duplicate-recipes`   | Allow later recipes to override earlier ones                                      |
| `allow-duplicate-variables` | Allow later variables to override earlier ones                                    |
| `default-list`              | List recipes instead of running the default recipe (v1.52.0+)                     |
| `default-script`            | Treat unannotated recipes as script recipes (v1.52.0+)                            |
| `dotenv-load`               | Load `.env` file automatically                                                    |
| `dotenv-required`           | Error if `.env` file is missing                                                   |
| `export`                    | Export all variables as environment variables                                     |
| `fallback`                  | Search parent directories for justfile                                            |
| `ignore-comments`           | Don't print comments in recipe listings                                           |
| `lazy`                      | Defer evaluation of unused variables (v1.47.0; stable v1.48.0+)                   |
| `lists`                     | Enable list-of-strings values (unstable, requires `set unstable`; v1.53.0+)       |
| `no-cd`                     | Don't change to justfile directory for any recipe (v1.51.0+)                      |
| `positional-arguments`      | Pass recipe arguments as $1, $2, etc.                                             |
| `quiet`                     | Don't echo recipe lines                                                           |
| `unstable`                  | Enable unstable features (user-defined functions, `eager` keyword, `lists`, etc.) |

### Value Settings

| Setting              | Example                              | Description                                                              |
| -------------------- | ------------------------------------ | ------------------------------------------------------------------------ |
| `shell`              | `["bash", "-euo", "pipefail", "-c"]` | Shell and arguments for linewise recipes                                 |
| `script-interpreter` | `["bash", "-euo", "pipefail"]`       | Interpreter for empty `[script]` recipes                                 |
| `dotenv-filename`    | `".env.local"`                       | Custom dotenv filename; accepts a list to load multiple files (v1.53.0+) |
| `dotenv-path`        | `"config/.env"`                      | Custom dotenv path; accepts a list to load multiple files (v1.53.0+)     |
| `tempdir`            | `"/tmp/just"`                        | Temporary file directory                                                 |
| `working-directory`  | `"src"`                              | Default working directory                                                |

**Const expressions in settings (v1.46.0+):**

All settings now accept const expressions:

```just
project_name := "myapp"
src_dir := "src"

set working-directory := src_dir
set dotenv-filename := project_name + ".env"
```

### Recommended Settings

```just
set allow-duplicate-recipes
set allow-duplicate-variables
set shell := ["bash", "-euo", "pipefail", "-c"]
set unstable
```

**Shell flags explained:**

- `-e`: Exit immediately on error
- `-u`: Treat unset variables as errors
- `-o pipefail`: Pipeline fails if any command fails
- `-c`: Execute following string as command

Note: `bash` here resolves via `PATH`. In agent sandboxes and stock macOS environments this can be `/bin/bash` 3.2, even when Homebrew Bash exists at `/opt/homebrew/bin/bash`; see [inline-scripts.md > Bash Version Pitfalls](inline-scripts.md#bash-version-pitfalls-macos) before relying on Bash-4+ features in recipe bodies.

### Lazy Evaluation (v1.47.0; stable v1.48.0+)

`set lazy` skips evaluating any variable that no executed recipe references. Useful when top-level assignments invoke
shell commands or remote calls that are only needed by some recipes.

```just
set lazy

# Only evaluated when a recipe actually uses `token`.
token := `gh auth token`

deploy:
    curl -H "Authorization: Bearer {{ token }}" https://example.com/deploy

clean:
    rm -rf dist/   # `token` never evaluated here.
```

Assignments marked `export` (or living in a module with `set export`) are always evaluated, even under `lazy`.

To force evaluation of a normally-unused assignment under `lazy`, prefix it with `eager` (requires `set unstable`):

```just
set unstable
set lazy

eager schema_version := `cat schema/VERSION`   # Evaluated even if no recipe uses it.
```

### Default Listing (v1.52.0+)

Use `default-list` when no recipe should be the default entrypoint:

```just
set default-list := true

build:
    cargo build

test:
    cargo test
```

With this setting, bare `just` lists recipes instead of running the `[default]` recipe or first recipe. The setting is
per-module, so invoking a module path with `default-list` enabled lists that module's recipes.

Runtime alternatives:

```bash
JUST_DEFAULT_LIST=true just
just --default-list
just --list foo::bar
```

### Default Script Recipes (v1.52.0+)

Use `default-script` only in script-heavy justfiles where most recipes should run as whole-file scripts rather than as
linewise shell commands:

```just
set default-script := true
set script-interpreter := ["bash", "-euo", "pipefail"]

deploy:
    trap 'echo failed' ERR
    npm run build
    npm publish

[shell]
status:
    echo "linewise shell recipe"
```

`set shell` still controls linewise shell recipes and backticks. `set script-interpreter` controls `[script]` recipes with
no explicit command, including unannotated recipes when `default-script` is enabled.

### Lists (unstable, v1.53.0+)

`set lists` introduces a list-of-strings value type. It is **unstable and will change in backwards-incompatible ways** —
gate it behind `set unstable` and track [the `set lists` issue](https://github.com/casey/just/issues/3377). Enabling it
changes several behaviors:

```just
set unstable
set lists

targets := ["x86", "arm"]            # List literal; literals flatten and hold only strings:
                                     #   [["a", "b"], [], "c"] == ["a", "b", "c"]
all := targets ++ ["wasm"]           # `++` concatenates lists
prefixed := "build-" + targets       # `+`/`/` broadcast a string across each element
pairs := ["a", "b"] / ["1", "2"]     # equal-length lists combine pairwise → ["a/1", "b/2"]
parts := split("a.ts b.ts")          # ["a.ts", "b.ts"]; separator defaults to whitespace (trimmed)
```

For file/path collections, use list literals:

```just
root_ox_paths := [
    "package.json",
    ".lintstagedrc.mjs",
    ".mcp.json",
    "biome.jsonc",
    "knip.jsonc",
    "oxlint.config.ts",
    "oxfmt.config.ts",
    "tsconfig.base.json",
    "vitest.shared.ts",
]
```

Do not build path sets with parenthesized, space-joined string assembly.

**Recipes & dependencies:**

- Variadic parameters (`*args`, `+args`) are lists of strings instead of one space-joined string.
- A parameter evaluates to its default when given an empty list; passing `[]` to a non-`*` parameter without a default is an error.
- Map a dependency over a list with `*(recipe *arg)` — see [recipes.md](recipes.md#mapped-dependencies-over-lists-unstable-v1530).
- Lists in recipe and `f`-string interpolations are space-joined into a single string.

**Booleans (reformed under `set lists`):** canonical true is `"true"`; canonical false is the empty list `[]`. Every
other value is truthy, **including `''`**.

- `!expr` evaluates to `"true"` when `expr` is `[]`, else `[]`.
- `==`, `!=`, `=~`, `!~` work in any expression (not just `if`/`assert`) and evaluate to `"true"` or `[]`. `==`/`!=` check structural equality.
- `value =~ regexes` is true if any element of `value` matches any regex in `regexes` (false if either is empty); `!~` is the negation.
- An `if` with no `else` evaluates to `[]` when its condition is false.

**Functions that accept lists:** `quote()`, `append()`, `prepend()`, `absolute_path()` map over each element; `env()`
and `[env]` take a list of keys / set the joined value (`[]` unsets the variable); `which()` now **requires** `set lists`
and returns `[]` when not found; `is_dependency()`, `path_exists()`, `semver_matches()` return the canonical booleans.

**New functions:**

| Function            | Behavior                                                                                          |
| ------------------- | ------------------------------------------------------------------------------------------------- |
| `split(s, sep)`     | Split `s` into a list on `sep`; default splits on whitespace with ends trimmed                    |
| `join_list(v, sep)` | Join list `v` into one string (default separator is a single space) — bridge to un-upgraded funcs |
| `bool(v)`           | `[]` for `""`/`"0"`/`"false"`/`[]`, `"true"` for `"1"`/`"true"`; any other value errors           |
| `show(v)`           | Literal representation, e.g. `"[]"`, `["foo", "bar"]`; single-element lists render as the element |

**Caveat:** using a list where a string is expected is an error — reach for `join_list()` (or interpolation) to bridge to
functions not yet list-aware.

**Multiple `.env` files:** under `set lists`, `dotenv-path` and `dotenv-filename` accept lists (and the matching
`--dotenv-path`/`--dotenv-filename` flags may be repeated). `dotenv-path` values are tried first; otherwise the
`dotenv-filename` names are searched in the current directory and its ancestors. When several files load, later entries
override earlier ones.

## Modules & Imports

### Imports

Include another justfile's contents directly:

```just
# Required import (error if missing)
import "path/to/file.just"
import "./just/settings.just"

# Optional import (no error if missing)
import? "local-overrides.just"
```

**Import behavior:**

- Imported recipes and variables merge into current namespace
- Later definitions override earlier ones (with `allow-duplicate-*`)
- Relative paths resolve from importing file's directory
- Duplicate imports are deduplicated automatically

### Modules

Load justfile as a submodule (requires `set unstable`):

```just
# Load from foo.just or foo/justfile
mod foo

# Load from custom path
mod bar "path/to/bar.just"
mod baz "other/directory"  # Looks for justfile inside

# Optional module (no error if missing)
mod? local

# Module with attributes
[private]
mod internal

[doc("Development tools")]
mod dev
```

As of v1.52.0, aliases and recipes that depend on an absent optional module are disabled with clearer errors instead of
failing unrelated listings or invocations.

**Calling module recipes:**

```just
# Subcommand syntax
just foo build

# Path syntax
just foo::build

# From another recipe
@all:
    just foo::build
    just bar::test
```

**Module namespacing:**

- Recipes inside modules are namespaced: `module::recipe`
- Variables inside modules are NOT accessible from parent
- Settings inside modules apply only to that module
- Modules can import/include other files

**Overriding module variables (v1.48.0+):**

Override `:=` assignments inside submodules from the command line with a `::`-separated path:

```bash
just foo::log_level=debug foo::run
just --set foo::log_level debug foo::run
```

Either form works; both target the submodule's own variable, not the parent's.

### Module Search Paths

When using `mod foo`:

1. `foo.just` in same directory
2. `foo/justfile` subdirectory
3. `foo/mod.just` subdirectory

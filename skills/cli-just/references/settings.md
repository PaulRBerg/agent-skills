# Just Settings & Modules Reference

Configuration and module system for Just command runner.

## Settings

Settings configure global behavior and must appear at the top of the justfile.

### Boolean Settings

Enable with `set NAME` or `set NAME := true`:

| Setting                     | Description                                                              |
| --------------------------- | ------------------------------------------------------------------------ |
| `allow-duplicate-recipes`   | Allow later recipes to override earlier ones                             |
| `allow-duplicate-variables` | Allow later variables to override earlier ones                           |
| `default-list`              | List recipes instead of running the default recipe (v1.52.0+)            |
| `default-script`            | Treat unannotated recipes as script recipes (v1.52.0+)                   |
| `dotenv-load`               | Load `.env` file automatically                                           |
| `dotenv-required`           | Error if `.env` file is missing                                          |
| `export`                    | Export all variables as environment variables                            |
| `fallback`                  | Search parent directories for justfile                                   |
| `ignore-comments`           | Don't print comments in recipe listings                                  |
| `lazy`                      | Defer evaluation of unused variables (v1.47.0; stable v1.48.0+)          |
| `no-cd`                     | Don't change to justfile directory for any recipe (v1.51.0+)             |
| `positional-arguments`      | Pass recipe arguments as $1, $2, etc.                                    |
| `quiet`                     | Don't echo recipe lines                                                  |
| `unstable`                  | Enable unstable features (user-defined functions, `eager` keyword, etc.) |
| `windows-powershell`        | Use PowerShell on Windows                                                |
| `windows-shell`             | Use cmd.exe on Windows                                                   |

### Value Settings

| Setting              | Example                              | Description                              |
| -------------------- | ------------------------------------ | ---------------------------------------- |
| `shell`              | `["bash", "-euo", "pipefail", "-c"]` | Shell and arguments for linewise recipes |
| `script-interpreter` | `["bash", "-euo", "pipefail"]`       | Interpreter for empty `[script]` recipes |
| `dotenv-filename`    | `".env.local"`                       | Custom dotenv filename                   |
| `dotenv-path`        | `"config/.env"`                      | Custom dotenv path                       |
| `tempdir`            | `"/tmp/just"`                        | Temporary file directory                 |
| `working-directory`  | `"src"`                              | Default working directory                |

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

Note: `bash` here resolves via `PATH` — `/bin/bash` 3.2 on stock macOS. See [inline-scripts.md > Bash Version Pitfalls](inline-scripts.md#bash-version-pitfalls-macos) before relying on Bash-4+ features in recipe bodies.

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

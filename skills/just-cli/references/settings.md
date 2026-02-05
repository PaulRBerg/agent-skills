# Just Settings & Modules Reference

Configuration and module system for Just command runner.

## Settings

Settings configure global behavior and must appear at the top of the justfile.

### Boolean Settings

Enable with `set NAME` or `set NAME := true`:

| Setting                     | Description                                          |
| --------------------------- | ---------------------------------------------------- |
| `allow-duplicate-recipes`   | Allow later recipes to override earlier ones         |
| `allow-duplicate-variables` | Allow later variables to override earlier ones       |
| `dotenv-load`               | Load `.env` file automatically                       |
| `dotenv-required`           | Error if `.env` file is missing                      |
| `export`                    | Export all variables as environment variables        |
| `fallback`                  | Search parent directories for justfile               |
| `ignore-comments`           | Don't print comments in recipe listings              |
| `positional-arguments`      | Pass recipe arguments as $1, $2, etc.                |
| `quiet`                     | Don't echo recipe lines                              |
| `unstable`                  | Enable unstable features (modules, script attribute) |
| `windows-powershell`        | Use PowerShell on Windows                            |
| `windows-shell`             | Use cmd.exe on Windows                               |

### Value Settings

| Setting             | Example                              | Description                     |
| ------------------- | ------------------------------------ | ------------------------------- |
| `shell`             | `["bash", "-euo", "pipefail", "-c"]` | Shell and arguments for recipes |
| `dotenv-filename`   | `".env.local"`                       | Custom dotenv filename          |
| `dotenv-path`       | `"config/.env"`                      | Custom dotenv path              |
| `tempdir`           | `"/tmp/just"`                        | Temporary file directory        |
| `working-directory` | `"src"`                              | Default working directory       |

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

### Module Search Paths

When using `mod foo`:

1. `foo.just` in same directory
2. `foo/justfile` subdirectory
3. `foo/mod.just` subdirectory

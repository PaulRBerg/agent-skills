# Just Syntax Reference

Language syntax and utilities for Just command runner.

## Constants

### Terminal Colors

Available globally without definition:

| Constant  | ANSI Code |
| --------- | --------- |
| `BLACK`   | `\e[30m`  |
| `RED`     | `\e[31m`  |
| `GREEN`   | `\e[32m`  |
| `YELLOW`  | `\e[33m`  |
| `BLUE`    | `\e[34m`  |
| `MAGENTA` | `\e[35m`  |
| `CYAN`    | `\e[36m`  |
| `WHITE`   | `\e[37m`  |

### Text Styles

| Constant        | Effect             |
| --------------- | ------------------ |
| `BOLD`          | Bold text          |
| `ITALIC`        | Italic text        |
| `UNDERLINE`     | Underlined text    |
| `STRIKETHROUGH` | Strikethrough text |
| `INVERT`        | Invert colors      |
| `HIDE`          | Hidden text        |

### Reset

| Constant | Effect               |
| -------- | -------------------- |
| `NORMAL` | Reset all formatting |

### Background Colors

| Constant     | Description        |
| ------------ | ------------------ |
| `BG_BLACK`   | Black background   |
| `BG_RED`     | Red background     |
| `BG_GREEN`   | Green background   |
| `BG_YELLOW`  | Yellow background  |
| `BG_BLUE`    | Blue background    |
| `BG_MAGENTA` | Magenta background |
| `BG_CYAN`    | Cyan background    |
| `BG_WHITE`   | White background   |

### System Constants

| Constant   | Value                       |
| ---------- | --------------------------- |
| `HEX`      | `0123456789ABCDEF`          |
| `HEXLOWER` | `0123456789abcdef`          |
| `PATH_SEP` | `:` (Unix) or `;` (Windows) |

### Usage Examples

```just
@success:
    echo -e '{{ GREEN }}✓ Success!{{ NORMAL }}'

@error:
    echo -e '{{ RED + BOLD }}✗ Error!{{ NORMAL }}'

@highlight:
    echo -e '{{ BG_YELLOW + BLACK }}Warning{{ NORMAL }}'

@combined:
    echo -e '{{ BOLD + UNDERLINE + CYAN }}Important{{ NORMAL }}'
```

## Functions

### Executable Functions

```just
# Require executable (fail if not found)
jq := require("jq")
# Returns full path: /usr/bin/jq

# Usage: invoke directly in recipes (not with interpolation)
process:
    jq '.name' package.json

# Check if executable exists
has_docker := `which docker > /dev/null 2>&1 && echo "true" || echo "false"`
```

**Note:** `require()` validates the tool exists and stores its path. Use the variable name directly (e.g., `jq`), not with interpolation (`{{ jq }}`).

### Environment Functions

```just
# Get env var (error if unset)
home := env("HOME")

# Get env var with default
log_level := env("LOG_LEVEL", "info")

# Export variable
export DATABASE_URL := env("DATABASE_URL", "postgres://localhost/dev")
```

### Path Functions

```just
# Justfile directory (absolute path)
root := justfile_dir()

# Justfile path
justfile := justfile()

# Source directory (for imported files)
source_dir := source_directory()
source_file := source_file()

# Invocation directory (where just was called from)
invocation_dir := invocation_directory()

# Module location (v1.49.0–1.50.0; meaningful inside `mod` files)
mod_file := module_file()         # Absolute path to the module's justfile
mod_dir := module_directory()     # Directory containing the module justfile
mod_path := module_path()         # Submodule path, e.g. "foo::bar"

# Runtime directory (v1.49.0; $XDG_RUNTIME_DIR or platform fallback)
rt := runtime_directory()

# Parent directory
parent := parent_directory(justfile_dir())

# Join paths
config := join(justfile_dir(), "config")

# File operations
exists := path_exists("config.json")
stem := file_stem("config.json")      # "config"
name := file_name("path/config.json") # "config.json"
ext := extension("config.json")       # "json"
```

### String Functions

```just
# Case conversion
upper := uppercase("hello")      # "HELLO"
lower := lowercase("HELLO")      # "hello"
kebab := kebabcase("HelloWorld") # "hello-world"
snake := snakecase("HelloWorld") # "hello_world"
title := titlecase("hello")      # "Hello"

# String manipulation
trimmed := trim("  hello  ")     # "hello"
replaced := replace("foo-bar", "-", "_")  # "foo_bar"

# Quoting
quoted := quote("path with spaces")  # "'path with spaces'"
shell_escaped := shell("echo 'test'")
```

### System Functions

```just
# Operating system
os := os()              # "linux", "macos", "windows"
family := os_family()   # "unix" or "windows"
arch := arch()          # "x86_64", "aarch64", etc.

# Number of CPUs
cpus := num_cpus()

# UUID generation
id := uuid()

# SHA256 hash
hash := sha256("content")
file_hash := sha256_file("config.json")

# Date/time
now := datetime("%Y-%m-%d")
timestamp := datetime("%s")
```

### Conditional Functions

```just
# Error if condition false
_ := assert(path_exists("config.json"), "Config file required!")

# Conditional value
mode := if env("CI", "") != "" { "ci" } else { "local" }

# Error message
_ := error("This recipe is deprecated")
```

## User-Defined Functions (v1.49.0+)

Define reusable named expressions with `name(args) := expression`. Requires `set unstable`. Functions live in the same
namespace as variables and may reference any module-level assignment.

```just
set unstable

base := "src"
join(extension) := base + "/main." + extension

# f-strings for interpolation
greet(name) := f"Hello, {{ name }}!"

# Compose with other functions and conditionals
asset(name) := if path_exists(join("ts")) { name + ".ts" } else { name + ".js" }

build:
    cp {{ join("ts") }} dist/
    echo '{{ greet("world") }}'
```

**When to use:**

- Replace repeated expression fragments across recipes.
- Prefer over backtick-evaluated variables when the value depends on a parameter (backticks evaluate once at
  justfile parse time; functions evaluate per call site).
- Function bodies are pure expressions — no shell, no side effects. For shell logic, use a `[script]` recipe.

## Variables

### Assignment

```just
# Simple
name := "value"

# From environment
port := env("PORT", "3000")

# From shell command
version := `git describe --tags`

# Exported (available to recipes)
export NODE_ENV := "production"

# Conditional
mode := if os() == "windows" { "win" } else { "unix" }
```

### Variable Scope

- Variables defined at top level are global
- Recipe parameters shadow global variables
- Imported variables can be overridden with `allow-duplicate-variables`

## Backtick Evaluation

````just
# Single line
version := `git describe --tags`

# Multi-line (indented)
files := ```
    find src -name "*.ts" \
        | grep -v test \
        | head -10
```
````

## Just CLI Options

| Option                       | Description                                                              |
| ---------------------------- | ------------------------------------------------------------------------ |
| `just --list`                | List available recipes                                                   |
| `just --list --unsorted`     | List in source order                                                     |
| `just --list --group NAME`   | Filter `--list` to one group (v1.47.0+)                                  |
| `just --summary`             | Brief recipe list                                                        |
| `just --show RECIPE`         | Show recipe source                                                       |
| `just --usage RECIPE`        | Show recipe argument usage (v1.46.0+)                                    |
| `just --dry-run RECIPE`      | Print commands without running                                           |
| `just --time RECIPE`         | Print recipe execution time (v1.49.0+)                                   |
| `just --evaluate`            | Print all variables                                                      |
| `just --evaluate-format FMT` | Format `--evaluate` output (`json`, `shell`; v1.49.0+)                   |
| `just --evaluate VAR`        | Evaluate a single variable/module path (v1.49.0+)                        |
| `just --json`                | Dump justfile metadata as JSON (v1.48.0+; alias of `--dump-format json`) |
| `just --fmt`                 | Format justfile (do not use — see SKILL.md)                              |
| `just --fmt --check`         | Check formatting                                                         |
| `just --choose`              | Interactive recipe selection (fzf)                                       |
| `just --choose --group NAME` | Restrict `--choose` to one group (v1.50.0+)                              |
| `just -f PATH`               | Use specific justfile (`-f -` reads stdin as of v1.51.0)                 |
| `just --justfile-name NAME`  | Override justfile filename for auto-discovery (v1.49.0+)                 |
| `just -d DIR`                | Set working directory                                                    |
| `just --indentation STR`     | Use STR for recipe indentation when formatting (v1.49.0+)                |

## Glob Patterns

Store glob patterns in variables with proper quoting:

```just
# Quote the pattern
GLOBS_TS := "\"**/*.{ts,tsx}\""
GLOBS_JSON := "\"**/*.{json,jsonc,yaml,yml}\""

# Use in recipes
lint:
    eslint {{ GLOBS_TS }}

format:
    prettier --check {{ GLOBS_JSON }}
```

## Error Handling

```just
# Fail on any error (in script block)
[script("bash")]
deploy:
    set -e
    npm run build
    npm run test
    npm publish

# Continue on error (line prefix)
cleanup:
    -rm -rf dist/
    -rm -rf node_modules/
    echo "Cleanup attempted"

# Assert condition
check:
    {{ assert(path_exists("package.json"), "Must run from project root") }}
```

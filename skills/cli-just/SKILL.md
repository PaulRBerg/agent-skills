---
disable-model-invocation: false
name: cli-just
user-invocable: false
description: 'Use for just/justfile task automation: create justfiles, write recipes, configure settings, add modules/attributes, or set up command-runner workflows.'
---

# Just Command Runner

## Overview

Expert guidance for Just, a command runner with syntax inspired by make. Use this skill for creating justfiles, writing recipes, configuring settings, and implementing task automation workflows.

**Targets just v1.55.0** (latest stable, released 2026-06-29). Features are tagged with the version that introduced them (e.g. `v1.51.0+`); a tag newer than your installed `just --version` means you must upgrade to use it. List features (`set lists`) and other gated capabilities additionally require `set unstable`.

**Key capabilities:**

- Create and organize justfiles with proper structure
- Write recipes with attributes, dependencies, and parameters
- Configure settings for shell, modules, and imports
- Use built-in constants for terminal formatting
- Implement check/write patterns for code quality tools

## No Justfile Formatter

Do not use `just --fmt` or `just --dump`. The user has bespoke formatting preferences that the built-in formatter does not respect. Preserve existing formatting as-is.

## Quick Reference

### Essential Settings

```just
set minimum-version := '1.55.0'   # Error out on older just (v1.55.0+); place at top
set allow-duplicate-recipes       # Allow recipes to override imported ones
set allow-duplicate-variables     # Allow variables to override imported ones
set shell := ["bash", "-euo", "pipefail", "-c"]  # Strict bash with error handling
set unstable                      # Enable unstable features (user-defined functions, eager keyword)
set dotenv-load                   # Auto-load .env file
set dotenv-command := 'sops -d .enc.env'  # Load env from command stdout (v1.54.0+)
set positional-arguments          # Pass recipe args as $1, $2, etc.
set lazy                          # Defer evaluation of unused variables (v1.47.0; stable v1.48.0+)
set no-cd                         # Don't change to justfile directory for any recipe (v1.51.0+)
set default-list := true          # Bare `just` lists recipes instead of running default (v1.52.0+)
set default-script := true        # Make unannotated recipes script recipes; use sparingly (v1.52.0+)
set lists                         # Enable list-of-strings values; unstable, requires `set unstable` (v1.53.0+)
```

### Common Attributes

| Attribute                    | Purpose                                                                     |
| ---------------------------- | --------------------------------------------------------------------------- |
| `[arg("p", long, ...)]`      | Configure parameter as `--flag` option (v1.46)                              |
| `[arg("p", short)]`          | Short option; bare `short` ⇒ first char of name (v1.55+)                    |
| `[arg("p", long, flag)]`     | Valueless flag ⇒ `"true"`/`[]`; needs `set lists` (v1.53+)                  |
| `[arg("p", multiple)]`       | Repeatable option ⇒ list; needs `set lists` (v1.55+)                        |
| `[arg("p", pattern="…")]`    | Constrain parameter to match regex pattern                                  |
| `[cache(inputs=, outputs=)]` | Cache script-recipe runs; unstable, script-only (v1.54+)                    |
| `[confirm("prompt")]`        | Require user confirmation (expressions OK as of v1.49)                      |
| `[continue]`                 | Continue if child exits OK after a caught signal (default `SIGINT`; v1.54+) |
| `[doc("text")]`              | Override recipe documentation                                               |
| `[env("NAME", "VALUE")]`     | Set env var for this recipe only (v1.47+, expr v1.51)                       |
| `[group("name")]`            | Group recipes in `just --list` output                                       |
| `[macos]`                    | Restrict a recipe to macOS                                                  |
| `[no-cd]`                    | Don't change to justfile directory                                          |
| `[parallel]`                 | Run direct dependencies concurrently                                        |
| `[positional-arguments]`     | Enable positional args for this recipe only                                 |
| `[private]`                  | Hide from `just --list` (same as `_` prefix)                                |
| `[script]`                   | Execute recipe as single script block                                       |
| `[script("interpreter")]`    | Use specific interpreter (bash, python, etc.)                               |
| `[shell]`                    | Force linewise shell mode under `set default-script` (v1.52+)               |
| `[working-directory: "…"]`   | Run from given path (expressions OK as of v1.51)                            |

When configuring recipe parameters as CLI-style `--flag`/`-x` options, read [Recipe Argument Flags](references/recipes.md#recipe-argument-flags-v1460) before using `[arg()]`.

Multiple attributes can be combined:

```just
[no-cd, private]
[group("checks")]
recipe:
    echo "hello"
```

### Built-in Constants

Terminal formatting constants are globally available (no definition needed):

| Constant                                            | Description                                |
| --------------------------------------------------- | ------------------------------------------ |
| `CYAN`, `GREEN`, `RED`, `YELLOW`, `BLUE`, `MAGENTA` | Text colors                                |
| `BOLD`, `ITALIC`, `UNDERLINE`, `STRIKETHROUGH`      | Text styles                                |
| `NORMAL`                                            | Reset formatting                           |
| `BG_*`                                              | Background colors (BG_RED, BG_GREEN, etc.) |
| `HEX`, `HEXLOWER`, `HEXUPPER`                       | Hexadecimal digits                         |

Usage:

```just
@status:
    echo -e '{{ GREEN }}Success!{{ NORMAL }}'
    echo -e '{{ BOLD + CYAN }}Building...{{ NORMAL }}'
```

For dynamic styling — named/256/24-bit colors, `fg:`/`bg:` variants, and `stdout`/`stderr` stream gates — use the [`style()` function](references/syntax.md#style-function) (v1.55.0+) instead of these fixed constants.

### Key Functions

```just
# Require executable exists (fails recipe if not found)
jq := require("jq")

# Get environment variable with default
log_level := env("LOG_LEVEL", "info")

# just version string (v1.55.0+)
ver := just_version()                # e.g. "1.55.0"

# Get justfile directory path
root := justfile_dir()

# Module location (useful inside `mod` files)
mod_path := module_path()            # Full submodule path, e.g. "foo::bar"
mod_file := module_file()            # Absolute path to module's justfile
mod_dir := module_directory()        # Directory containing the module justfile

# Runtime directory (v1.49.0; typically $XDG_RUNTIME_DIR, falls back to tempdir)
rt := runtime_directory()

# Name of the currently-running recipe (v1.53.0+)
self:
    echo "running {{ recipe_name() }}"
```

When an expression repeats across recipes, read [User-Defined Functions](references/syntax.md#user-defined-functions-v1490) before adding a `name(args) := expression` (requires `set unstable`).

When a variable needs to hold a list of strings rather than one joined string, read [Lists](references/settings.md#lists-unstable-v1530) before enabling `set lists` (unstable, requires `set unstable`) — it also reforms how booleans work. For mapping a dependency over a list, see [references/recipes.md](references/recipes.md#mapped-dependencies-over-lists-unstable-v1530).

## Recipe Patterns

When designing recipes that use status reporting, check/write semantics, or alias conventions, see [references/patterns.md](references/patterns.md).

To skip re-running expensive script recipes, see Cached Recipes (`[cache]`, unstable, v1.54.0+) in [references/recipes.md](references/recipes.md#cached-recipes-unstable-v1540).

## Inline Scripts

When writing recipes that need shell scripts (script attribute or shebang style), see [references/inline-scripts.md](references/inline-scripts.md). On stock macOS, `bash` resolves to `/bin/bash` 3.2 — see that file's Bash Version Pitfalls section before using Bash-4+ features.

## Modules & Imports

### Import Pattern

Include recipes from another file:

```just
import "./just/settings.just"
import "./just/base.just"
import? "./local.just"    # Optional (no error if missing)
```

### Module Pattern

Load submodule (requires `set unstable`):

```just
mod foo                   # Loads foo.just or foo/justfile
mod bar "path/to/bar"     # Custom path
mod? optional             # Optional module

alias f := foo            # Alias a module (v1.55.0+); `just f build` runs `foo::build`

# Call module recipes
just foo::build
```

### Devkit Import Pattern

For projects using `@sablier/devkit`:

```just
import "./node_modules/@sablier/devkit/just/base.just"
import "./node_modules/@sablier/devkit/just/npm.just"
```

## Markdown Justfiles (v1.53.0+)

When `--justfile` points at a `.md` file, just extracts the contents of unindented ```` ```just ```` fenced code blocks and runs them as a justfile. Useful for keeping runnable recipes inside documentation:

````markdown
Build the project:

```just
build:
    echo Building…
```
````

```bash
just --justfile README.md build
```

`--fmt` prints the formatted justfile to stdout (rather than rewriting) when the source is a markdown file or stdin.

## Section Organization

Standard section header format:

```just
# ---------------------------------------------------------------------------- #
#                                 DEPENDENCIES                                 #
# ---------------------------------------------------------------------------- #
```

Common sections (in order):

1. **DEPENDENCIES** - Required tools with URLs
2. **CONSTANTS** - Glob patterns, environment vars
3. **RECIPES / COMMANDS** - Main entry points
4. **CHECKS** - Code quality recipes
5. **UTILITIES / INTERNAL HELPERS** - Private helpers

## Default Recipe

Define a curated default recipe when one action should be the entrypoint:

```just
# Run all checks by default
default: full-check
```

If no single default makes sense, prefer `set default-list := true` (v1.52.0+) over a `default` recipe that shells out to `just --list` — read [Default Listing](references/settings.md#default-listing-v1520) for the setting and its runtime alternatives.

For compatibility with older `just` versions, keep the explicit listing recipe:

```just
default:
    @just --list
```

## Dependencies Declaration

Document required tools at the top:

```just
# ---------------------------------------------------------------------------- #
#                                 DEPENDENCIES                                 #
# ---------------------------------------------------------------------------- #

# Bun: https://bun.sh
bun := require("bun")

# Ni: https://github.com/antfu-collective/ni
na := require("na")
ni := require("ni")
nlx := require("nlx")

# Usage: invoke directly in recipes (not with interpolation)
build:
    bun next build
```

See [Executable Functions](references/syntax.md#executable-functions) for how `require()` validates and resolves tools.

## Context7 Fallback

For Just features not covered in this skill (new attributes, advanced functions, edge cases), fetch the latest documentation:

```
Use context7 MCP with library ID `/websites/just_systems_man_en` to get up-to-date Just documentation.
```

Example topics to search:

- `modules import mod` - Module system details
- `settings` - All available settings
- `attributes` - Recipe attributes
- `functions` - Built-in functions
- `script recipes` - Script block syntax

## Additional Resources

### Example Templates

Working justfile templates in `examples/`:

- **[`devkit.just`](examples/devkit.just)** - Minimal template importing @sablier/devkit
- **[`standalone.just`](examples/standalone.just)** - Full standalone template with all patterns

### External Documentation

- **Official Manual**: https://just.systems/man/en/
- **GitHub Repository**: https://github.com/casey/just
- **Context7 Library ID**: `/websites/just_systems_man_en`

## Tips

1. Use `[no-cd]` in monorepos to stay in current directory.
2. Always define aliases after recipe names for discoverability.

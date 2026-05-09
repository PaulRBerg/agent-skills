---
disable-model-invocation: false
name: cli-just
user-invocable: false
description: This skill should be used when the user asks to "create a justfile", "write just recipes", "configure just settings", "add just modules", "use just attributes", "set up task automation", mentions justfile, just command runner, or task automation with just.
---

# Just Command Runner

## Overview

Expert guidance for Just, a command runner with syntax inspired by make. Use this skill for creating justfiles, writing recipes, configuring settings, and implementing task automation workflows.

**Key capabilities:**

- Create and organize justfiles with proper structure
- Write recipes with attributes, dependencies, and parameters
- Configure settings for shell, modules, and imports
- Use built-in constants for terminal formatting
- Implement check/write patterns for code quality tools

## Quick Reference

### Essential Settings

```just
set allow-duplicate-recipes       # Allow recipes to override imported ones
set allow-duplicate-variables     # Allow variables to override imported ones
set shell := ["bash", "-euo", "pipefail", "-c"]  # Strict bash with error handling
set unstable                      # Enable unstable features (modules, script attribute)
set dotenv-load                   # Auto-load .env file
set positional-arguments          # Pass recipe args as $1, $2, etc.
```

### Common Attributes

| Attribute                 | Purpose                                        |
| ------------------------- | ---------------------------------------------- |
| `[arg("p", long, ...)]`   | Configure parameter as `--flag` option (v1.46) |
| `[arg("p", pattern="…")]` | Constrain parameter to match regex pattern     |
| `[group("name")]`         | Group recipes in `just --list` output          |
| `[no-cd]`                 | Don't change to justfile directory             |
| `[parallel]`              | Run direct dependencies concurrently           |
| `[private]`               | Hide from `just --list` (same as `_` prefix)   |
| `[script]`                | Execute recipe as single script block          |
| `[script("interpreter")]` | Use specific interpreter (bash, python, etc.)  |
| `[confirm("prompt")]`     | Require user confirmation before running       |
| `[doc("text")]`           | Override recipe documentation                  |
| `[positional-arguments]`  | Enable positional args for this recipe only    |

### Recipe Argument Flags (v1.46.0+)

The `[arg()]` attribute configures parameters as CLI-style options:

```just
# Long option (--target)
[arg("target", long)]
build target:
    cargo build --target {{ target }}

# Short option (-v)
[arg("verbose", short="v")]
run verbose="false":
    echo "Verbose: {{ verbose }}"

# Combined long + short
[arg("output", long, short="o")]
compile output:
    gcc main.c -o {{ output }}

# Flag without value (presence sets to "true")
[arg("release", long, value="true")]
build release="false":
    cargo build {{ if release == "true" { "--release" } else { "" } }}

# Help string (shown in `just --usage`)
[arg("target", long, help="Build target architecture")]
build target:
    cargo build --target {{ target }}
```

**Usage examples:**

```bash
just build --target x86_64
just build --target=x86_64
just compile -o main
just build --release
just --usage build    # Show recipe argument help
```

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
| `HEX`, `HEXLOWER`                                   | Hexadecimal digits                         |

Usage:

```just
@status:
    echo -e '{{ GREEN }}Success!{{ NORMAL }}'
    echo -e '{{ BOLD + CYAN }}Building...{{ NORMAL }}'
```

### Key Functions

```just
# Require executable exists (fails recipe if not found)
jq := require("jq")

# Get environment variable with default
log_level := env("LOG_LEVEL", "info")

# Get justfile directory path
root := justfile_dir()
```

## Recipe Patterns

When designing recipes that use status reporting, check/write semantics, or alias conventions, see [references/patterns.md](references/patterns.md).

## Inline Scripts

When writing recipes that need shell scripts (script attribute or shebang style), see [references/inline-scripts.md](references/inline-scripts.md).

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

# Call module recipes
just foo::build
```

### Devkit Import Pattern

For projects using `@sablier/devkit`:

```just
import "./node_modules/@sablier/devkit/just/base.just"
import "./node_modules/@sablier/devkit/just/npm.just"
```

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

Always define a default recipe:

```just
# Show available commands
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

**Note:** `require()` validates the tool exists at recipe evaluation time. Use the variable name directly (e.g., `bun`), not with interpolation (`{{ bun }}`).

## Context7 Fallback

For Just features not covered in this skill (new attributes, advanced functions, edge cases), fetch the latest documentation:

```
Use context7 MCP with library ID `/websites/just_systems-man` to get up-to-date Just documentation.
```

Example topics to search:

- `modules import mod` - Module system details
- `settings` - All available settings
- `attributes` - Recipe attributes
- `functions` - Built-in functions
- `script recipes` - Script block syntax

## Additional Resources

### Reference Files

For detailed patterns and comprehensive coverage, consult:

- **[`references/settings.md`](references/settings.md)** - Settings configuration and module system
- **[`references/recipes.md`](references/recipes.md)** - Recipe attributes, parameters, dependencies, and prefixes
- **[`references/syntax.md`](references/syntax.md)** - Constants, functions, variables, and CLI options
- **[`references/patterns.md`](references/patterns.md)** - Established conventions, section organization, helper patterns

### Example Templates

Working justfile templates in `examples/`:

- **[`devkit.just`](examples/devkit.just)** - Minimal template importing @sablier/devkit
- **[`standalone.just`](examples/standalone.just)** - Full standalone template with all patterns

### External Documentation

- **Official Manual**: https://just.systems/man/en/
- **GitHub Repository**: https://github.com/casey/just
- **Context7 Library ID**: `/websites/just_systems-man`

## No Justfile Formatter

Do not use `just --fmt` or `just --dump`. The user has bespoke formatting preferences that the built-in formatter does not respect. Preserve existing formatting as-is.

## Tips

1. Use `@` prefix to suppress command echo: `@echo "quiet"`
2. Use `+` for variadic parameters: `test +args`
3. Use `*` for optional variadic: `build *flags`
4. Quote glob patterns in variables: `GLOBS := "\"**/*.json\""`
5. Use `[no-cd]` in monorepos to stay in current directory
6. Private recipes start with `_` or use `[private]`
7. Always define aliases after recipe names for discoverability

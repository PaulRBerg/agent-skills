# Justfile Patterns & Conventions

Established patterns and conventions for organizing justfiles, based on real-world usage across multiple projects.

## Section Headers

Use centered ASCII art headers to organize justfiles:

```just
# ---------------------------------------------------------------------------- #
#                                    SECTION                                   #
# ---------------------------------------------------------------------------- #
```

## Constants Section

### Glob Patterns

Quote glob patterns for shell expansion:

```just
# ---------------------------------------------------------------------------- #
#                                   CONSTANTS                                  #
# ---------------------------------------------------------------------------- #

GLOBS_PRETTIER := "\"**/*.{json,jsonc,yaml,yml}\""
GLOBS_SOLIDITY := "{scripts,src,tests}/**/*.sol"
GLOBS_CLEAN := "**/{.logs,bindings,build,generated}"
GLOBS_CLEAN_IGNORE := "!graph/common/bindings"
```

### Path Lists

When a command consumes multiple concrete paths, require `set unstable` and `set lists`, then use list literals:

```just
set unstable
set lists

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

oxlint-check:
    oxlint {{ root_ox_paths }}
```

Do not recommend parenthesized, space-joined string assembly for path sets.

### Environment Variables

```just
export LOG_LEVEL := env("LOG_LEVEL", "info")
export NODE_ENV := env("NODE_ENV", "development")
```

## Recipe Groups

Use `[group()]` attribute for organized `just --list` output:

```just
[group("checks")]     # Linting, formatting, type checking
[group("codegen")]    # Code generation
[group("test")]       # Testing
[group("cli")]        # CLI command helpers
[group("dev")]        # Development utilities
[group("deploy")]     # Deployment recipes
[group("print")]      # Debug/print utilities
```

**Multiple groups per recipe:**

```just
[group("codegen"), group("envio")]
codegen-envio:
    ./codegen.sh
```

## Alias Conventions

Define aliases immediately after recipe names for discoverability:

```just
# Run all code checks
[group("checks")]
@full-check:
    just _run-with-status biome-check
alias fc := full-check
```

### Example Aliases

| Recipe        | Alias | Purpose         |
| ------------- | ----- | --------------- |
| `full-check`  | `fc`  | Run all checks  |
| `full-write`  | `fw`  | Apply all fixes |
| `biome-check` | `bc`  | Biome check     |
| `biome-write` | `bw`  | Biome fix       |

## Helper Patterns

### Run-With-Status Pattern

Display formatted status during multi-step workflows:

```just
# Private recipe to run a check with formatted output
@_run-with-status recipe *args:
    echo ""
    echo -e '{{ CYAN }}→ Running {{ recipe }}...{{ NORMAL }}'
    just {{ recipe }} {{ args }}
    echo -e '{{ GREEN }}✓ {{ recipe }} completed{{ NORMAL }}'
alias rws := _run-with-status
```

**Usage in aggregate recipes:**

```just
[group("checks")]
@full-check:
    just _run-with-status biome-check
    just _run-with-status prettier-check
    just _run-with-status tsc-check
    echo ""
    echo -e '{{ GREEN }}All code checks passed!{{ NORMAL }}'
alias fc := full-check
```

## Default Recipe

Prefer a meaningful default action when a project has one:

```just
# Run all checks by default
default: full-check
```

When no single action is the obvious entrypoint, use `default-list` (v1.52.0+) instead of a wrapper recipe:

```just
set default-list := true
```

Keep the explicit listing recipe only when supporting older `just` versions:

```just
default:
    @just --list
```

## Monorepo Patterns

### Module Per Package

```just
mod client "apps/client"
mod server "apps/server"
mod shared "packages/shared"
```

### No-CD for Cross-Package Commands

```just
[no-cd]
build-all:
    just client::build
    just server::build
```

### Shared Recipes via Import

```just
# apps/client/justfile
import "../../just/shared.just"

build: shared-setup
    npm run build
```

## Script Block Patterns

### Multi-line Bash

```just
[script("bash")]
deploy chain_slug:
    set -e
    case {{ chain_slug }} in
        mainnet)
            DEPLOY_URL="https://prod.example.com"
            ;;
        testnet)
            DEPLOY_URL="https://test.example.com"
            ;;
        *)
            echo "Unknown chain: {{ chain_slug }}"
            exit 1
            ;;
    esac
    curl -X POST "$DEPLOY_URL/deploy"
```

### Conditional TUI Mode

```just
[script("bash")]
envio command tui_mode="tui_on" *args:
    set -a
    if [ "{{ tui_mode }}" = "tui_off" ]; then
        TUI_OFF=true
    fi
    pnpm envio {{ command }} {{ args }}
```

## Import Organization

### Settings First

```just
import "./just/settings.just"
import "./just/base.just"
import "./just/npm.just"
```

### Devkit Pattern

```just
# See https://github.com/sablier-labs/devkit/blob/main/just/base.just
import "./node_modules/@sablier/devkit/just/base.just"
import "./node_modules/@sablier/devkit/just/npm.just"
```

### Local Overrides Last

```just
import "./just/base.just"
import? "./just/local.just"  # Optional local overrides
```

## Attribute Combinations

### Private Group Recipe

```just
[group("internal")]
[private]
_helper:
    echo "helper"
```

### Confirmed No-CD Script

```just
[no-cd]
[script("bash")]
[confirm("Deploy to production?")]
deploy-prod:
    set -e
    npm run build
    aws s3 sync dist/ s3://prod-bucket/
```

### Documented Group Recipe

```just
[doc("Generate TypeScript types from GraphQL schema")]
[group("codegen")]
codegen-types:
    graphql-codegen
```

## Environment Variable Patterns

### Export with Default

```just
export LOG_LEVEL := env("LOG_LEVEL", "info")
export NODE_ENV := env("NODE_ENV", "development")
```

### Load from .env

```just
set dotenv-load

# Or specify path
set dotenv-path := ".env.local"
```

### Per-Recipe Environment

```just
test-integration:
    DATABASE_URL="postgres://localhost/test" npm run test:integration
```

## Error Handling

### Ignore Specific Errors

```just
cleanup:
    -rm -rf dist/
    -rm -rf coverage/
    echo "Cleanup complete"
```

### Assert Preconditions

```just
deploy:
    {{ assert(path_exists("dist/"), "Run 'just build' first") }}
    aws s3 sync dist/ s3://bucket/
```

### Graceful Fallbacks

```just
@version:
    git describe --tags 2>/dev/null || echo "v0.0.0-dev"
```

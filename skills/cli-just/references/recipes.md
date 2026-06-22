# Just Recipes Reference

Recipe definition and behavior for Just command runner.

## Attributes

Attributes modify recipe behavior. Place before recipe definition.

### Recipe Visibility

```just
# Private via underscore prefix
_helper:
    echo "private"

# Private via attribute
[private]
helper:
    echo "also private"
```

### Grouping

```just
[group("checks")]
lint:
    npm run lint

[group("checks")]
format:
    npm run format
```

Groups organize `just --list` output:

```
Available recipes:
    default

[checks]
    format
    lint
```

### Directory Control

```just
# Don't change to justfile directory
[no-cd]
status:
    git status

# Set specific working directory
[working-directory: "packages/core"]
build-core:
    npm run build

# Expression-valued working directory (v1.51.0+)
src := justfile_dir() / "src"

[working-directory: src]
generate:
    codegen
```

For a justfile-wide opt-out, use `set no-cd` (v1.51.0+) instead of annotating every recipe.

### OS-Restricted Recipes

Restrict a recipe to macOS when the command intentionally uses macOS tools.

```just
[macos]
open url:
    open {{ url }}
```

This catalog is macOS-first; omit non-Mac OS guards unless the project itself must publish a portable justfile.

### Script Blocks

```just
# Default shell script
[script]
multiline:
    if [ -f "config.json" ]; then
        echo "Found config"
    else
        echo "No config"
    fi

# Specific interpreter
[script("python3")]
process:
    import json
    data = json.load(open("config.json"))
    print(data["name"])

[script("bash")]
deploy:
    set -e
    npm run build
    aws s3 sync dist/ s3://bucket/

[script("node")]
analyze:
    const fs = require('fs');
    const pkg = JSON.parse(fs.readFileSync('package.json'));
    console.log(`Package: ${pkg.name}@${pkg.version}`);
```

With `set default-script := true` (v1.52.0+), unannotated recipes are script recipes by default. Add `[shell]` to force
normal linewise shell execution for a specific recipe:

```just
set default-script := true

[shell]
quick:
    echo "run this as a shell line, not a temp script"
```

### Confirmation

```just
# Default confirmation prompt
[confirm]
delete-all:
    rm -rf dist/

# Custom prompt
[confirm("Are you sure you want to deploy to production?")]
deploy-prod:
    ./deploy.sh production

# Expression-valued prompt (v1.49.0+): skip the prompt in CI
[confirm(if env("CI", "") == "true" { "" } else { "Deploy to prod?" })]
deploy:
    ./deploy.sh
```

### Per-Recipe Environment Variables

The `[env(NAME, VALUE)]` attribute (v1.47.0+) sets environment variables scoped to one recipe — narrower than `export` (whole justfile) or `set dotenv-load` (everything).

```just
[env("RUST_BACKTRACE", "1")]
test:
    cargo test                # RUST_BACKTRACE=1 is set only for `test`

# Multiple env attributes stack
[env("NODE_ENV", "test")]
[env("LOG_LEVEL", "debug")]
test-integration:
    bun test:integration
```

As of v1.51.0, the value accepts arbitrary expressions, including other variables and functions:

```just
build_id := `git rev-parse --short HEAD`

[env("BUILD_ID", build_id)]
[env("CACHE_DIR", justfile_dir() / ".cache")]
build:
    cargo build
```

`[env(...)]` is the preferred mechanism for one-recipe env overrides — it composes with `set dotenv-load` and module-level
exports (which it overrides as of v1.51.0).

### Parallel Execution

Marks a recipe so its **direct dependencies run concurrently** instead of sequentially. Introduced in just 1.42.0
([casey/just#2803](https://github.com/casey/just/pull/2803)).

```just
[parallel]
ci: lint test build

lint:
    bun lint
test:
    bun test
build:
    bun run build
```

**Behavior:**

- Only **direct** dependencies fan out. The recipe body itself still runs after all parallel deps finish.
- **Shared transitive deps run exactly once before the parallel fan-out**, so this is the right primitive for
  `setup → fan-out work` graphs — a dep referenced by multiple parallel siblings is deduped, not run once per sibling.
- No concurrency cap is exposed; just launches all direct deps at once.
- Empirical timing on a sleep-0.5 fan-out of three deps: **1.54 s sequential → 0.54 s with `[parallel]`**.

Pairs naturally with parameterized dependencies (`recipe: (sub-recipe "arg1" "arg2")`) for table-driven fan-out where
every sibling shares a setup step:

```just
[parallel]
check: \
    (_check "lint")  \
    (_check "test")  \
    (_check "types")

_check kind: setup
    ./scripts/check.sh {{ kind }}

setup:
    bun install --frozen-lockfile
```

Here `setup` runs once thanks to transitive dedup, then the three `_check` invocations run concurrently.

### Documentation

```just
# Comment becomes doc (default)
# Build the project
build:
    npm run build

# Override with attribute
[doc("Compile TypeScript and bundle")]
build:
    npm run build

# Suppress documentation
[doc]
internal-helper:
    echo "hidden"
```

### Combining Attributes

```just
# Same line (comma-separated)
[no-cd, private]
helper:
    echo "helper"

# Multiple lines
[group("codegen")]
[script("bash")]
[confirm("Generate bindings?")]
codegen:
    ./generate.sh
```

### Per-Recipe Positional Arguments

```just
[positional-arguments]
@greet name:
    echo "Hello, $1!"
```

## Recipe Parameters

### Required Parameters

```just
greet name:
    echo "Hello, {{ name }}"
```

### Default Parameters

```just
greet name="World":
    echo "Hello, {{ name }}"
```

### Variadic Parameters

```just
# One or more arguments (required)
test +files:
    npm test {{ files }}

# Zero or more arguments (optional)
build *flags:
    npm run build {{ flags }}
```

### Parameter with Environment Variable

```just
# Set from env or use default
deploy env=env("DEPLOY_ENV", "staging"):
    ./deploy.sh {{ env }}
```

## Recipe Argument Flags (v1.46.0+)

The `[arg()]` attribute configures parameters as command-line options.

### Long Options

Use `long` to accept `--name` style options:

```just
# Explicit long name
[arg("target", long="target")]
build target:
    cargo build --target {{ target }}

# Default to parameter name (recommended)
[arg("target", long)]
build target:
    cargo build --target {{ target }}

# Usage:
#   just build --target x86_64
#   just build --target=x86_64
```

### Short Options

Use `short` to accept `-x` style options:

```just
[arg("verbose", short="v")]
run verbose="false":
    echo "Verbose: {{ verbose }}"

# Usage: just run -v true
```

### Combined Long and Short

A parameter can accept both styles:

```just
[arg("output", long="output", short="o")]
compile output:
    gcc main.c -o {{ output }}

# Usage:
#   just compile --output main
#   just compile -o main
```

### Flags Without Values

Use `value` (v1.46.0+) for boolean-style flags that set a predefined value when present. Give the parameter a default so the flag is optional. This is the stable form and works on any `just` ≥ 1.46.0:

```just
[arg("release", long, value="true")]
build release="false":
    cargo build {{ if release == "true" { "--release" } else { "" } }}

# Usage:
#   just build           → release="false" (default)
#   just build --release → release="true"
```

Under `set lists` (unstable, v1.53.0+), the `flag` keyword is an alternative: presence sets the parameter to `"true"`, absence leaves it `[]` (the canonical false). Flag parameters may **not** have a default, and `set unstable` + `set lists` are required:

```just
set unstable
set lists

[arg("release", long, flag)]
build release:
    cargo build {{ if release == "true" { "--release" } else { "" } }}

# Usage:
#   just build           → release=[] (falsy)
#   just build --release → release="true"
```

### Help Strings

Use `help` to add descriptions visible in `just --usage`:

```just
[arg("target", long, help="Target architecture")]
[arg("release", long, value="true", help="Build in release mode")]
build target release="false":
    cargo build --target {{ target }}
```

```console
$ just --usage build
Usage: just build [OPTIONS] --target <target>

Arguments:
  --target <target>  Target architecture
  --release          Build in release mode
```

### Multiple arg Attributes

Each parameter with options needs its own `[arg()]`:

```just
[arg("input", long, short="i", help="Input file")]
[arg("output", long, short="o", help="Output file")]
[arg("verbose", long, short="v", value="true")]
convert input output verbose="false":
    convert {{ input }} {{ output }} {{ if verbose == "true" { "-v" } else { "" } }}
```

### Pattern Constraints

Use `pattern` to constrain arguments to match a regular expression:

```just
# Require numeric input
[arg('n', pattern='\d+')]
double n:
    echo $(({{n}} * 2))

# Usage:
#   just double 5      → valid
#   just double abc    → error: argument doesn't match pattern
```

Use the `|` operator to constrain to specific alternatives:

```just
[arg('flag', pattern='--help|--version')]
info flag:
    just {{flag}}

# Usage:
#   just info --help      → valid
#   just info --version   → valid
#   just info --foo       → error: argument doesn't match pattern
```

### Arg Attribute Syntax Summary

| Option            | Description                                                              |
| ----------------- | ------------------------------------------------------------------------ |
| `long`            | Accept `--param` (defaults to name)                                      |
| `long="name"`     | Accept `--name`                                                          |
| `short="x"`       | Accept `-x`                                                              |
| `value="val"`     | Set this value when flag present (stable; v1.46.0+)                      |
| `flag`            | Valueless flag ⇒ `"true"`/`[]`; needs `set lists`, no default (v1.53.0+) |
| `help="text"`     | Description for `just --usage`                                           |
| `pattern="regex"` | Constrain argument to match regex                                        |

## Recipe Dependencies

### Simple Dependencies

```just
build: clean compile
    echo "Build complete"

clean:
    rm -rf dist/

compile:
    tsc
```

### Dependencies with Arguments

```just
deploy env: (build env)
    ./deploy.sh {{ env }}

build env:
    npm run build:{{ env }}
```

### Conditional Execution

```just
test: && lint
    npm test

# lint runs only if test succeeds
```

### Mapped Dependencies over Lists (unstable, v1.53.0+)

With `set lists`, a dependency can be invoked once per element of a list argument using `*(recipe *arg)`. Combine with `[parallel]` to fan out concurrently:

```just
set unstable
set lists

[parallel]
build target *platform: *(compile target *platform)

@compile target platform:
    echo compiling {{ target }} for {{ platform }}…
```

```console
$ just build x86 foo bar
compiling foo for x86…
compiling bar for x86…
```

Each argument to a non-mapped dependency binds to exactly one parameter; passing extra arguments to a variadic dependency is an error.

## Command Prefixes

| Prefix       | Effect             |
| ------------ | ------------------ |
| `@`          | Don't echo command |
| `-`          | Ignore errors      |
| `@-` or `-@` | Both               |

```just
@quiet:
    echo "Only output shown"

-ignore-error:
    false
    echo "Still runs"

@-both:
    false
    echo "Quiet and ignores error"
```

## Shebang Recipes

Execute with specific interpreter:

```just
python-script:
    #!/usr/bin/env python3
    import sys
    print(f"Python {sys.version}")

node-script:
    #!/usr/bin/env node
    console.log(process.version)
```

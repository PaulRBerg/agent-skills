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
```

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
```

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

Use `value` for boolean-style flags that set a predefined value when present:

```just
[arg("release", long, value="true")]
build release="false":
    cargo build {{ if release == "true" { "--release" } else { "" } }}

# Usage:
#   just build           → release="false" (default)
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

| Option            | Description                         |
| ----------------- | ----------------------------------- |
| `long`            | Accept `--param` (defaults to name) |
| `long="name"`     | Accept `--name`                     |
| `short="x"`       | Accept `-x`                         |
| `value="val"`     | Set this value when flag present    |
| `help="text"`     | Description for `just --usage`      |
| `pattern="regex"` | Constrain argument to match regex   |

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

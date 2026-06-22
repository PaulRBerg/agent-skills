# Inline Scripts

> When to read: when writing recipes that need shell scripts (script attribute or shebang style).

Just supports inline scripts in any language via two methods:

## Script Attribute (Recommended)

Use `[script("interpreter")]` for cross-platform compatibility:

```just
[script("node")]
fetch-data:
    const response = await fetch('https://api.example.com/data');
    const data = await response.json();
    console.log(data);

[script("python3")]
analyze:
    import json
    with open('package.json') as f:
        pkg = json.load(f)
    print(f"Package: {pkg['name']}@{pkg['version']}")

[script("bash")]
deploy:
    set -e
    npm run build
    aws s3 sync dist/ s3://bucket/
```

## Default Script Mode

Use `set default-script := true` (v1.52.0+) when most recipes should run as complete scripts instead of independent shell
lines:

```just
set default-script := true
set script-interpreter := ["bash", "-euo", "pipefail"]

deploy:
    trap 'echo failed' ERR
    npm run build
    npm publish

[shell]
status:
    echo "force this one back to linewise shell execution"
```

`[shell]` overrides `default-script` for one recipe. `set shell` still controls linewise recipes and backticks; `set script-interpreter` controls `[script]` recipes with no explicit command.

## Shebang Method

Use `#!/usr/bin/env interpreter` at the recipe start:

```just
node-script:
    #!/usr/bin/env node
    console.log(`Node ${process.version}`);
    console.log(JSON.stringify(process.env, null, 2));

python-script:
    #!/usr/bin/env python3
    import sys
    print(f"Python {sys.version}")

bash-script:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Running on $(uname -s)"
```

**When to use which:**

- `[script()]` - Cleaner multi-line recipe syntax
- Shebang - Traditional Unix approach, works without `set unstable`

## Bash Version Pitfalls (macOS)

`[script("bash")]`, `#!/usr/bin/env bash`, and `set shell := ["bash", ...]` all resolve `bash` via `PATH`. On stock macOS — and in minimal-`PATH` agent sandboxes — that is `/bin/bash` 3.2 (2007), not Homebrew's 5.x. Recipes written against Bash 4+ fail with signatures like:

```
mapfile: command not found
declare: -A: invalid option
```

**Bash-4+ features and 3.2-safe replacements:**

| Bash 4+ feature         | 3.2-safe replacement                           |
| ----------------------- | ---------------------------------------------- |
| `mapfile -t arr < file` | `while IFS= read -r line; do ...; done < file` |
| `declare -A map`        | `case` statement or parallel arrays            |
| `${var,,}` / `${var^^}` | `tr '[:upper:]' '[:lower:]'` (and inverse)     |
| `${arr[-1]}`            | `${arr[${#arr[@]}-1]}`                         |
| `cmd \|& other`         | `cmd 2>&1 \| other`                            |

**Pinning a newer interpreter** works on this Apple Silicon/Homebrew profile when the path exists, but keep it explicit and guarded because agent sandboxes may expose only `/bin/bash`:

```just
set shell := ["/opt/homebrew/bin/bash", "-euo", "pipefail", "-c"]
```

**3.2-safe version guard** when a recipe genuinely needs Bash 4+ (`BASH_VERSINFO` exists in 3.2, so the check itself never breaks):

```just
[script("bash")]
modern:
    if [ "${BASH_VERSINFO[0]}" -lt 4 ]; then
        echo "error: bash >= 4 required (found $BASH_VERSION)" >&2
        exit 1
    fi
    declare -A map=([a]=1)
```

**Default recommendation:** write recipe bodies that are Bash-3.2-safe. Pin `/opt/homebrew/bin/bash` only when a recipe genuinely needs Bash 4+ semantics.

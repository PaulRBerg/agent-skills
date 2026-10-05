# Inline Scripts

> Read when a recipe needs multi-line shell or another interpreter. Confirm the installed syntax with `just --help`,
> `just --man`, or the current manual before adding an attribute or setting.

Use linewise recipes for short commands. Use `[script("interpreter")]` or a shebang when the body needs shell state,
traps, control flow, or a non-shell language. `set default-script` changes the default. `[shell]` returns one recipe to
linewise execution.

```just
[script("bash")]
deploy:
    set -e
    ./build.sh
    ./deploy.sh
```

Script recipes are quiet already. Do not prefix a `[script]` recipe with `@`. On a script recipe, that prefix prints the
generated script body. See [recipes.md](recipes.md#quiet-recipes-and-command-prefixes) for linewise echo rules.

## Bash on macOS

Assume `/bin/bash` 3.2 unless the recipe selects another shell. Write 3.2-compatible bodies by default. Use a fixed
newer interpreter only for semantics that require it. Ensure that interpreter path is available in CI and agent
environments.

```just
set shell := ["/opt/homebrew/bin/bash", "-euo", "pipefail", "-c"]
```

For a Bash-4+-only recipe, add a guard before using its newer syntax:

```just
[script("bash")]
modern:
    if [ "${BASH_VERSINFO[0]}" -lt 4 ]; then
        echo "error: bash >= 4 required (found $BASH_VERSION)" >&2
        exit 1
    fi
    declare -A map=([a]=1)
```

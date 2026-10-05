# Just Recipes Reference

> Read for recipe shape, dependencies, parameters, attributes, or command echoing. Discover version-sensitive syntax
> from the installed `just --help`, `just --man`, or current manual before writing it.

Keep recipes explicit and short. Use parameters and dependencies only when they make the invocation clearer. If a helper
is not an intended entrypoint, make it private. Scope environment variables, working directories, OS restrictions,
confirmations, grouping, and parallelism to the recipe that needs them.

## Native executable checks

Use Just-native checks instead of shell `command -v` or backtick `which`. When the justfile already enables the needed
`set unstable` and `set lists`, `which()` with `assert()` can give an actionable install message. Otherwise use
`require()` when its standard missing-executable error is enough. If every recipe needs the tool, put the assertion at
top level. Otherwise, put it in a private prerequisite of affected recipes.

## Quiet recipes and command prefixes

Make ordinary recipes quiet with `@recipe:` when echoed commands add no value. In a quiet recipe, a line-level `@`
inverts that choice and deliberately echoes that line, so leave lines unprefixed by default. In a non-quiet recipe,
line-level `@` suppresses one command. `-` ignores one command error. See [inline-scripts.md](inline-scripts.md) for
script echoing.

```just
@check:
    cargo test
    @echo "this command is deliberately echoed"
```

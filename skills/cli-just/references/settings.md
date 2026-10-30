# Just Settings & Modules Reference

> Read for global configuration, dotenv behavior, lists, imports, or modules. Verify every setting against the installed
> `just --help`, `just --man`, or current manual.

Put settings before imports and recipes. Preserve the repository's existing choices for shell, dotenv, fallback,
quietness, module layout, and duplicate definitions. Put shared imports first and local or optional overrides last.

For an unstable feature, check whether it also requires a companion setting such as `set lists`. Lists represent
collections of distinct values. Parenthesized string concatenation builds one string.

When a root justfile mounts a package justfile with `mod`, `just` parses every module on every invocation. A module that
imports from `node_modules` or asserts a binary at load time, such as the devkit `import` and `require("na")` lines in
`examples/devkit.just`, breaks unrelated recipes on checkouts without that toolchain, for example `just ios test` in CI.
In a module, make the import optional with `import?` and drop load-time `require()` asserts. Devkit recipes marked
`[no-cd]` run in the caller's directory, so run them from the package directory rather than through the module.

For the exact setting name, module path syntax, dotenv precedence, and version gate, inspect the installed manual.

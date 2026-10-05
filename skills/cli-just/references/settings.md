# Just Settings & Modules Reference

> Read for global configuration, dotenv behavior, lists, imports, or modules. Verify every setting against the installed
> `just --help`, `just --man`, or current manual.

Put settings before imports and recipes. Preserve the repository's existing choices for shell, dotenv, fallback,
quietness, module layout, and duplicate definitions. Put shared imports first and local or optional overrides last.

For an unstable feature, check whether it also requires a companion setting such as `set lists`. Lists represent
collections of distinct values. Parenthesized string concatenation builds one string.

For the exact setting name, module path syntax, dotenv precedence, and version gate, inspect the installed manual.

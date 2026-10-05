# Just Syntax Reference

> Read for expressions, functions, constants, interpolation, or version-gated syntax. Prefer the installed manual:
> `just --help`, `just --man`, or the current online manual.

Evaluate shell work only when Just cannot express it directly. For the collection-versus-string distinction, see
[settings.md](settings.md).

Place `set minimum-version` at the very top, before features it guards. The check runs in the parser. Incompatible lexer
syntax can still fail before that check. Confirm the feature's introduction version and choose compatible syntax or
raise the repository's minimum version. See the
[minimum-version manual](https://just.systems/man/en/requiring-a-minimum-just-version.html).

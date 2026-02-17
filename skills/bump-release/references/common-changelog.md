# Common Changelog

Stripped-down reference for generating changelogs. Full spec: <https://common-changelog.org/>

## Guiding Principles

- Changelogs are for humans.
- Communicate the impact of changes.
- Sort content by importance.
- Skip content that isn't important.
- Link each change to further information.

## Format

### File Format

Filename must be `CHANGELOG.md`. Content must be Markdown starting with:

```md
# Changelog
```

Releases must be sorted latest-first by semver. There must be an entry for every new stable release.

### Release

A release starts with a second-level heading:

```md
## VERSION - DATE
```

`VERSION` is semver-valid (no "v" prefix) matching a git tag (with optional "v" prefix). `DATE` is `YYYY-MM-DD` (ISO 8601).

```md
## 1.0.1 - 2019-08-24
```

The version should link to further information. If hosted on GitHub, link to a GitHub release. Use reference-style links to keep raw Markdown readable:

```md
## [1.0.1] - 2019-08-24

### Fixed

- Prevent segmentation fault upon `close()`

[1.0.1]: https://github.com/owner/name/releases/tag/v1.0.1
```

After the heading, a release must contain either:

1. One or more change groups
2. A notice followed by zero or more change groups

### Notice

A single-sentence paragraph with Markdown emphasis, used for upgrade guides, status clarification, or first releases:

```md
## [2.0.0] - 2020-07-23

_If you are upgrading: please see [`UPGRADING.md`](UPGRADING.md)._
```

```md
## [1.0.0] - 2019-08-23

_First release._
```

There can only be one notice per release.

### Change Group

A change group starts with a third-level heading containing a category:

```md
### <category>
```

Categories must be one of (in this order):

- `Changed` — changes in existing functionality
- `Added` — new functionality
- `Removed` — removed functionality
- `Fixed` — bug fixes

The heading is followed by an unnumbered Markdown list. Each item is a single line: a change, then references, then optional authors.

```md
- Prevent buffer overflow ([#28](https://github.com/owner/name/pull/28))
```

Sort the list: breaking changes first, then by importance, then latest-first.

#### Change

Write using imperative mood. Must start with a present-tense verb (e.g. `Add`, `Refactor`, `Bump`, `Document`, `Fix`, `Deprecate`).

Each change must be self-describing, as if no category heading exists. Instead of:

```md
### Added

- Support of CentOS
- `write()` method
```

Write:

```md
### Added

- Support CentOS
- Add `write()` method
```

#### References

Each change should reference a PR when available; fall back to a commit hash only if no PR exists. References are written after the change on the same line, wrapped in parentheses. Each reference must be a Markdown link.

**Pull requests (preferred):**

```md
([#194](https://github.com/owner/name/issues/194))
```

**Commits (fallback):**

```md
([`53bd922`](https://github.com/owner/name/commit/53bd922))
```

When there are more than two references, only include the best starting point.

#### Authors

Author names are optional and should generally be omitted. Only include them in multi-contributor projects where attribution matters.

#### Prefixes

Breaking changes must be prefixed in bold: `**Breaking:** ` and listed before other changes per category:

```md
### Changed

- **Breaking:** emit `close` event after `end`
- Refactor `sort()` internals to improve performance

### Removed

- **Breaking:** drop support of Node.js 8
```

For subsystem prefixes: `**<subsystem> (breaking):** `:

```md
- **Installer (breaking):** enable silent mode by default
- **UI:** tune button colors for accessibility
```

## Writing

### Remove Noise

Exclude maintenance changes not interesting to consumers:

- Dotfile changes (`.gitignore`, `.github`, `.gitlab`, etc.)
- Changes to development-only dependencies
- Minor code style changes
- Formatting changes in documentation

Do **not** exclude:

- Refactorings (may have unintentional side effects)
- Changes to supported runtime environments
- Code style changes that use new language features
- New documentation for previously undocumented features

### Rephrase Changes

Align terminology across contributors. Add details where missing, strip details when irrelevant. Instead of:

```md
- Upgrade json-parser from 2.2.0 to 3.0.1
- Bump `xml-parser`
```

Write:

```md
- Bump `json-parser` from 2.x to 3.x
- Bump `xml-parser` from 6.x to 8.x
```

### Merge Related Changes

If a change happened over multiple commits, list them as one:

```md
- Bump `standard` from 14.x to 16.x (a, b)
```

Fixups get absorbed into the original change:

```md
- Support filtering entries by name (a, b)
```

### Skip No-Op Changes

If commits between releases negate each other (e.g. one reverts the other), leave them out.

### Separate Message and Description

A change should be brief — no more than one line. Long descriptions belong in commits or referenced PRs:

```md
- **Breaking:** bump `yaml-parser` from 4.x to 5.x (`15d5a9e`)
```

Exception — if the commit lacks a description:

```md
- **Breaking:** bump `yaml-parser` from 4.x to 5.x (`15d5a9e`). Removes the `unsafe` option.
```

### Promoting a Prerelease

Three approaches:

**A. Copy content to release** — Merge content from prereleases into the release entry, following the same writing practices. Write as if prereleases don't exist.

**B. Skip changelog entry for prerelease** — When the prerelease is for internal testing only.

**C. Refer to prerelease** — Use a notice: `_Stable release based on <prerelease version>._`

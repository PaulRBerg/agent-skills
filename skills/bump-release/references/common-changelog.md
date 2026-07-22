# Common Changelog

Write stable release notes for consumers. The structure validator owns mechanical conformance; this reference owns the
agent's semantic and editorial decisions. Full specification: <https://common-changelog.org/>.

## Consumer Contract

- Changelogs are for humans. Describe surviving user/developer impact, not the commit sequence.
- Keep releases latest-first and include every new stable release.
- Use `Changed`, `Added`, `Removed`, and `Fixed` according to meaning. Put the most important consumer effect first.
- A first release or upgrade warning may use one emphasized notice.
- Link the release version to the matching GitHub release/tag and each change to the best PR, falling back to a commit
  only when no PR exists.

Run `scripts/validate-changelog.py` with the expected version, date, and tag after writing. Fix its structural errors;
do not ask it to decide category, wording, importance, or whether a change belongs in the release.

## Write Entries

Start each item with an imperative present-tense verb and make it understandable without its category heading:

```md
### Added

- Support CentOS ([#28](https://github.com/owner/name/pull/28))
- Add `write()` streaming mode ([`53bd922`](https://github.com/owner/name/commit/53bd922))
```

Mark breaking effects explicitly and place them before non-breaking items in the same category:

```md
- **Breaking:** emit `close` after `end`
- **Installer (breaking):** enable silent mode by default
```

Use subsystem prefixes only when they improve comprehension. Keep each change self-contained and brief; longer
explanation belongs in the linked PR/commit unless the source lacks necessary context.

## Select Relevant Changes

Inspect the complete bounded net diff. Exclude changes that do not matter to consumers, such as formatting-only churn,
development-only dependency maintenance, or negated intermediate work. Do not exclude a change solely from its filename
or location. Runtime environment changes, refactors, language-feature changes, and newly documented behavior may all be
release-relevant.

Rephrase inconsistent commit language into product terminology. Merge related commits and fixups into one surviving
outcome. Prefer the PR that best explains the change; include at most the few references needed to reach that context.
Author attribution is optional and useful only when the project's conventions make it meaningful.

## Prerelease Promotion

For a stable release after prereleases, choose the consumer-appropriate narrative: merge the prerelease content into the
stable entry, omit internal-only prerelease notes, or use a short notice referring to the prerelease. Write the stable
release as the supported outcome, not as a transcript of beta iterations.

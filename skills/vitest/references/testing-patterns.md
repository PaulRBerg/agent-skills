# Testing Patterns

> Match local naming, imports/globals, DOM utilities, fixtures, and cleanup.

- For components, drive accessible observable behavior: query by role or label, then visible text, with semantic test
  IDs last. Keep provider state fresh. Only when several tests need a wrapper, centralize it. Use jest-dom matchers only
  when the setup file imports them.
- Await or return every promise. Wrap callback APIs in a promise. A test callback parameter is Vitest context, not a
  completion callback. Since v4, pass an options object as the second argument (a numeric final timeout remains
  supported).
- Put fixture cleanup with the fixture and scenario data with its test. Use tables only where cases clarify distinct
  behavior. Use snapshots only for stable, reviewable output. Focused assertions are usually better.
- Do not commit `only`. Use skips, tags, or concurrency only with an explicit condition and safe isolation.
- Only when the repository already owns browser configuration, use browser mode and visual assertions. Do not add it
  merely for a component test covered by the established environment.

See [Test API](https://vitest.dev/api/test) for version-specific options.

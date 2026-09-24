# Configuration

> Configuration and runner scripts are repository-owned. Add or migrate config only for the requested behavior.

- Keep project-specific environment and setup ownership. Use `test.projects` for distinct projects and give each a
  stable name; a project container does not run tests itself. Do not change a suite-wide environment for one exception.
- Keep aliases consistent with TypeScript and Vite resolution; aliases do not repair arbitrary externalized
  dependencies.
- Setup files are for truly suite-wide polyfills, matcher registration, cleanup, and mocks. Keep scenario data and heavy
  initialization in owned fixtures or tests.
- Coverage is opt-in. When requested, specify the intended source set and retain repository thresholds; do not use a
  provider change or lower threshold to hide a bad include/exclude set.
- For v4 migration, replace removed workspace/match-glob and worker settings with the documented project/worker model;
  `coverage.all` and the `basic` reporter are gone, `verbose` is flat, and browser APIs come from `vitest/browser`. See
  the [v4 migration guide](https://v4.vitest.dev/guide/migration).
- For v5 migration, require Vite 6.4+ and Node.js 22.12+; `vite` is now a peer dependency, so Yarn projects must add it.
  Audit these changes, which can silently alter results rather than fail:
  - `clearMocks` defaults to `true`, clearing call history recorded in setup files, at module scope, or in `beforeAll`.
  - Inline `test.projects` inherit the root config (`extends: true`, arrays such as `setupFiles` append) and share its
    Vite server (`sharedViteServer`). A referenced config that declares `projects` now expands into nested projects, so
    do not merge a root config that defines them.
  - `-t`/`testNamePattern` matches the `' > '`-joined full name; `test.each`/`test.for` titles no longer quote `$`
    strings.
  - Coverage `include`/`exclude` match project-relative paths without `contains`; glob thresholds no longer inherit
    `perFile`.
  - Config files are no longer found in parent directories; pass `--config` from a subdirectory.
  - Artifacts live under `.vitest/`, and the `json` and `junit` reporters write files instead of stdout unless
    `stdout: true` is set.
  - `VITEST_POOL_ID` and `VITEST_WORKER_ID` are 1-based.
  - Browser locators and `toHaveTextContent` match exactly (partial or regex matching moved to `toMatchTextContent`),
    and `browser.api` moved to top-level `api`.

  These now fail loudly: nested `vi.mock`/`vi.unmock`/`vi.hoisted`, unawaited
  `resolves`/`rejects`/`toMatchFileSnapshot`, late `expect.poll`, removed `sequential` APIs (use
  `{ concurrent: false }`), module-scope `bench` (now a test-context fixture), and the removed `vitest/coverage`,
  `vitest/reporters` (use `vitest/node`), `vitest/environments`, and `vitest/snapshot` (use `vitest/runtime`) entry
  points. Check the installed version's [migration guide](https://vitest.dev/guide/migration.html) rather than carrying
  shims.

Use repository reporter conventions. Otherwise prefer minimal agent-oriented output: Vitest 4.1 introduced
`--reporter=agent`; current versions call it `minimal` and retain `agent` as an alias. Use automatic agent detection
when available, and check the installed version's [reporter documentation](https://vitest.dev/guide/reporters.html)
before passing a flag. See [configuration](https://vitest.dev/config/file),
[projects](https://vitest.dev/config/projects), and [aliases](https://vitest.dev/config/alias).

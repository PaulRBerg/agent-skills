---
disable-model-invocation: false
name: vitest
user-invocable: true
description:
  "Use for Vitest in TypeScript React/Next.js: write, run, or debug unit/component tests, mocks, testing utilities, and
  coverage."
---

# Vitest

Follow the repository's Vitest configuration and test conventions before introducing generic patterns.

## Workflow

1. Inspect package scripts, Vitest config/projects, setup files, neighboring tests, path aliases, environment selection,
   and repository instructions. In a workspace, read [references/monorepo-testing.md](references/monorepo-testing.md)
   for the relevant shared-versus-package boundary.
2. Define the behavior or regression the test must prove. Prefer public behavior and observable outcomes over
   implementation details.
3. Match local file placement, naming, imports/globals, fixtures, cleanup, DOM utilities, and assertion style. Do not
   enable globals, jsdom, coverage, or new setup merely because they are common defaults.
4. Load conditional guidance only when needed:
   - components, async behavior, custom matchers, snapshots, type tests, tables: `references/testing-patterns.md`;
   - spies, module mocks, timers: `references/mocking.md`;
   - timeouts, flaky async tests, mock failures, environment/config errors: `references/troubleshooting.md`.
5. Run the narrowest established command for the changed file or test name, then the affected package suite when shared
   setup or contracts changed. Use `nlx vitest run ...` only when the repository has no preferred recipe/script.

## Defaults

- Colocate tests when the repository does.
- Restore mocks, timers, environment, and mutable shared state using the local cleanup convention.
- Mock system boundaries, not the behavior under test.
- Add coverage configuration only when coverage is the requested outcome.
- For a bug fix, reproduce the failure before relying on the passing result when practical.

Completion requires a focused test that fails for the intended regression or meaningfully exercises the new behavior,
then passes under the repository's configuration with no leaked state.

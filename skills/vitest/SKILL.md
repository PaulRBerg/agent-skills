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
   and repository instructions. Read [references/configuration.md](references/configuration.md) for config anatomy,
   projects, environments, and setup files.
2. Define the behavior or regression the test must prove. Prefer public behavior and observable outcomes over
   implementation details.
3. Match local file placement, naming, imports/globals, fixtures, cleanup, DOM utilities, and assertion style. Do not
   enable globals, jsdom, coverage, or new setup merely because they are common defaults.
4. Load conditional guidance only when needed:
   - components, async behavior, snapshots, type tests, tables, fixtures, tags:
     [references/testing-patterns.md](references/testing-patterns.md);
   - spies, module mocks, timers, environment stubs: [references/mocking.md](references/mocking.md);
   - config, projects, environments, coverage, reporters, v4 migration:
     [references/configuration.md](references/configuration.md);
   - timeouts, flaky tests, mock failures, resolution errors:
     [references/troubleshooting.md](references/troubleshooting.md).
5. Run the narrowest established command for the changed file or test name, then the affected package suite when shared
   setup or contracts changed. Use `nlx vitest run ...` only when the repository has no preferred recipe/script. Prefer
   `--reporter=agent` on Vitest 4.1+ for minimal agent-friendly output when the repository has no reporter convention.

## Defaults

- Colocate tests when the repository does.
- Restore mocks, timers, environment, and mutable shared state using the local cleanup convention.
- Mock system boundaries, not the behavior under test.
- Add coverage configuration only when coverage is the requested outcome.
- Do not use jest-dom matchers unless setup imports `@testing-library/jest-dom`.
- In Effect-TS repositories, follow `@effect/vitest` conventions (`it.effect`, Layers, TestClock) instead of generic
  patterns.
- For a bug fix, reproduce the failure before relying on the passing result when practical.

Completion requires a focused test that fails for the intended regression or meaningfully exercises the new behavior,
then passes under the repository's configuration with no leaked state.

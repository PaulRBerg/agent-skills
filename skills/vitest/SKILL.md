---
name: vitest
description:
  "Use for Vitest in TypeScript projects (Node, bun, React/Next.js, Effect): write, run, or debug unit/component tests,
  mocks, testing utilities, and coverage."
---

# Vitest

Repository configuration, projects, setup, imports/globals, environments, aliases, test placement, and cleanup own the
test contract. Inspect them and nearby tests before adding generic patterns. Do not add globals, jsdom, coverage, new
setup, or browser mode by default.

## Workflow

Define the behavior or regression, test its observable public result, and mock system boundaries rather than the
behavior under test. Follow local fixture and cleanup ownership. In Effect repositories, use established
`@effect/vitest` conventions such as `it.effect`, Layers, and TestClock rather than generic replacements. For a bug fix,
reproduce the failure before relying on a passing result when practical.

Read [patterns](references/testing-patterns.md) for component, async, fixture, snapshot, tag, or browser questions;
[mocking](references/mocking.md) for module, timer, spy, or global boundaries;
[configuration](references/configuration.md) for projects, migration, coverage, or reporter selection; and
[troubleshooting](references/troubleshooting.md) for hangs, discovery, resolution, or flaky state.

For test changes, run the narrowest established command for the changed behavior, then the affected package suite only
when shared setup or contracts changed. Use `nlx vitest run` only when no project recipe or script exists. Reuse passing
results until new edits, failures, or unresolved concerns justify another run.

Completion requires a meaningful passing focused test under repository configuration and concise command/result evidence
for test changes. A read-only explanation instead requires supporting configuration or API evidence; do not add or run
tests merely to satisfy the change workflow. Use `### 🧪 Regression covered` when red-before-green evidence exists;
otherwise `### 🧪 Tests verified` for executed tests.

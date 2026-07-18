# Troubleshooting

> Read for hangs, timeouts, flaky tests, mock failures, environment errors, resolution failures, or missing tests.

Reproduce with the repository's narrowest established command. Use `nlx vitest run ...` only when the repository has no
preferred script or recipe.

## Hangs and timeouts

Start hang diagnosis with async leak detection:

```bash
nlx vitest run path/to/problem.test.ts --detect-async-leaks
```

It reports resources such as timers and sockets created by the test file and left open. It slows execution, so keep it
diagnostic-only.

For a legitimate slow boundary, pass options as the second argument:

```typescript
test("completes the integration", { timeout: 10_000 }, async () => {
  await runIntegration();
});
```

Before raising a timeout, check for:

- an unreturned or unawaited promise;
- a mock that never resolves;
- a retry loop without a bound;
- a server, socket, subprocess, or timer missing teardown;
- fake timers controlling an awaited scheduler.

Wrap callback APIs in a promise and resolve or reject on every path. Vitest does not use a completion callback argument
for tests.

If fake timers stall, install them before scheduling work and advance async callbacks with the async API:

```typescript
vi.useFakeTimers();
const result = scheduleRefresh();
await vi.advanceTimersByTimeAsync(1_000);
await expect(result).resolves.toBe("refreshed");
```

Restore real timers in `afterEach`. See `mocking.md` for timer ownership and cleanup.

## Mock and resolution failures

When a mock is not applied, compare the mocked specifier with the production import:

```typescript
// Different module identities: this mock will not replace the aliased import.
vi.mock("./logger");
import { logger } from "@shared/logger";

// Match the production specifier.
vi.mock("@shared/logger");
```

Mirror TypeScript paths in `resolve.alias`, without trailing slash differences. Put aliases needed by package setup
files in the shared project config.

If a factory reports an initialization or hoisting error, move referenced mock handles into `vi.hoisted` and keep the
`vi.mock` declaration at file scope. Do not move a module mock into the test body. See `mocking.md` for the canonical
partial-mock pattern.

Type function mocks with one function signature:

```typescript
const fetchMock = vi.fn<(url: string) => Promise<Response>>();
fetchMock.mockResolvedValue(new Response("ok"));
```

An unnamed mock's `getMockName()` returns `"vi.fn()"` in current Vitest. Account for that string when reading errors or
reviewing snapshots that contain mock names; call `mockName` when a stable domain name matters.

In Vitest 4, `vi.restoreAllMocks()` restores manual spies only. It does not restore automocked exports. Reset owned
automocks explicitly or use the repository's `mockReset` convention.

## State bleeding and flaky order

Suspect leaked state when tests pass alone but fail together, failure order changes, or disabling file parallelism makes
the run pass. Diagnose without adopting serialization as the fix:

```bash
nlx vitest run path/to/problem.test.ts
nlx vitest run --no-file-parallelism
```

Own cleanup where the state is created:

```typescript
afterEach(() => {
  fetchUserMock.mockReset();
  vi.restoreAllMocks();
  vi.unstubAllEnvs();
  vi.useRealTimers();
  store.reset();
});
```

Create fresh stores, clients, query caches, and stateful mock factories per test. Preserve and restore global property
descriptors rather than assigning or deleting globals. Do not hide a race by setting `maxWorkers: 1`.

## Environment and assertion errors

`ReferenceError: document is not defined` means the test is running in a non-DOM environment. Follow the project's web
environment, or override the exceptional file:

```typescript
// @vitest-environment jsdom

import { render } from "@testing-library/react";
```

`ReferenceError: describe is not defined` means globals are disabled or the file is executing outside Vitest. Prefer
explicit imports even when `globals: true` is configured:

```typescript
import { describe, expect, test } from "vitest";
```

`toBeInTheDocument is not a function` means setup does not install `@testing-library/jest-dom`. Use plain DOM assertions
such as `expect(element.textContent).toContain("Saved")`, unless the repository intentionally uses jest-dom.

## Tests not found

Confirm all four boundaries:

1. The filename matches the configured `test.include`, commonly `.test.ts` or `.test.tsx`.
2. No `exclude` glob removes the file.
3. The command runs from the expected config root or selects the correct `--project`.
4. The named project includes the package containing the file.

Inspect collected tests without executing them when useful:

```bash
nlx vitest list path/to/package
nlx vitest list --project web
```

Do not rename files to `.spec.*` unless that convention is already included.

## Reporter choice

Use the repository's reporter convention. For an ad hoc diagnostic run:

```bash
nlx vitest run --reporter=agent
nlx vitest run --reporter=tree
nlx vitest run --reporter=verbose
```

`agent` minimizes output for coding agents. `tree` preserves suite hierarchy. `verbose` is flat in Vitest 4; the old
`basic` reporter no longer exists.

## Coverage discrepancies

Coverage requires explicit source inclusion. Keep thresholds under `coverage.thresholds`:

```typescript
export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      include: ["src/**/*.{ts,tsx}"],
      exclude: ["src/**/*.test.{ts,tsx}", "src/**/*.d.ts"],
      thresholds: {
        branches: 80,
        functions: 80,
        lines: 90,
        statements: 90,
      },
    },
  },
});
```

Check the include and exclude sets before changing thresholds. V8 coverage is AST-aware in Vitest 4; do not change
provider solely to compensate for an incorrect source set.

## Cache after config changes

Vitest's default cache is under `node_modules/.vite`, but Vite `cacheDir` can move it. Prefer Vitest's cache command to
guessing the directory:

```bash
nlx vitest --clearCache
```

Clear the cache only after dependency, transform, alias, or config changes produce evidence of stale resolution. It is
not a general fix for deterministic test failures.

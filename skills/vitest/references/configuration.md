# Configuration

> Read for Vitest config anatomy, projects, environments, aliases, coverage, reporters, or v4 migration.

Follow the repository's existing config and runner scripts. Add configuration only when the requested behavior needs it.

## Requirements and v4 removals

| Area            | Current requirement or replacement                                                                                                                                                    |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Runtime         | Use Node 20 or newer and Vite 6 or newer. Vite 8 is supported.                                                                                                                        |
| Workspaces      | Replace `vitest.workspace.*` files with `test.projects` in a root Vitest config.                                                                                                      |
| Workers         | Replace `poolOptions`, `maxThreads`, `maxForks`, `minWorkers`, `singleThread`, and `singleFork` with `maxWorkers`; use `isolate: false` only when shared worker state is intentional. |
| Coverage        | Declare `coverage.include` explicitly. The former all-files switch, `extensions`, and `ignoreEmptyLines` are removed.                                                                 |
| Reporters       | The `basic` reporter is removed. `verbose` is flat; use `tree` for hierarchical output.                                                                                               |
| Browser imports | Import browser test APIs from `vitest/browser`, not `@vitest/browser`.                                                                                                                |

Do not copy compatibility shims into a new config. Migrate the setting and verify the affected project directly.

## Single-package config

Use `node` for pure TypeScript and server code. Use `jsdom` for web components when that matches the repository.

```typescript
// vitest.config.ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: ["./src/__tests__/setup.ts"],
  },
});
```

Override the configured environment for an exceptional file:

```typescript
// @vitest-environment node

import { expect, test } from "vitest";
```

Keep the pragma on the first line. Do not change the suite-wide environment for one test.

## Projects

List package configs through `test.projects` at the workspace root:

```typescript
// vitest.config.ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    projects: ["apps/*/vitest.config.ts", "packages/*/vitest.config.ts"],
  },
});
```

Put cross-project defaults in a shared base and merge them into named projects:

```typescript
// vitest.shared.ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    globals: false,
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
```

```typescript
// apps/web/vitest.config.ts
import { defineProject, mergeConfig } from "vitest/config";
import shared from "../../vitest.shared";

export default mergeConfig(
  shared,
  defineProject({
    test: {
      environment: "jsdom",
      name: "web",
      setupFiles: ["./src/__tests__/setup.ts"],
    },
  }),
);
```

Give every project a stable `name`; run it with `--project web`. Keep environment and setup files project-specific when
packages have different runtime assumptions.

For many structurally identical projects, use a small config factory instead of duplicating the merge:

```typescript
import { defineProject, mergeConfig } from "vitest/config";
import shared from "./vitest.shared";

export function project(name: string, environment: "node" | "jsdom") {
  return mergeConfig(shared, defineProject({ test: { environment, name } }));
}
```

Use inline project configs only when they are clearer than separate package-owned files.

## Path aliases

Mirror TypeScript `paths` in Vite's `resolve.alias`. Use identical keys without trailing slashes:

```typescript
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

export default defineConfig({
  resolve: {
    alias: {
      "@shared": fileURLToPath(new URL("./packages/shared/src", import.meta.url)),
    },
  },
});
```

Put aliases needed by multiple projects or their setup files in the shared base. A mock path must match the production
import path; do not mix aliased and relative forms for the same module.

## Running and reporters

Use the repository's narrowest script or recipe. When none exists, run Vitest directly:

```bash
nlx vitest run src/parser.test.ts
nlx vitest run src/parser.test.ts -t 'rejects invalid input'
nlx vitest run --project web
nlx vitest run packages/shared --reporter=agent
```

Use the `agent` reporter in Vitest 4.1+ for minimal agent-oriented output when the repository has no reporter
convention. Use `tree` when suite hierarchy matters; `verbose` now prints a flat list.

In watch mode, press `p` to filter by file and `t` to filter by test name. Press `f` for failed tests, `a` for all, and
`q` to quit.

For hangs or leaked timers, diagnose with `nlx vitest run --detect-async-leaks`; it is intentionally slower and should
not become the default command.

## Coverage

Add coverage only when coverage is the requested outcome. Vitest 4's V8 provider uses AST-aware remapping and requires
explicit source inclusion:

```typescript
export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      include: ["src/**/*.{ts,tsx}"],
      exclude: ["src/**/*.test.{ts,tsx}", "src/**/*.d.ts"],
      thresholds: {
        branches: 80,
        functions: 85,
        lines: 90,
        statements: 90,
      },
    },
  },
});
```

Use `coverage.changed: true` to focus on files changed since the configured base. Keep thresholds aligned with the
repository's enforcement policy; do not lower them to make a run pass.

## Console filtering

Suppress only known, asserted noise. Preserve unexpected output:

```typescript
export default defineConfig({
  test: {
    onConsoleLog(log, type) {
      if (type === "stderr" && log.includes("expected offline response")) return false;
    },
  },
});
```

Prefer assertions on the originating logger mock when output is behavior under test.

## Setup files

Use setup files for shared polyfills, DOM cleanup, matcher registration, and module mocks that truly apply to every test
in that project. Keep test data, scenario-specific mocks, network calls, and heavy initialization in owned fixtures or
tests.

If setup mutates globals, register deterministic cleanup. Import `@testing-library/jest-dom` there only when the
repository intentionally adopts its matcher surface.

Experimental performance knobs such as `experimental.fsModuleCache` and `viteModuleRunner: false` exist; introduce them
only after measuring the target suite and checking their current constraints.

---
disable-model-invocation: false
name: vitest
user-invocable: true
description: 'Use for Vitest in TypeScript React/Next.js: write, run, or debug unit/component tests, mocks, testing utilities, and coverage.'
---

# Quick Start

**Typical setup:** Vitest v4 with jsdom environment; globals enabled (`describe`, `test`, `expect`, `vi`); path aliases configured per project. In a workspace monorepo, read [references/monorepo-testing.md](references/monorepo-testing.md) for shared-vs-app test strategies, path aliases, and organization.

## Running Tests

```bash
# Run all unit tests
nlx vitest run

# Run tests matching pattern
nlx vitest run tokens

# Run specific test file
nlx vitest run src/utils/format.test.ts

# Run tests with matching name
nlx vitest run -t "adds token"

# Watch mode
nlx vitest
```

## Writing Your First Test

**File naming:** `*.test.ts` or `*.test.tsx`

**Location:** Colocate with source files

```typescript
import { describe, test, expect } from "vitest";
import { myFunction } from "./my-function";

describe("myFunction", () => {
  test("returns expected value", () => {
    expect(myFunction(5)).toBe(10);
  });
});
```

# Project-Specific Patterns

## Test Organization

Use visual separators and descriptive blocks:

```typescript
describe("TokenStore", () => {
  /* ----------------------------------------------------------------
   * Setup
   * ------------------------------------------------------------- */

  const validToken = { address: "0x123", symbol: "TEST" };

  afterEach(() => {
    // Reset state between tests
    useTokensStore.getState().clearAll();
  });

  /* ----------------------------------------------------------------
   * Adding tokens
   * ------------------------------------------------------------- */

  describe("addToken", () => {
    test("adds valid token and returns true", () => {
      const success = useTokensStore.getState().addToken(validToken);
      expect(success).toBe(true);
    });
  });
});
```

## Cleanup Pattern

Always reset state in `afterEach()`:

```typescript
import { afterEach } from "vitest";

afterEach(() => {
  // Reset mocks
  vi.clearAllMocks();

  // Reset environment
  process.env.NODE_ENV = originalEnv;

  // Reset stores
});
```

## Shared Setup File

Global mocks and configuration live in a setup file (e.g., `tests/setup.ts`):

```typescript
import { vi } from "vitest";

// Mock logger for all tests
vi.mock("@/utils/logger", () => ({
  createLogger: vi.fn(() => ({
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn()
  }))
}));
```

# Further Patterns

For component tests, complex mocking, async delays/callbacks, snapshot testing, type testing, custom matchers, and parameterized tests, read [references/testing-patterns.md](references/testing-patterns.md) when the basics above aren't enough.

For function mocks (vi.fn, spyOn), module mocks (vi.mock, vi.doMock), and timer mocks (vi.useFakeTimers), see [references/mocking.md](references/mocking.md).

# Debugging Failed Tests

## Reading Test Output

Focus on these signals:

- **File and line number** - Where the failure occurred
- **Expected vs. received** - What went wrong
- **Stack trace** - Ignore framework internals, focus on your code

## Common Failures

For known failure modes and how to recover (timeouts, async assertions, mock not called, snapshot drift, etc.), see [references/troubleshooting.md](references/troubleshooting.md).

## Debugging Tools

```bash
nlx vitest --reporter=verbose   # Detailed output
nlx vitest --ui                  # Visual debugging interface
nlx vitest --coverage            # See what's tested
nlx vitest --inspect             # Node debugger
nlx vitest --run                 # Disable watch mode
```

# Coverage Analysis

To add coverage:

```typescript
// vitest.config.ts
export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      reporter: ["text", "html", "json"],
      exclude: ["**/*.test.ts", "**/__mocks__/**", "**/node_modules/**"]
    }
  }
});
```

Run with: `nlx vitest --coverage`

# Configuration Reference

Example config: `vitest.config.ts`

```typescript
{
  environment: "jsdom",           // React/DOM APIs available
  globals: true,                  // No imports needed for describe/test/expect
  include: ["**/*.test.{js,ts,tsx}"],
  exclude: ["**/node_modules/**", "**/e2e/**"],
  setupFiles: ["./tests/setup.ts"],
  alias: {
    "@": "./src",
    // Add your project's path aliases
  },
}
```

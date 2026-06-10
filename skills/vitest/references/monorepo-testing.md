# Monorepo Testing Guide

Testing strategies for workspace monorepos with multiple apps and shared code.

## Workspace Structure

```
monorepo/
├── apps/
│   ├── web/              # Main app
│   ├── admin/            # Admin app
│   └── docs/             # Documentation app
├── packages/
│   └── shared/           # Shared code across all apps
├── package.json          # Root workspace config
├── vitest.config.ts      # Shared test config
└── tests/
    └── setup.ts          # Global test setup
```

**Key principle:** One Vitest config for all apps and shared code.

## Test Organization

### Recommended Structure

```
packages/shared/
├── tests/
│   └── setup.ts              # Global setup (runs before all tests)
├── utils/
│   ├── cn.ts
│   ├── cn.test.ts            # Colocate with source
│   ├── format.ts
│   └── format.test.ts
└── components/
    ├── Button.tsx
    └── Button.test.tsx

apps/web/
├── lib/
│   └── stores/
│       ├── __mocks__/
│       │   └── localStorage.ts    # App-specific mock
│       ├── user.ts
│       └── user.test.ts
└── app/
    └── page.test.tsx              # Page component test

apps/admin/
└── tests/                         # E2E tests (excluded from unit tests)
    └── dashboard.spec.ts
```

**Patterns:**

- **Shared code:** Colocate tests with source
- **App code:** Colocate tests with source
- **Mocks:** Keep in `__mocks__/` directory
- **Global setup:** `tests/setup.ts` or `packages/shared/tests/setup.ts`
- **E2E tests:** Separate `tests/` directory (excluded from Vitest)

### Where to Put Tests

**In `packages/shared/` if:**

- Utilities used by multiple apps
- Shared components
- Type definitions
- Constants

**In `apps/<app-name>/` if:**

- App-specific logic
- Store implementations
- Page components
- App-specific utilities

**Example:**

```typescript
// packages/shared/utils/format-currency.ts
// packages/shared/utils/format-currency.test.ts
export function formatCurrency(amount: number) { ... }

// apps/web/lib/stores/user.ts
// apps/web/lib/stores/user.test.ts
export const useUserStore = create<UserStore>(...);
```

## Path Aliases

### Configuration

**vitest.config.ts:**

```typescript
export default defineConfig({
  test: {
    alias: {
      "@shared": "./packages/shared",
      "@web": "./apps/web",
      "@admin": "./apps/admin"
    }
  }
});
```

**tsconfig.json:**

```json
{
  "compilerOptions": {
    "paths": {
      "@shared/*": ["./packages/shared/*"],
      "@web/*": ["./apps/web/*"],
      "@admin/*": ["./apps/admin/*"]
    }
  }
}
```

### Usage in Tests

**Prefer aliases over relative paths:**

```typescript
// Fragile relative paths
import { cn } from "../../../packages/shared/utils/cn";
import { useUserStore } from "../../lib/stores/user";

// Stable aliases
import { cn } from "@shared/utils/cn";
import { useUserStore } from "@web/lib/stores/user";
```

**Import from app-specific code:**

```typescript
// In apps/web/lib/stores/user.test.ts
import { useUserStore } from "@web/lib/stores/user";
import { createLocalStorageMock } from "@web/lib/stores/__mocks__/localStorage";
```

**Import shared code:**

```typescript
// Any test file
import { cn } from "@shared/utils/cn";
import { createLogger } from "@shared/utils/logger";
```

### Troubleshooting Aliases

**Import not found:**

1. Check `vitest.config.ts` alias matches import
2. Verify `tsconfig.json` paths match
3. Ensure no trailing slashes in config

```typescript
// Wrong
alias: {
  "@shared/": "./packages/shared/",  // Trailing slash
}

// Correct
alias: {
  "@shared": "./packages/shared",
}
```

## Running Tests by Scope

### All Tests

```bash
nlx vitest run                 # Run all tests (shared + all apps)
```

### Shared Tests Only

```bash
nlx vitest run packages/shared/         # All shared tests
nlx vitest run packages/shared/utils/   # Specific shared directory
nlx vitest run packages/shared/utils/cn.test.ts  # Specific file
```

### App-Specific Tests

```bash
nlx vitest run apps/web/       # All web app tests
nlx vitest run apps/admin/     # All admin app tests
```

### Pattern Matching

```bash
nlx vitest run user            # Any file matching "user"
nlx vitest run format          # Any file matching "format"
nlx vitest run stores/         # Any file in stores/ directory
```

### Name-Based Filtering

```bash
nlx vitest run -t "adds user"    # Tests with matching name
nlx vitest run -t "UserStore"    # All tests in UserStore describe blocks
```

### Combined Filters

```bash
nlx vitest run apps/web/ -t "store"  # Web tests with "store" in name
```

## Shared Setup Files

### Global Setup File

**tests/setup.ts** runs before all tests:

```typescript
import { vi } from "vitest";

// Mock logger for all tests
vi.mock("@shared/utils/logger", () => ({
  createLogger: vi.fn(() => ({
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn()
  }))
}));

// Setup global test utilities
global.testUtils = {
  // ...
};
```

**When to use:**

- Mocks needed by all tests
- Global test utilities
- Environment variable defaults
- Browser API polyfills

**When NOT to use:**

- App-specific mocks
- Test data (use factories instead)
- Heavy initialization (slows all tests)

### App-Specific Setup

Can be added per workspace:

```typescript
// vitest.config.ts
export default defineConfig({
  test: {
    setupFiles: [
      "./tests/setup.ts", // Global
      "./apps/web/tests/setup.ts" // Web-specific
    ]
  }
});
```

Or use `beforeAll` in test files:

```typescript
// apps/web/lib/stores/user.test.ts
import { beforeAll } from "vitest";

beforeAll(() => {
  // Web-specific setup
});
```

## App-Specific vs Shared Mocks

### Shared Mocks

**Location:** `tests/` or inline in `setup.ts`

**Use for:**

- Utilities used by all apps
- Browser APIs (localStorage, fetch)
- Logger, analytics

**Example:**

```typescript
// tests/setup.ts
vi.mock("@shared/utils/logger", () => ({
  createLogger: vi.fn(() => ({
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn()
  }))
}));
```

### App-Specific Mocks

**Location:** `apps/<app-name>/**/__mocks__/`

**Use for:**

- App-specific stores
- App-specific API clients
- App-specific utilities

**Example:**

```typescript
// apps/web/lib/stores/__mocks__/localStorage.ts
import { vi } from "vitest";

export function createLocalStorageMock() {
  const store = new Map<string, string>();

  return {
    getItem: vi.fn((key: string) => store.get(key) ?? null),
    setItem: vi.fn((key: string, value: string) => {
      store.set(key, value);
    }),
    removeItem: vi.fn((key: string) => {
      store.delete(key);
    }),
    clear: vi.fn(() => {
      store.clear();
    })
  };
}
```

**Usage:**

```typescript
// apps/web/lib/stores/user.test.ts
import { createLocalStorageMock } from "./__mocks__/localStorage";

const mockStorage = createLocalStorageMock();
global.localStorage = mockStorage as Storage;
```

## Cross-App Dependencies

### Testing Shared Code Used by Apps

**Scenario:** `packages/shared/utils/cn.ts` is used by all apps.

```typescript
// packages/shared/utils/cn.test.ts
import { describe, test, expect } from "vitest";
import { cn } from "./cn";

describe("cn", () => {
  test("merges class names", () => {
    expect(cn("px-4", "text-sm")).toBe("px-4 text-sm");
  });

  test("handles conditionals", () => {
    expect(cn("px-4", false && "text-sm")).toBe("px-4");
  });

  test("overrides conflicting classes", () => {
    expect(cn("px-4", "px-6")).toBe("px-6");
  });
});
```

**No need to test in each app** - test once in shared.

### Testing App Code That Uses Shared Code

**Scenario:** `apps/web/lib/stores/user.ts` uses `@shared/utils/logger`.

```typescript
// apps/web/lib/stores/user.test.ts
import { describe, test, expect, vi } from "vitest";
import { useUserStore } from "./user";
import { createLogger } from "@shared/utils/logger";

// Logger is already mocked in tests/setup.ts
// No need to mock again

describe("UserStore", () => {
  test("logs when user is added", () => {
    const logger = createLogger("test");
    useUserStore.getState().addUser({ id: "1", name: "Test" });

    expect(logger.debug).toHaveBeenCalled();
  });
});
```

## Workspace Dependencies

### Testing Peer Dependencies

Apps may depend on each other (rare but possible):

```typescript
// apps/admin/lib/use-web-feature.ts
import { useUserStore } from "@web/lib/stores/user";

// apps/admin/lib/use-web-feature.test.ts
import { useUserStore } from "@web/lib/stores/user";

// Test works because vitest.config.ts has alias configured
```

**Generally avoid cross-app imports** - use shared code instead.

## Parallel Execution

Vitest runs test files in parallel by default. Tests within a file run sequentially.

### File-Level Parallelism

```bash
nlx vitest run                    # Runs all files in parallel
nlx vitest run --no-file-parallelism  # Sequential file execution
```

### Concurrent Tests

```typescript
import { test } from "vitest";

test.concurrent("parallel test 1", async () => {
  // Runs in parallel with other concurrent tests
});

test.concurrent("parallel test 2", async () => {
  // Runs in parallel with other concurrent tests
});

test("sequential test", () => {
  // Runs after concurrent tests complete
});
```

**Use concurrent for:**

- Independent tests
- No shared state
- API calls (with mocks)

**Avoid concurrent for:**

- Tests that modify global state
- Tests that use fake timers
- Tests that depend on execution order

## Watch Mode Strategies

### Full Watch

```bash
nlx vitest                   # Watch all files
```

Vitest detects changes and re-runs affected tests.

### Scoped Watch

```bash
nlx vitest packages/shared/ --watch    # Watch only shared tests
nlx vitest apps/web/ --watch           # Watch only web tests
```

### Filter in Watch Mode

While in watch mode, press keys:

- `a` - Run all tests
- `f` - Run only failed tests
- `p` - Filter by filename pattern
- `t` - Filter by test name pattern
- `q` - Quit watch mode

### Watch with UI

```bash
nlx vitest --ui              # Watch + visual interface
```

Opens browser UI for interactive testing.

## CI/CD Considerations

### Running All Tests

```bash
nlx vitest run              # Disable watch mode (for CI)
nlx vitest run --reporter=verbose # Detailed output for CI logs
```

### Coverage in CI

```bash
nlx vitest run --coverage         # Generate coverage report
nlx vitest run --coverage --reporter=json  # JSON for tooling
```

### Workspace-Specific CI

Run tests for changed workspaces only:

```bash
# Detect changed files
CHANGED=$(git diff --name-only HEAD~1 HEAD)

# Run tests for changed workspace
if echo "$CHANGED" | grep -q "^apps/web/"; then
  nlx vitest run apps/web/
fi

if echo "$CHANGED" | grep -q "^packages/shared/"; then
  nlx vitest run packages/shared/
  # Also run app tests (shared code affects all apps)
  nlx vitest run apps/
fi
```

## Monorepo Best Practices

### DO

- Colocate tests with source files
- Use path aliases consistently
- Share setup files for common mocks
- Test shared code once, not per app
- Use pattern matching to run scoped tests
- Keep app-specific mocks in app directories
- Use factory functions for reusable mocks

### DON'T

- Create separate test directories far from source
- Use relative imports when aliases exist
- Duplicate test setup across apps
- Test shared code in multiple apps
- Run full suite when working on single app
- Put app-specific mocks in shared/
- Hardcode mock data (use factories)

## Performance Optimization

### Faster Test Runs

**1. Run only what changed:**

```bash
nlx vitest run apps/web/       # Not entire suite
```

**2. Use pattern matching:**

```bash
nlx vitest run user             # Specific feature
```

**3. Parallelize when safe:**

```typescript
test.concurrent("independent test", async () => {
  // Faster execution
});
```

**4. Avoid heavy setup:**

```typescript
// Slow: Runs for every test file
// tests/setup.ts
await seedDatabase();

// Fast: Only when needed
// specific-test.ts
beforeAll(async () => {
  await seedDatabase();
});
```

### Optimizing Watch Mode

**1. Use UI mode for debugging:**

```bash
nlx vitest --ui              # Better DX than terminal watch
```

**2. Filter aggressively:**

```bash
nlx vitest -t "critical"     # Only critical tests
```

**3. Disable coverage in dev:**

```bash
nlx vitest                   # No coverage overhead
```

## Debugging Monorepo Tests

### Import Resolution Issues

**Symptom:** `Cannot find module '@shared/utils/cn'`

**Fix:**

1. Check `vitest.config.ts` alias
2. Verify `tsconfig.json` paths
3. Restart IDE/dev server
4. Clear cache: `rm -rf node_modules/.vite`

### Mock Not Working Across Apps

**Symptom:** Shared mock in `setup.ts` not applied to app tests

**Fix:**

1. Verify mock path matches import:

```typescript
// setup.ts
vi.mock("@shared/utils/logger"); // Must use alias

// app test
import { logger } from "@shared/utils/logger"; // Same alias
```

2. Check setup file is loaded:

```typescript
// vitest.config.ts
setupFiles: ["./tests/setup.ts"],
```

### Test File Not Found

**Symptom:** `No test files found`

**Fix:**

1. Check file naming: `*.test.ts` or `*.test.tsx`
2. Verify not in exclude pattern:

```typescript
// vitest.config.ts
exclude: ["**/node_modules/**", "**/e2e/**"],  // Excludes e2e
```

3. Check running from repo root

### State Bleeding Between Tests

**Symptom:** Tests pass individually but fail when run together

**Fix:**

1. Add cleanup:

```typescript
afterEach(() => {
  vi.clearAllMocks();
  // Reset stores, globals, etc.
});
```

2. Isolate state:

```typescript
// Shared state
const store = createStore();

// Fresh state per test
beforeEach(() => {
  store = createStore();
});
```

## Next Steps

- For detailed testing patterns - See `TESTING_PATTERNS.md`
- For debugging strategies - See `TROUBLESHOOTING.md`

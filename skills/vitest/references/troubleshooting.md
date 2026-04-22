# Troubleshooting Guide

Comprehensive debugging strategies for Vitest tests in TypeScript React/Next.js projects.

## Common Errors

### Import Errors

#### Cannot find module '@shared/...'

**Cause:** Path alias not configured or misconfigured.

**Fix:**

```typescript
// vitest.config.ts
export default defineConfig({
  test: {
    alias: {
      "@shared": "./packages/shared", // Must match tsconfig.json
      "@web": "./apps/web"
    }
  }
});
```

**Verify:**

```bash
# Check both configs match
cat vitest.config.ts | grep alias
cat tsconfig.json | grep paths
```

#### Cannot find module '../../../utils/...'

**Cause:** Using relative imports instead of aliases.

**Fix:**

```typescript
// Fragile
import { cn } from "../../../packages/shared/utils/cn";

// Stable
import { cn } from "@shared/utils/cn";
```

#### Module not found during watch mode

**Cause:** Vitest cache stale after config changes.

**Fix:**

```bash
# Clear cache and restart
rm -rf node_modules/.vite
nlx vitest
```

### Mock Errors

#### Mock is not a function

**Cause:** Mock not hoisted or module path mismatch.

**Fix:**

```typescript
// Wrong path
vi.mock("./logger");
import { logger } from "@shared/utils/logger"; // Different path!

// Matching paths
vi.mock("@shared/utils/logger");
import { logger } from "@shared/utils/logger";
```

#### Mock not applied to imports

**Cause:** Mock must be at top level, before imports.

**Fix:**

```typescript
// Mock after import
import { fetchUser } from "./api";
vi.mock("./api");

// Mock before import
vi.mock("./api");
import { fetchUser } from "./api";
```

#### Need to access mock before import

**Use `vi.hoisted()`:**

```typescript
const { mockFetchUser } = vi.hoisted(() => ({
  mockFetchUser: vi.fn()
}));

vi.mock("./api", () => ({
  fetchUser: mockFetchUser
}));

// Now can configure before import
mockFetchUser.mockResolvedValue({ id: 1 });

import { fetchUser } from "./api";
```

#### Global mock not applying to all tests

**Cause:** Mock in test file overrides global mock.

**Fix:**

```typescript
// tests/setup.ts
vi.mock("@shared/utils/logger", () => ({
  createLogger: vi.fn(() => ({ debug: vi.fn() }))
}));

// In test file - don't re-mock!
// This overrides global mock
vi.mock("@shared/utils/logger");

// Just import and use
import { createLogger } from "@shared/utils/logger";
```

### State Errors

#### Tests pass individually but fail together

**Cause:** State bleeding between tests.

**Symptoms:**

- Tests fail in different order
- `--no-file-parallelism` makes tests pass
- First test always passes, second fails

**Fix:**

```typescript
import { afterEach } from "vitest";

afterEach(() => {
  // Reset mocks
  vi.clearAllMocks();
  vi.restoreAllMocks();

  // Reset stores
  useUserStore.getState().reset();

  // Reset globals
  delete global.localStorage;

  // Reset environment
  process.env.NODE_ENV = originalEnv;
});
```

**Debug:**

```bash
# Run tests sequentially to confirm
nlx vitest run --no-file-parallelism

# Run specific test file alone
nlx vitest run path/to/problem.test.ts
```

#### Store state persists between tests

**For stores:**

```typescript
import { afterEach, beforeEach } from "vitest";

let store: Store;

beforeEach(() => {
  // Create fresh instance
  store = createStore();
});

afterEach(() => {
  // Cleanup
  store.destroy();
});
```

#### Global variables polluted

**Cause:** Tests modify global scope without cleanup.

**Fix:**

```typescript
describe("localStorage tests", () => {
  const originalLocalStorage = global.localStorage;

  afterEach(() => {
    global.localStorage = originalLocalStorage;
  });

  test("mocks localStorage", () => {
    global.localStorage = createLocalStorageMock();
    // Test...
  });
});
```

### Async Errors

#### Test times out

**Symptoms:**

```
Error: Test timeout of 5000ms exceeded
```

**Causes:**

1. Unresolved promise
2. Missing await
3. Infinite loop
4. Slow operation

**Fix 1: Increase timeout**

```typescript
test("slow operation", async () => {
  await slowFunction();
}, 10000); // 10 second timeout
```

**Fix 2: Find unresolved promise**

```typescript
// Missing await
test("async test", () => {
  fetchData(); // Promise not awaited!
  expect(data).toBeDefined();
});

// Await promise
test("async test", async () => {
  const data = await fetchData();
  expect(data).toBeDefined();
});
```

**Fix 3: Check for infinite loops**

```typescript
// Infinite retry
async function fetchWithRetry() {
  while (true) {  // Never exits!
    try {
      return await fetch("/api");
    } catch {
      // Retry forever
    }
  }
}

// Limited retries
async function fetchWithRetry(maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fetch("/api");
    } catch {
      if (i === maxRetries - 1) throw;
    }
  }
}
```

#### Promise rejection not handled

**Symptom:**

```
UnhandledPromiseRejectionWarning
```

**Fix:**

```typescript
// Rejection not caught
test("failing api", async () => {
  await fetchData(); // Throws, but not caught
});

// Expect rejection
test("failing api", async () => {
  await expect(fetchData()).rejects.toThrow("Error");
});

// Try-catch
test("failing api", async () => {
  try {
    await fetchData();
    expect.fail("Should have thrown");
  } catch (error) {
    expect(error.message).toBe("Error");
  }
});
```

#### Fake timers not advancing

**Cause:** Timer not advanced or wrong timer API used.

**Fix:**

```typescript
import { vi } from "vitest";

test("debounce with timers", () => {
  vi.useFakeTimers();

  const callback = vi.fn();
  const debounced = debounce(callback, 1000);

  debounced();

  // Timer not advanced
  expect(callback).toHaveBeenCalled(); // Fails!

  // Advance timer
  vi.advanceTimersByTime(1000);
  expect(callback).toHaveBeenCalled(); // Passes

  vi.useRealTimers();
});
```

**Remember to restore:**

```typescript
afterEach(() => {
  vi.useRealTimers();
});
```

### Type Errors

#### Type error in test file

**Cause:** Mock types don't match real types.

**Fix:**

```typescript
// Wrong type
const mockFetch = vi.fn(); // Returns unknown

// Typed mock
const mockFetch = vi.fn<[string], Promise<Response>>();

// Or type assertion
const mockFetch = vi.fn() as MockedFunction<typeof fetch>;
```

#### Cannot find type definitions

**Cause:** Missing `@types` package or wrong tsconfig.

**Fix:**

```bash
# Install missing types
npm add -D @types/node

# Check tsconfig includes test files
cat tsconfig.json | grep "include"
```

```json
// tsconfig.json
{
  "include": [
    "**/*.ts",
    "**/*.tsx",
    "**/*.test.ts", // Include test files
    "**/*.test.tsx"
  ]
}
```

### Configuration Errors

#### Tests not found

**Symptoms:**

```
No test files found
```

**Causes:**

1. Wrong file naming
2. Excluded by config
3. Running from wrong directory

**Fix 1: Check file naming**

```bash
# Must match pattern
*.test.ts
*.test.tsx

# Wrong
*_test.ts
*.spec.ts  # Unless configured
```

**Fix 2: Check exclude pattern**

```typescript
// vitest.config.ts
export default defineConfig({
  test: {
    include: ["**/*.test.{js,ts,tsx}"],
    exclude: [
      "**/node_modules/**",
      "**/e2e/**" // Excludes e2e tests
    ]
  }
});
```

**Fix 3: Run from repo root**

```bash
# Wrong directory
cd apps/web
nlx vitest run

# From repo root
cd /path/to/monorepo
nlx vitest run apps/web/
```

#### Environment not jsdom

**Symptom:** `document is not defined`

**Fix:**

```typescript
// vitest.config.ts
export default defineConfig({
  test: {
    environment: "jsdom" // Required for React tests
  }
});
```

**Or per-file:**

```typescript
// @vitest-environment jsdom
import { render } from "@testing-library/react";
```

#### Globals not available

**Symptom:** `describe is not defined`

**Fix:**

```typescript
// vitest.config.ts
export default defineConfig({
  test: {
    globals: true // Makes describe/test/expect global
  }
});
```

**Or import explicitly:**

```typescript
import { describe, test, expect } from "vitest";
```

## Debugging Strategies

### Reading Test Output

**Focus on these signals:**

```
FAIL apps/web/lib/stores/user.test.ts > UserStore > addUser > validates user format
AssertionError: expected false to be true

  - Expected: true
  + Received: false

   45|   test("validates user format", () => {
   46|     const invalid = { id: "invalid" };
   47|     const result = useUserStore.getState().addUser(invalid);
 > 48|     expect(result).toBe(true);
   49|   });
```

**Key information:**

1. **File path:** `apps/web/lib/stores/user.test.ts`
2. **Test path:** `UserStore > addUser > validates user format`
3. **Error type:** `AssertionError`
4. **Expected vs Received:** `true` vs `false`
5. **Line number:** `48`

**Ignore:**

- Vitest internal stack traces
- Framework setup calls
- Node module paths

### Verbose Output

```bash
nlx vitest run --reporter=verbose
```

Shows:

- All passing tests (not just failures)
- Test execution time
- Detailed stack traces
- Console output from tests

### UI Mode

```bash
nlx vitest --ui
```

**Features:**

- Visual test tree
- Click to run individual tests
- Real-time updates
- Detailed failure info
- Console output viewer

**Best for:**

- Debugging single test
- Exploring test structure
- Interactive development

### Node Debugger

```bash
nlx vitest --inspect
```

Then open Chrome DevTools and set breakpoints.

**Or use `debugger` statement:**

```typescript
test("debug this", () => {
  const value = calculateValue();
  debugger; // Pause here
  expect(value).toBe(10);
});
```

### Console Debugging

```typescript
test("debug with console", () => {
  const store = useUserStore.getState();

  console.log("Before:", store.users);

  store.addUser(newUser);

  console.log("After:", store.users);

  expect(store.users).toHaveLength(1);
});
```

**View console output:**

```bash
nlx vitest run --reporter=verbose  # Shows console.log
```

### Isolating Failures

**Run only failing test:**

```typescript
test.only("this one test", () => {
  // Runs alone
});
```

**Skip passing tests:**

```typescript
test.skip("passing test", () => {
  // Skipped
});
```

**Run failed tests only:**

```bash
nlx vitest  # Start watch mode
# Press 'f' to run only failed tests
```

### Bisecting Test Runs

**Find which test causes failure:**

```bash
# Run first half
nlx vitest run apps/web/lib/stores/

# Run second half
nlx vitest run apps/web/app/

# Narrow down to specific file
nlx vitest run apps/web/lib/stores/user.test.ts
```

## Performance Optimization

### Slow Test Suite

**Symptoms:**

- Tests take > 30s total
- Watch mode feels sluggish
- CI times out

**Diagnose:**

```bash
nlx vitest run --reporter=verbose  # Shows test times
```

**Fixes:**

**1. Reduce setup overhead:**

```typescript
// Slow: Heavy setup runs for every test
beforeEach(async () => {
  await seedDatabase(); // Slow!
});

// Fast: Setup once per file
beforeAll(async () => {
  await seedDatabase();
});

// Fast: Or use mocks instead
beforeEach(() => {
  mockDatabase.seed(); // Fast!
});
```

**2. Parallelize independent tests:**

```typescript
test.concurrent("independent 1", async () => {
  // Runs in parallel
});

test.concurrent("independent 2", async () => {
  // Runs in parallel
});
```

**3. Avoid real timers:**

```typescript
// Slow: Real delay
test("debounce", async () => {
  debounced();
  await sleep(1000); // Slow!
  expect(callback).toHaveBeenCalled();
});

// Fast: Fake timers
test("debounce", () => {
  vi.useFakeTimers();
  debounced();
  vi.advanceTimersByTime(1000); // Instant!
  expect(callback).toHaveBeenCalled();
  vi.useRealTimers();
});
```

**4. Mock expensive operations:**

```typescript
// Slow: Real API calls
test("fetches data", async () => {
  const data = await fetch("/api/users"); // Slow!
});

// Fast: Mock API
vi.mock("./api", () => ({
  fetchUsers: vi.fn(() => Promise.resolve(mockUsers))
}));

test("fetches data", async () => {
  const data = await fetchUsers(); // Fast!
});
```

**5. Use scoped test runs:**

```bash
# Run all tests while developing
nlx vitest run

# Run only related tests
nlx vitest run user
nlx vitest run apps/web/lib/stores/
```

### Memory Issues

**Symptoms:**

- Process killed
- Out of memory errors
- Tests hang

**Fixes:**

**1. Increase Node memory:**

```bash
NODE_OPTIONS="--max-old-space-size=4096" nlx vitest run
```

**2. Clean up after tests:**

```typescript
afterEach(() => {
  vi.clearAllMocks();
  // Clean up large objects
  store.clear();
});
```

**3. Avoid memory leaks:**

```typescript
// Event listeners not removed
test("adds listener", () => {
  window.addEventListener("resize", handler);
  // Leaks!
});

// Clean up listeners
test("adds listener", () => {
  window.addEventListener("resize", handler);

  return () => {
    window.removeEventListener("resize", handler);
  };
});
```

## Coverage Issues

### Low Coverage

**Check what's missing:**

```bash
nlx vitest run --coverage
```

Opens HTML report showing uncovered lines.

**Common uncovered code:**

- Error handlers (test error paths!)
- Edge cases (test boundary conditions!)
- Default branches (test all branches!)

### Coverage Not Accurate

**Cause:** Excluded files or wrong provider.

**Fix:**

```typescript
// vitest.config.ts
export default defineConfig({
  test: {
    coverage: {
      provider: "v8", // More accurate than istanbul
      include: ["apps/**/*.ts", "packages/**/*.ts"],
      exclude: ["**/*.test.ts", "**/__mocks__/**", "**/node_modules/**", "**/*.d.ts"]
    }
  }
});
```

### Coverage Thresholds Failing

**Symptom:**

```
Coverage threshold for lines (80%) not met: 75%
```

**Options:**

**1. Add more tests** (preferred)

**2. Adjust thresholds** (if realistic)

```typescript
// vitest.config.ts
coverage: {
  lines: 75,
  functions: 75,
  branches: 75,
  statements: 75,
}
```

**3. Exclude files** (use sparingly)

```typescript
coverage: {
  exclude: [
    "**/*.config.ts",
    "**/types/**",
  ],
}
```

## CI/CD Integration

### Tests Fail in CI but Pass Locally

**Common causes:**

**1. Different Node versions:**

```yaml
# .github/workflows/test.yml
- uses: actions/setup-node@v3
  with:
    node-version: "20" # Match local version
```

**2. Different timezones:**

```typescript
// Fix: Mock dates
vi.setSystemTime(new Date("2024-01-01T00:00:00.000Z"));
```

**3. Parallelism differences:**

```bash
# CI: Disable parallelism if causing issues
nlx vitest run --no-file-parallelism
```

**4. Missing environment variables:**

```yaml
# .github/workflows/test.yml
env:
  NODE_ENV: test
  CI: true
```

**5. Race conditions:**

```typescript
// Fix: Add proper awaits
await waitFor(() => {
  expect(element).toBeInTheDocument();
});
```

### CI Performance

**Speed up CI tests:**

**1. Cache dependencies:**

```yaml
# .github/workflows/test.yml
- uses: actions/cache@v3
  with:
    path: ~/.npm
    key: ${{ runner.os }}-node-${{ hashFiles('**/package-lock.json') }}
```

**2. Run tests in parallel:**

```yaml
strategy:
  matrix:
    shard: [1, 2, 3, 4]
steps:
  - run: nlx vitest run --shard=${{ matrix.shard }}/4
```

**3. Only test changed code:**

```yaml
- run: |
    if [[ $(git diff --name-only HEAD~1 | grep "^apps/web/") ]]; then
      nlx vitest run apps/web/
    fi
```

**4. Skip coverage on PR:**

```bash
# Run coverage only on main
if [[ $GITHUB_REF == "refs/heads/main" ]]; then
  nlx vitest run --coverage
else
  nlx vitest run
fi
```

### Debugging CI Failures

**1. Enable verbose output:**

```yaml
- run: nlx vitest run --reporter=verbose
```

**2. Upload test results:**

```yaml
- uses: actions/upload-artifact@v3
  if: failure()
  with:
    name: test-results
    path: test-results/
```

**3. Run CI locally:**

```bash
# Install act: https://github.com/nektos/act
act -j test
```

**4. SSH into CI:**

```yaml
# Add to workflow for debugging
- uses: mxschmitt/action-tmate@v3
  if: failure()
```

## Next Steps

- For testing patterns - See `TESTING_PATTERNS.md`
- For monorepo strategies - See `MONOREPO_TESTING.md`

# Testing Patterns Reference

Complete pattern library for Vitest testing in TypeScript React/Next.js projects.

## Unit Testing Patterns

### Testing Pure Functions

```typescript
import { describe, test, expect } from "vitest";
import { formatCurrency } from "./format-currency";

describe("formatCurrency", () => {
  test("formats USD correctly", () => {
    expect(formatCurrency(1234.56, "USD")).toBe("$1,234.56");
  });

  test("handles zero", () => {
    expect(formatCurrency(0, "USD")).toBe("$0.00");
  });

  test("handles negative values", () => {
    expect(formatCurrency(-100, "USD")).toBe("-$100.00");
  });
});
```

### Testing Functions with Side Effects

```typescript
import { describe, test, expect, vi, afterEach } from "vitest";
import { logError } from "./error-logger";

describe("logError", () => {
  const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

  afterEach(() => {
    consoleSpy.mockClear();
  });

  test("logs error message", () => {
    logError("Test error");
    expect(consoleSpy).toHaveBeenCalledWith("Test error");
  });

  test("logs error object", () => {
    const error = new Error("Something failed");
    logError(error);
    expect(consoleSpy).toHaveBeenCalledWith(error);
  });
});
```

### Testing Utility Classes

```typescript
import { describe, test, expect, beforeEach } from "vitest";
import { Cache } from "./cache";

describe("Cache", () => {
  let cache: Cache<string>;

  beforeEach(() => {
    cache = new Cache<string>();
  });

  test("stores and retrieves values", () => {
    cache.set("key", "value");
    expect(cache.get("key")).toBe("value");
  });

  test("returns undefined for missing keys", () => {
    expect(cache.get("missing")).toBeUndefined();
  });

  test("overwrites existing values", () => {
    cache.set("key", "first");
    cache.set("key", "second");
    expect(cache.get("key")).toBe("second");
  });
});
```

## Mocking Patterns

### Basic Function Mocking

```typescript
import { vi } from "vitest";

// Create mock function
const mockFn = vi.fn();

// With implementation
const mockAdd = vi.fn((a: number, b: number) => a + b);

// With return value
const mockGetUser = vi.fn().mockReturnValue({ id: 1, name: "Test" });

// With resolved promise
const mockFetch = vi.fn().mockResolvedValue({ data: "test" });

// With rejected promise
const mockFailingFetch = vi.fn().mockRejectedValue(new Error("Network error"));

// Assertions
expect(mockFn).toHaveBeenCalled();
expect(mockFn).toHaveBeenCalledTimes(2);
expect(mockFn).toHaveBeenCalledWith("arg1", "arg2");
expect(mockFn).toHaveReturnedWith("value");
```

### Spying on Methods

```typescript
import { vi } from "vitest";

test("spy on object method", () => {
  const user = {
    name: "John",
    greet: () => `Hello, ${this.name}`
  };

  const greetSpy = vi.spyOn(user, "greet");

  user.greet();

  expect(greetSpy).toHaveBeenCalled();
  greetSpy.mockRestore(); // Restore original implementation
});

test("spy with mock implementation", () => {
  const spy = vi.spyOn(console, "log").mockImplementation(() => {});

  console.log("test");

  expect(spy).toHaveBeenCalledWith("test");
  spy.mockRestore();
});
```

### Module Mocking

**Automatic mock:**

```typescript
// At top level, before imports
vi.mock("./api-client");

import { fetchUser } from "./api-client";

test("uses automatic mock", () => {
  // All exports are mocked automatically
  expect(vi.isMockFunction(fetchUser)).toBe(true);
});
```

**Manual mock:**

```typescript
vi.mock("./api-client", () => ({
  fetchUser: vi.fn(() => Promise.resolve({ id: 1, name: "Test" })),
  deleteUser: vi.fn(() => Promise.resolve())
}));

import { fetchUser, deleteUser } from "./api-client";

test("uses manual mock", async () => {
  const user = await fetchUser();
  expect(user.name).toBe("Test");
});
```

**Partial mock:**

```typescript
import * as apiClient from "./api-client";

vi.spyOn(apiClient, "fetchUser").mockResolvedValue({
  id: 1,
  name: "Test"
});

// Other exports work normally
```

**Factory mock with `vi.hoisted()`:**

```typescript
const { mockFetchUser } = vi.hoisted(() => ({
  mockFetchUser: vi.fn()
}));

vi.mock("./api-client", () => ({
  fetchUser: mockFetchUser
}));

test("can access mock before import", () => {
  mockFetchUser.mockResolvedValue({ id: 1 });
  // Now import and use
});
```

### Factory Mock Pattern

**Create reusable mock factories:**

```typescript
// __mocks__/create-api-mock.ts
import { vi } from "vitest";

export function createApiMock() {
  return {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn()
  };
}

// In test file
import { createApiMock } from "./__mocks__/create-api-mock";

describe("API client", () => {
  let api: ReturnType<typeof createApiMock>;

  beforeEach(() => {
    api = createApiMock();
  });

  test("makes GET request", async () => {
    api.get.mockResolvedValue({ data: "test" });

    const response = await api.get("/users");
    expect(response.data).toBe("test");
  });
});
```

**Complex factory with state:**

```typescript
// __mocks__/create-storage-mock.ts
import { vi } from "vitest";

export function createStorageMock() {
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
    }),
    get size() {
      return store.size;
    }
  };
}
```

## Async Testing Patterns

### Testing Promises

```typescript
test("resolves with value", async () => {
  const result = await Promise.resolve("success");
  expect(result).toBe("success");
});

test("rejects with error", async () => {
  await expect(Promise.reject(new Error("failed"))).rejects.toThrow("failed");
});

test("async function resolves", async () => {
  async function fetchData() {
    return { data: "value" };
  }

  const result = await fetchData();
  expect(result).toEqual({ data: "value" });
});
```

### Testing Async Functions with Delays

```typescript
import { vi } from "vitest";

test("waits for async operation", async () => {
  const delayedFunction = async () => {
    await new Promise((resolve) => setTimeout(resolve, 100));
    return "done";
  };

  const result = await delayedFunction();
  expect(result).toBe("done");
});

test("uses fake timers for delays", async () => {
  vi.useFakeTimers();

  const promise = new Promise((resolve) => {
    setTimeout(() => resolve("done"), 1000);
  });

  vi.advanceTimersByTime(1000);

  const result = await promise;
  expect(result).toBe("done");

  vi.useRealTimers();
});
```

### Testing Callbacks

```typescript
test("callback is called", (done) => {
  function asyncOperation(callback: (result: string) => void) {
    setTimeout(() => callback("success"), 10);
  }

  asyncOperation((result) => {
    expect(result).toBe("success");
    done(); // Signal test completion
  });
});

// Or use promises (preferred)
test("callback is called with promise", async () => {
  function asyncOperation(callback: (result: string) => void) {
    setTimeout(() => callback("success"), 10);
  }

  const result = await new Promise((resolve) => {
    asyncOperation(resolve);
  });

  expect(result).toBe("success");
});
```

### Testing Error Handling

```typescript
test("handles async errors", async () => {
  async function failingFunction() {
    throw new Error("Something went wrong");
  }

  await expect(failingFunction()).rejects.toThrow("Something went wrong");
});

test("handles try-catch", async () => {
  async function withErrorHandling() {
    try {
      throw new Error("Error");
    } catch (error) {
      return "handled";
    }
  }

  const result = await withErrorHandling();
  expect(result).toBe("handled");
});
```

## Timer and Date Mocking

### Fake Timers

```typescript
import { vi } from "vitest";

test("debounce function", () => {
  vi.useFakeTimers();

  const callback = vi.fn();
  const debounced = debounce(callback, 1000);

  debounced();
  expect(callback).not.toHaveBeenCalled();

  vi.advanceTimersByTime(500);
  expect(callback).not.toHaveBeenCalled();

  vi.advanceTimersByTime(500);
  expect(callback).toHaveBeenCalledTimes(1);

  vi.useRealTimers();
});

test("throttle function", () => {
  vi.useFakeTimers();

  const callback = vi.fn();
  const throttled = throttle(callback, 1000);

  throttled();
  throttled();
  throttled();

  expect(callback).toHaveBeenCalledTimes(1);

  vi.advanceTimersByTime(1000);
  throttled();

  expect(callback).toHaveBeenCalledTimes(2);

  vi.useRealTimers();
});
```

### System Time Mocking

```typescript
import { vi } from "vitest";

test("mocks system time", () => {
  const mockDate = new Date("2024-01-01T00:00:00.000Z");
  vi.setSystemTime(mockDate);

  expect(new Date().toISOString()).toBe("2024-01-01T00:00:00.000Z");
  expect(Date.now()).toBe(mockDate.getTime());

  vi.useRealTimers();
});

test("advances time", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2024-01-01"));

  const start = Date.now();

  vi.advanceTimersByTime(1000); // Advance 1 second

  const end = Date.now();
  expect(end - start).toBe(1000);

  vi.useRealTimers();
});
```

### Testing Intervals

```typescript
import { vi } from "vitest";

test("interval callback", () => {
  vi.useFakeTimers();

  const callback = vi.fn();
  setInterval(callback, 1000);

  vi.advanceTimersByTime(2500);

  expect(callback).toHaveBeenCalledTimes(2);

  vi.clearAllTimers();
  vi.useRealTimers();
});
```

## Component Testing Setup

To add React Testing Library:

```bash
npm add -D @testing-library/react @testing-library/user-event
```

### Basic Component Test

```typescript
import { render, screen } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { describe, test, expect, vi } from "vitest";
import { Button } from "./Button";

describe("Button", () => {
  test("renders children", () => {
    render(<Button>Click me</Button>);
    expect(screen.getByRole("button")).toHaveTextContent("Click me");
  });

  test("calls onClick handler", async () => {
    const user = userEvent.setup();
    const handleClick = vi.fn();

    render(<Button onClick={handleClick}>Click</Button>);

    await user.click(screen.getByRole("button"));

    expect(handleClick).toHaveBeenCalledTimes(1);
  });

  test("is disabled when disabled prop is true", () => {
    render(<Button disabled>Click</Button>);
    expect(screen.getByRole("button")).toBeDisabled();
  });
});
```

### Testing with Props

```typescript
import { render, screen } from "@testing-library/react";

test("renders with variant", () => {
  render(<Button variant="primary">Click</Button>);

  const button = screen.getByRole("button");
  expect(button).toHaveClass("bg-primary");
});

test("renders with custom className", () => {
  render(<Button className="custom-class">Click</Button>);

  const button = screen.getByRole("button");
  expect(button).toHaveClass("custom-class");
});
```

### Testing User Interactions

```typescript
import { render, screen } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";

test("input value changes on type", async () => {
  const user = userEvent.setup();

  render(<input type="text" />);

  const input = screen.getByRole("textbox");

  await user.type(input, "Hello");

  expect(input).toHaveValue("Hello");
});

test("form submission", async () => {
  const user = userEvent.setup();
  const handleSubmit = vi.fn((e) => e.preventDefault());

  render(
    <form onSubmit={handleSubmit}>
      <input name="username" />
      <button type="submit">Submit</button>
    </form>,
  );

  await user.type(screen.getByRole("textbox"), "testuser");
  await user.click(screen.getByRole("button"));

  expect(handleSubmit).toHaveBeenCalledTimes(1);
});
```

### Testing Async Components

```typescript
import { render, screen, waitFor } from "@testing-library/react";

test("shows loading state then data", async () => {
  render(<UserProfile userId="1" />);

  expect(screen.getByText("Loading...")).toBeInTheDocument();

  await waitFor(() => {
    expect(screen.getByText("John Doe")).toBeInTheDocument();
  });
});

test("handles error state", async () => {
  // Mock API to fail
  vi.mock("./api", () => ({
    fetchUser: vi.fn().mockRejectedValue(new Error("Failed")),
  }));

  render(<UserProfile userId="1" />);

  await waitFor(() => {
    expect(screen.getByText("Error loading user")).toBeInTheDocument();
  });
});
```

### Query Priorities

Use queries in this order:

1. **getByRole** - Accessible to everyone

```typescript
screen.getByRole("button", { name: /submit/i });
screen.getByRole("textbox", { name: /username/i });
```

2. **getByLabelText** - For form elements

```typescript
screen.getByLabelText("Username");
```

3. **getByPlaceholderText** - If no label

```typescript
screen.getByPlaceholderText("Enter username");
```

4. **getByText** - For non-interactive content

```typescript
screen.getByText("Welcome");
```

5. **getByTestId** - Last resort

```typescript
screen.getByTestId("user-profile");
```

### Custom Render with Providers

```typescript
// test-utils.tsx
import { render, RenderOptions } from "@testing-library/react";
import { ReactElement } from "react";

function AllProviders({ children }: { children: React.ReactNode }) {
  return (
    <ThemeProvider>
      <QueryClientProvider client={queryClient}>
        {children}
      </QueryClientProvider>
    </ThemeProvider>
  );
}

export function renderWithProviders(
  ui: ReactElement,
  options?: Omit<RenderOptions, "wrapper">,
) {
  return render(ui, { wrapper: AllProviders, ...options });
}

// In tests
import { renderWithProviders } from "./test-utils";

test("component with providers", () => {
  renderWithProviders(<MyComponent />);
  // ...
});
```

## Snapshot Testing

```typescript
import { render } from "@testing-library/react";

test("matches snapshot", () => {
  const { container } = render(<Button>Click me</Button>);
  expect(container.firstChild).toMatchSnapshot();
});

// Update snapshots with: nlx vitest -u
```

**Use sparingly:** Snapshots are brittle. Prefer targeted assertions.

## Type Testing

Vitest can validate TypeScript types:

```typescript
import { expectTypeOf } from "vitest";

test("type checks", () => {
  expectTypeOf({ a: 1 }).toMatchTypeOf<{ a: number }>();
  expectTypeOf([1, 2, 3]).toEqualTypeOf<number[]>();
  expectTypeOf<string>().toBeString();
  expectTypeOf<number>().toBeNumber();
});
```

## Custom Matchers

Extend expect with custom matchers:

```typescript
import { expect } from "vitest";

expect.extend({
  toBeWithinRange(received: number, min: number, max: number) {
    const pass = received >= min && received <= max;
    return {
      pass,
      message: () => `expected ${received} to be within range ${min} - ${max}`
    };
  }
});

// Usage
test("custom matcher", () => {
  expect(5).toBeWithinRange(1, 10);
});
```

## Parameterized Tests

```typescript
import { describe, test, expect } from "vitest";

describe.each([
  { input: 1, expected: 2 },
  { input: 2, expected: 4 },
  { input: 3, expected: 6 }
])("double($input)", ({ input, expected }) => {
  test(`returns ${expected}`, () => {
    expect(double(input)).toBe(expected);
  });
});

// Or with test.each
test.each([
  [1, 2],
  [2, 4],
  [3, 6]
])("double(%i) returns %i", (input, expected) => {
  expect(double(input)).toBe(expected);
});
```

## Global Test Patterns

### Setup and Teardown

```typescript
import { beforeAll, afterAll, beforeEach, afterEach } from "vitest";

beforeAll(() => {
  // Runs once before all tests in this file
});

afterAll(() => {
  // Runs once after all tests in this file
});

beforeEach(() => {
  // Runs before each test
});

afterEach(() => {
  // Runs after each test
});
```

### Test Lifecycle

```typescript
describe("nested describe blocks", () => {
  beforeAll(() => console.log("1 - beforeAll"));
  afterAll(() => console.log("1 - afterAll"));
  beforeEach(() => console.log("1 - beforeEach"));
  afterEach(() => console.log("1 - afterEach"));

  test("test 1", () => console.log("1 - test"));

  describe("nested", () => {
    beforeAll(() => console.log("2 - beforeAll"));
    afterAll(() => console.log("2 - afterAll"));
    beforeEach(() => console.log("2 - beforeEach"));
    afterEach(() => console.log("2 - afterEach"));

    test("test 2", () => console.log("2 - test"));
  });
});

// Output:
// 1 - beforeAll
// 2 - beforeAll
// 1 - beforeEach
// 1 - test
// 1 - afterEach
// 1 - beforeEach
// 2 - beforeEach
// 2 - test
// 2 - afterEach
// 1 - afterEach
// 2 - afterAll
// 1 - afterAll
```

## Test Filtering

```typescript
// Run only this test
test.only("focused test", () => {
  // ...
});

// Skip this test
test.skip("skipped test", () => {
  // ...
});

// Skip conditionally
test.skipIf(process.env.CI)("runs only locally", () => {
  // ...
});

// Run conditionally
test.runIf(process.platform === "darwin")("runs only on macOS", () => {
  // ...
});

// Concurrent tests
test.concurrent("runs in parallel 1", async () => {
  // ...
});

test.concurrent("runs in parallel 2", async () => {
  // ...
});
```

## Next Steps

- For monorepo-specific testing - See `MONOREPO_TESTING.md`
- For debugging strategies - See `TROUBLESHOOTING.md`

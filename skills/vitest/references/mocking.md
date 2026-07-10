# Mocking

> When to read: when writing tests that mock functions, modules, or timers.

## Mocking Functions

```typescript
import { vi } from "vitest";

// Mock a function
const mockCallback = vi.fn((x: number) => x * 2);
mockCallback(5);
expect(mockCallback).toHaveBeenCalledWith(5);
expect(mockCallback).toHaveReturnedWith(10);

// Spy on object method
const spy = vi.spyOn(console, "log").mockImplementation(() => {});
console.log("test");
expect(spy).toHaveBeenCalledWith("test");
spy.mockRestore();
```

## Mocking Modules

```typescript
// At top level, before imports
vi.mock("./api-client", () => ({
  fetchUser: vi.fn(() => Promise.resolve({ id: 1, name: "Test" })),
}));

import { fetchUser } from "./api-client";

test("uses mocked API", async () => {
  const user = await fetchUser();
  expect(user.name).toBe("Test");
});
```

## Timer Mocking

```typescript
import { vi } from "vitest";

test("debounced function", () => {
  vi.useFakeTimers();

  const callback = vi.fn();
  const debounced = debounce(callback, 1000);

  debounced();
  debounced();
  debounced();

  vi.advanceTimersByTime(1000);
  expect(callback).toHaveBeenCalledTimes(1);

  vi.useRealTimers();
});
```

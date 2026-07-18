# Mocking

> Read when writing function mocks, module mocks, timer tests, or environment stubs.

## Functions and spies

Type mocks with a function signature. Configure the narrowest behavior the test needs.

```typescript
import { expect, test, vi } from "vitest";

const fetchMock = vi.fn<(url: string) => Promise<Response>>();

test("handles a response", async () => {
  fetchMock.mockResolvedValue(new Response("ok"));

  await fetchMock("/health");

  expect(fetchMock).toHaveBeenCalledWith("/health");
});
```

Use `mockReturnValue`, `mockResolvedValue`, and `mockRejectedValue` for persistent behavior; append `Once` for a single
call. Vitest 4.1 also provides `mockThrow` and `mockThrowOnce` for synchronous failures:

```typescript
const parseMock = vi.fn<(input: string) => object>();

parseMock.mockReturnValue({ ok: true });
parseMock.mockThrowOnce(new SyntaxError("invalid payload"));
parseMock.mockThrow(new Error("parser unavailable"));
```

Spy when the real object should remain in place:

```typescript
const writeSpy = vi.spyOn(logger, "write").mockImplementation(() => {});
const tokenSpy = vi.spyOn(session, "token", "get").mockReturnValue("test-token");

expect(session.token).toBe("test-token");
expect(writeSpy).not.toHaveBeenCalled();
```

`vi.spyOn` also works on constructor exports in Vitest 4. Prefer dependency injection when it makes the production
interface simpler; otherwise restore the spy after the test.

## Module mocks

Declare module mocks at file scope. Use `vi.hoisted` for mock handles referenced by a hoisted `vi.mock` factory:

```typescript
import { afterEach, expect, test, vi } from "vitest";
import { loadUser } from "./service";

const { fetchUserMock } = vi.hoisted(() => ({
  fetchUserMock: vi.fn<(id: string) => Promise<{ id: string; name: string }>>(),
}));

vi.mock("./api", () => ({
  fetchUser: fetchUserMock,
}));

afterEach(() => {
  fetchUserMock.mockReset();
});

test("loads a user", async () => {
  fetchUserMock.mockResolvedValue({ id: "1", name: "Ada" });
  await expect(loadUser("1")).resolves.toEqual({ id: "1", name: "Ada" });
});
```

For a typed partial mock, preserve the real exports through `importOriginal`:

```typescript
const { auditMock } = vi.hoisted(() => ({ auditMock: vi.fn() }));

vi.mock("./audit", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./audit")>()),
  audit: auditMock,
}));
```

Use `vi.mock("./dependency")` when Vitest's automatic mock is sufficient. The path passed to `vi.mock` must match the
specifier used by the production import; visually similar relative and aliased paths are different module identities.

Put reusable mock factories under the repository's established shared-test location, commonly `__tests__/`:

```typescript
import { vi } from "vitest";

export function createStorageMock() {
  const values = new Map<string, string>();

  return {
    clear: vi.fn(() => values.clear()),
    getItem: vi.fn((key: string) => values.get(key) ?? null),
    removeItem: vi.fn((key: string) => values.delete(key)),
    setItem: vi.fn((key: string, value: string) => values.set(key, value)),
  };
}
```

Create a fresh stateful factory result per test. Keep one-off module wiring in the test file instead of global setup.

## Timers and dates

Install fake timers before scheduling work. Prefer async advancement when a timer callback schedules promises.

```typescript
import { afterEach, expect, test, vi } from "vitest";

afterEach(() => {
  vi.useRealTimers();
});

test("debounces writes", async () => {
  vi.useFakeTimers();
  const write = vi.fn();
  const debouncedWrite = debounce(write, 250);

  debouncedWrite("first");
  debouncedWrite("second");
  await vi.advanceTimersByTimeAsync(250);

  expect(write).toHaveBeenCalledOnce();
  expect(write).toHaveBeenCalledWith("second");
});
```

Use `vi.advanceTimersByTime` when the callback is entirely synchronous. Pin wall-clock behavior with system time:

```typescript
test("expires after midnight", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2026-01-01T23:59:59Z"));

  const expiresAt = nextMidnight();

  expect(expiresAt.toISOString()).toBe("2026-01-02T00:00:00.000Z");
});
```

Do not mix fake timers with uncontrolled real-time waits. Restore real timers even when an assertion fails.

## Environment and globals

Use `vi.stubEnv` instead of assigning `process.env`; restore all stubs after each test.

```typescript
import { afterEach, expect, test, vi } from "vitest";

afterEach(() => {
  vi.unstubAllEnvs();
});

test("uses the configured endpoint", () => {
  vi.stubEnv("API_URL", "https://api.test");
  expect(getApiUrl()).toBe("https://api.test");
});
```

When replacing a global property, preserve its descriptor so getters, setters, writability, and configurability are
restored exactly:

```typescript
const localStorageDescriptor = Object.getOwnPropertyDescriptor(globalThis, "localStorage")!;

afterEach(() => {
  Object.defineProperty(globalThis, "localStorage", localStorageDescriptor);
});

test("writes preferences", () => {
  const storage = createStorageMock();
  Object.defineProperty(globalThis, "localStorage", { configurable: true, value: storage });

  savePreference("theme", "dark");
  expect(storage.setItem).toHaveBeenCalledWith("theme", "dark");
});
```

## Cleanup

Follow the repository's cleanup convention. A typical file restores manual spies, environment stubs, and timers, while
resetting owned module mocks explicitly:

```typescript
afterEach(() => {
  fetchUserMock.mockReset();
  vi.restoreAllMocks();
  vi.unstubAllEnvs();
  vi.useRealTimers();
});
```

In Vitest 4, `vi.restoreAllMocks()` restores only manually created spies; it does not restore automocks. Reset or
reconfigure automocked exports explicitly. Use `test.mockReset: true` in config only when that matches the repository's
suite-wide convention.

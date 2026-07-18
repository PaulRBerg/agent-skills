# Testing patterns

> Read for component tests, async boundaries, fixtures, tables, snapshots, type tests, filtering, or tags.

Import APIs explicitly from `vitest` even when the repository enables globals. Match the repository's colocated test
naming, commonly `.test.ts` and `.test.tsx`.

## Component tests

Test accessible behavior through the DOM and user interactions:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, test, vi } from "vitest";
import { Button } from "./Button";

test("submits once", async () => {
  const user = userEvent.setup();
  const onSubmit = vi.fn();

  render(<Button onClick={onSubmit}>Submit</Button>);
  const button = screen.getByRole("button", { name: "Submit" });
  await user.click(button);

  expect(button.textContent).toBe("Submit");
  expect(onSubmit).toHaveBeenCalledOnce();
});
```

Use plain DOM assertions unless setup imports `@testing-library/jest-dom`:

```typescript
expect(button.disabled).toBe(true);
expect(button.classList.contains("primary")).toBe(true);
expect(message.textContent).toContain("Saved");
```

For async rendering, use `findBy*` for one awaited element and `waitFor` for an assertion that must be retried. Wire
module failures at file scope, not inside a test:

```tsx
import { render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { UserProfile } from "./UserProfile";

const { fetchUserMock } = vi.hoisted(() => ({
  fetchUserMock: vi.fn(),
}));

vi.mock("./api", () => ({ fetchUser: fetchUserMock }));

test("renders the error state", async () => {
  fetchUserMock.mockRejectedValueOnce(new Error("offline"));
  render(<UserProfile userId="1" />);

  const alert = await screen.findByRole("alert");
  expect(alert.textContent).toContain("Could not load user");
});
```

Prefer queries by role or label, then visible text, then semantic test IDs as a last resort. Avoid selectors tied to
markup structure or styling.

Centralize providers only when multiple tests need the same wrapper:

```tsx
import { render, type RenderOptions } from "@testing-library/react";
import type { ReactElement, ReactNode } from "react";

function Providers({ children }: { children: ReactNode }) {
  return <ThemeProvider theme={testTheme}>{children}</ThemeProvider>;
}

export function renderWithProviders(ui: ReactElement, options?: Omit<RenderOptions, "wrapper">) {
  return render(ui, { wrapper: Providers, ...options });
}
```

Keep provider state fresh per render. Do not hide scenario-specific data or mocks in the render helper.

## Async boundaries and timeouts

Return or await every promise. Convert callback APIs to a promise; the first test callback argument is Vitest's test
context, not a completion callback.

```typescript
test("receives the result", async () => {
  const result = await new Promise<string>((resolve, reject) => {
    legacyOperation((error, value) => (error ? reject(error) : resolve(value)));
  });

  expect(result).toBe("ready");
});

test("slow integration", { timeout: 10_000 }, async () => {
  await runIntegration();
});
```

Pass test options as the second argument. The positional third-argument timeout form is not valid in Vitest 4.

## Fixtures and lifecycle wrappers

Use `test.extend` for reusable setup with ownership and cleanup. Vitest 4.1's builder pattern infers fixture types:

```typescript
import { expect, test as baseTest } from "vitest";

const test = baseTest
  .extend("config", { baseUrl: "https://api.test" })
  .extend("client", async ({ config }, { onCleanup }) => {
    const client = createClient(config.baseUrl);
    onCleanup(() => client.close());
    return client;
  });

test("loads health", async ({ client }) => {
  await expect(client.health()).resolves.toEqual({ ok: true });
});
```

Retain object-style `test.extend<Fixtures>({...})` when established locally. Put shared factories and fixture builders
in the repository's shared-test location, commonly a `__tests__/` directory; keep scenario data beside its test.

Use `aroundEach` or `aroundAll` when the test or suite must execute inside a transaction, trace, or async context. The
hook must invoke its `runTest` or `runSuite` callback exactly once:

```typescript
test.aroundEach(async (runTest, { client }) => {
  await client.transaction(runTest);
});
```

## Tables and fixtures

Use `test.each` for positional tables and `test.for` when the test needs the case as one typed value:

```typescript
test.each([
  [0, 0],
  [2, 4],
  [-2, -4],
])("double(%i) returns %i", (input, expected) => {
  expect(double(input)).toBe(expected);
});

test.for([
  { input: " A ", expected: "a" },
  { input: "B", expected: "b" },
])("normalizes $input", ({ input, expected }) => {
  expect(normalize(input)).toBe(expected);
});
```

Name cases so failures identify the input without reading the implementation.

## Type tests, matchers, and snapshots

Use type assertions for public type contracts:

```typescript
import { expectTypeOf, test } from "vitest";

test("result type stays compatible", () => {
  expectTypeOf<Result>().toExtend<BaseResult>();
  expectTypeOf({ id: 1, name: "Ada" }).toMatchObjectType<{ id: number }>();
  expectTypeOf<string>().not.toEqualTypeOf<number>();
});
```

Add a custom matcher only when it makes repeated domain assertions clearer:

```typescript
expect.extend({
  toBeWithinRange(value: number, min: number, max: number) {
    const pass = value >= min && value <= max;
    return { pass, message: () => `expected ${value} to be within ${min}..${max}` };
  },
});
```

Use snapshots sparingly for stable, reviewable output. Prefer inline snapshots for short values and focused assertions
for behavior. Update intentionally with the repository's script or `nlx vitest run -u` as a fallback.

## Selection and tags

Use `only` temporarily; do not commit it. Use `skip`, `skipIf`, `runIf`, and `concurrent` only when their condition or
parallel-safety is explicit:

```typescript
test.skipIf(process.platform === "win32")("uses Unix permissions", () => {});
test.runIf(Boolean(process.env.RUN_INTEGRATION))("calls the sandbox", async () => {});
test.concurrent("parses independently", async () => {});
```

Vitest 4.1 tags must first be declared in `test.tags`. Attach them through the options object and filter with an
expression:

```typescript
test("persists a user", { tags: ["db", "integration"] }, async () => {});
```

```bash
nlx vitest run --tags-filter='integration && !flaky'
```

## Browser mode

If the repository already uses browser mode, import browser APIs from `vitest/browser`, configure `playwright()` from
`@vitest/browser-playwright`, and use `toMatchScreenshot` for intentional visual assertions. Do not introduce browser
mode solely for component tests that the existing jsdom setup covers.

# Effect Atom

Effect Atom separates core atoms from framework bindings. The `@effect-atom/*` packages are v3-only. Do not mix them
with Effect 4. Verify APIs against the installed packages:

- core `Atom`, `AsyncResult`, `AtomRegistry`, `AtomRef`, `AtomRpc`, `AtomHttpApi`, and `Hydration`: `effect/reactivity`
  (`@stability unstable`).
- React hooks, `RegistryProvider`, and `HydrationBoundary`: `@effect/atom-react` (also `@effect/atom-solid`,
  `@effect/atom-vue`), at the same version as `effect`.

```ts
import { AsyncResult, Atom } from "effect/reactivity";
import { RegistryProvider, useAtomSet, useAtomValue } from "@effect/atom-react";
```

## Atom Semantics

- `Atom.make(value)` creates writable state. `Atom.make(get => value)` creates derived state.
- An Effect or Stream passed to `Atom.make` produces an `AsyncResult` (v3 `Result`), not the raw success value.
- Use `Atom.family` for stable parameterized atoms and `Atom.keepAlive` only when state must outlive component mounts.
- When atoms need an Effect runtime with services, use `Atom.runtime(layer)`. Its `atom`, `fn`, and `pull` accept
  Effects requiring those services.
- `Atom.fn` creates a writable Effect/Stream function. Its handler receives the written argument and an `FnContext`.
- Use `get.addFinalizer` or a scoped Effect for listeners and resources owned by an atom.

## React Boundaries

Use `useAtomValue` to read and `useAtomSet` to write. For `AsyncResult`-backed mutation atoms, select
`mode: "promiseExit"` when the caller must branch on typed success or failure. Do not discard the `Exit` merely to mimic
an untyped async callback.

Render `AsyncResult` states explicitly (`AsyncResult.match` or `AsyncResult.builder`), including initial/waiting and
failure. Use `useAtomSuspense` only when the surrounding React boundary is designed to suspend or surface failures.

Inspect `AtomRpc`, `AtomHttpApi`, and hydration modules only when the task uses them. For ordinary state work, do not
load their APIs.

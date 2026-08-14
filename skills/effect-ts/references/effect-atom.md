# Effect Atom

Effect Atom separates core atoms from framework bindings. Verify APIs against the installed packages:

- core constructors, Registry, Result, RPC, and HTTP integrations: `@effect-atom/atom/*`;
- React hooks and Registry provider: `@effect-atom/atom-react`.

```ts
import * as Atom from "@effect-atom/atom/Atom";
import * as Result from "@effect-atom/atom/Result";
import { RegistryProvider, useAtomSet, useAtomValue } from "@effect-atom/atom-react";
```

## Atom Semantics

- `Atom.make(value)` creates writable state; `Atom.make(get => value)` creates derived state.
- An Effect or Stream passed to `Atom.make` produces a `Result`, not the raw success value.
- Use `Atom.family` for stable parameterized atoms and `Atom.keepAlive` only when state must outlive component mounts.
- Use `Atom.runtime(layer)` when atoms need an Effect runtime with services.
- `Atom.fn` creates a writable Effect/Stream function. Its handler receives the written argument and atom context.
- Use `get.addFinalizer` or a scoped Effect for listeners and resources owned by an atom.

## React Boundaries

Use `useAtomValue` to read and `useAtomSet` to write. For Effect-backed mutation atoms, select `mode: "promiseExit"`
when the caller must branch on typed success or failure; do not throw away the `Exit` merely to mimic an untyped async
callback.

Render `Result` states explicitly, including initial/waiting and failure. Use suspense hooks only when the surrounding
React boundary is designed to suspend or surface failures.

Inspect `AtomRpc`, `AtomHttpApi`, and hydration modules only when the task uses them; do not load their APIs for
ordinary state work.

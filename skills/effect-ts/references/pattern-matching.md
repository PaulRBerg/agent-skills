# Pattern Matching (Match Module)

> When to read: pull this in when you need Effect's Match module to handle tagged unions, replace nested catchTag chains, or pattern-match on values exhaustively.

**Default branching tool for tagged unions and complex conditionals.**

```typescript
import { Match } from "effect"

// Type-safe exhaustive matching on tagged errors
const handleError = Match.type<AppError>().pipe(
  Match.tag("UserCancelledError", () => null),          // Expected rejection
  Match.tag("ValidationError", (e) => e.message),       // Domain error
  Match.tag("NetworkError", () => "Connection failed"), // Retryable
  Match.exhaustive  // Compile error if case missing
)

// Replace nested catchTag chains
// BEFORE: effect.pipe(catchTag("A", ...), catchTag("B", ...), catchTag("C", ...))
// AFTER:
Effect.catchAll(effect, (error) =>
  Match.value(error).pipe(
    Match.tag("A", handleA),
    Match.tag("B", handleB),
    Match.tag("C", handleC),
    Match.exhaustive
  )
)

// Match on values (cleaner than if/else)
const describe = Match.value(status).pipe(
  Match.when("pending", () => "Loading..."),
  Match.when("success", () => "Done!"),
  Match.orElse(() => "Unknown")
)
```

## `Data.taggedEnum` Matching

Use the constructor `$match` helper for `Data.taggedEnum` unions when you want exhaustiveness and variant-specific
payload types without casts.

As of `effect@3.21.3`, `$match(value, cases)` preserves generic type parameters inside each arm for generic tagged
enums.

```typescript
import { Data } from "effect"

type Tree<A> = Data.TaggedEnum<{
  Leaf: { readonly value: A }
  Branch: { readonly children: ReadonlyArray<Tree<A>> }
}>

interface TreeDefinition extends Data.TaggedEnum.WithGenerics<1> {
  readonly taggedEnum: Tree<this["A"]>
}

const Tree = Data.taggedEnum<TreeDefinition>()

const collect = <A>(tree: Tree<A>): ReadonlyArray<A> =>
  Tree.$match(tree, {
    Leaf: (leaf) => [leaf.value],
    Branch: (branch) => branch.children.flatMap(collect<A>)
  })
```

Prefer this over `switch` plus `as` casts when recursive generic variants are involved.

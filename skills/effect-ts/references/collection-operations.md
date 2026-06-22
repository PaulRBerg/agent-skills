# Collection Operations (Array, Record, Order)

> When to read: pull this in when replacing ad hoc reducers, filters, predicates, object traversal, or sorting with Effect
> collection helpers.

## Array Reducers and Predicates

Use `Array as Arr` when the input is already an iterable collection, or after you have intentionally converted a record
with `Record.values` or `Record.toEntries`.

```typescript
import { Array as Arr, Option } from "effect"

const totalCount = Arr.reduce(items, 0, (sum, item) => sum + item.count)

const activeItems = Arr.filter(items, (item) => item.active)

const hasFailures = Arr.some(items, (item) => item.status === "failed")

const allReady = Arr.every(items, (item) => item.status === "ready")

const firstAdmin = Arr.findFirst(users, (user) => user.role === "admin")

if (Option.isSome(firstAdmin)) {
  firstAdmin.value.id
}

const [disabledItems, enabledItems] = Arr.partition(items, (item) => item.enabled)
```

## Object and Record Traversal

`Arr` does not enumerate object properties. Use Effect's `Record` module when keys matter or the result should remain
record-shaped; use `Record.values` or `Record.toEntries` when you deliberately want array-shaped traversal.

```typescript
import { Array as Arr, Record } from "effect"

// Replaces Object.values(record).reduce(...)
const totalCount = Record.reduce(countsById, 0, (sum, count) => sum + count)

// Replaces Object.entries(record).map(...) when key and value both matter
const labels = Record.collect(usersById, (id, user) => `${id}:${user.name}`)

// Use Arr after Record.values / Record.toEntries when array operators are clearer
const positiveCounts = Arr.filter(Record.values(countsById), (count) => count > 0)

const adminLabels = Arr.filter(Record.toEntries(usersById), ([id, user]) =>
  id.startsWith("admin-") && user.role === "admin"
)

// Prefer Record.* when preserving record shape or key-aware traversal
const enabledById = Record.filter(usersById, (user) => user.enabled)

const namesById = Record.map(usersById, (user) => user.name)

const activeCount = Record.reduce(usersById, 0, (sum, user) => sum + (user.active ? 1 : 0))

const hasAdmin = Record.some(usersById, (user, id) => id.startsWith("admin-") && user.role === "admin")

const allEnabled = Record.every(usersById, (user) => user.enabled)
```

## Sorting

Use `Order` with `Arr.sort`, `Arr.sortWith`, or `Arr.sortBy` instead of copied mutable sorts like `[...items].sort(...)`.
These helpers accept any iterable and return a new array.

```typescript
import { Array as Arr, Order } from "effect"

Arr.sort([3, 1, 2], Order.number)              // [1, 2, 3]
Arr.sort(["b", "a", "c"], Order.string)        // ["a", "b", "c"]
Arr.sort(new Set([3n, 1n, 2n]), Order.bigint)  // [1n, 2n, 3n]

Arr.sortWith(users, (user) => user.age, Order.number)

Arr.sortBy(
  users,
  Order.mapInput(Order.number, (user: User) => user.age),
  Order.mapInput(Order.string, (user: User) => user.name)
)

// Built-in orderings: Order.string, Order.number, Order.bigint, Order.boolean, Order.Date
// Reverse ordering: Order.reverse(Order.number)
```

## Replacement Cheatsheet

| Existing logic                                       | Prefer                                                                                       |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `array.reduce(...)`                                  | `Arr.reduce(array, zero, f)`                                                                 |
| `array.filter(...)`                                  | `Arr.filter(array, predicate)`                                                               |
| `array.some(...)` / `array.every(...)`               | `Arr.some(array, predicate)` / `Arr.every(array, predicate)`                                 |
| `array.find(...)`                                    | `Arr.findFirst(array, predicate)` returning `Option<A>`                                      |
| `Object.values(record).reduce(...)`                  | `Record.reduce(record, zero, f)`                                                             |
| `Object.values(record).filter/map(...)`              | `Arr.filter` / `Arr.map` over `Record.values(record)`                                        |
| `Object.entries(record).filter/map/fromEntries(...)` | `Record.filter`, `Record.map`, `Record.collect`, `Record.fromEntries`, or `Record.toEntries` |
| `[...items].sort(compare)`                           | `Arr.sort`, `Arr.sortWith`, or `Arr.sortBy` with `Order`                                     |

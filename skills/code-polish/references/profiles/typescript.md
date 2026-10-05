# TypeScript Profile

Load when the diff touches TypeScript source, declarations, compiler config, package metadata, or generated types.

## Checks

- `TS-001` Unsafe boundary typing (`HIGH`): `any`, assertions, or unchecked generics hide invalid external data.
- `TS-002` Unhandled async failure (`HIGH`): Missing `await`, `.catch`, `return`, or error propagation drops failures.
- `TS-003` Async ordering bug (`HIGH`): Concurrent operations can overwrite newer state or observe stale data.
- `TS-004` Module/tool drift (`MEDIUM`): `tsconfig`, package metadata, generated types, or lockfiles change
  inconsistently.
- `TS-005` Lifecycle leak (`MEDIUM`): Timers, listeners, subscriptions, or caches outlive their owner.

## Evidence Expectations

- Show the invalid input, async timeline, or ownership path.
- Name the narrow TypeScript command that proves the finding.

# Go Profile

Load when the diff touches Go services, CLIs, concurrency, context propagation, error handling, modules, or tests.

## Checks

- `GO-001` Context loss (`HIGH`): request-scoped, command-scoped, or shutdown work ignores cancellation, deadlines, or parent context.
- `GO-002` Goroutine/channel leak (`HIGH`): goroutines, timers, tickers, readers, or channels can block or outlive their owner.
- `GO-003` Error loss (`HIGH`): returned errors are ignored, wrapped without useful context, or replaced with unsafe zero values.
- `GO-004` Nil/zero-value trap (`HIGH`): nil pointers, nil interfaces, nil maps/slices, or zero-value structs break valid inputs or error paths.
- `GO-005` Loop/capture aliasing (`MEDIUM`): loop variables, pointer reuse, or shared buffers produce incorrect references for the module's Go version.
- `GO-006` Interface bloat (`MEDIUM`): broad interfaces, package-level globals, or hard-coded dependencies make behavior hard to test or substitute.
- `GO-007` Module/tool drift (`MEDIUM`): `go.mod`, `go.sum`, generated files, or tool versions change without a matching reason or reproducible command.
- `GO-008` Test blind spot (`MEDIUM`): table tests, race-sensitive paths, or error branches do not cover changed behavior.

## Evidence Expectations

- Show the failing input, cancellation path, concurrency schedule, or error branch.
- Name the narrow Go command that proves the finding, such as `go test ./pkg/foo`, `go test -race ./pkg/foo`, `go test ./...`, or `go mod tidy`.

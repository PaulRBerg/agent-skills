# Streams and Backpressure

Streams are lazy and may be infinite. Before consuming one, determine its termination, backpressure, error, and resource
semantics.

## Bound Consumption

Never collect a stream that may be infinite without a bound. Use `Stream.take`, `Stream.takeUntil`, a domain termination
condition, or an Effect timeout. Prefer `runForEach`, `runFold`, or another incremental consumer when the whole result
does not need to be retained. In v4, `runFold` takes a lazy initial value (`Stream.runFold(self, () => s, f)`), and the
`run*Scoped` variants are gone because run functions manage the stream scope.

## Preserve Backpressure and Chunking

Streams are pull-based; avoid converting them to eager arrays merely for familiar collection APIs. Use `mapEffect` or
`flatMap` when a transformation is effectful, and choose concurrency explicitly. Batch with `grouped` or `groupedWithin`
only when the downstream system benefits from the chosen size or time window.

## Own Resources and Failures

Use `Stream.scoped(Stream.fromEffect(Effect.acquireRelease(acquire, release)))` (v3 `Stream.acquireRelease`),
`Stream.ensuring`, or `Stream.onExit` for resources and cleanup. A consuming Scope must outlive the stream. Recovery
with `Stream.catchTag`, `Stream.catch` (v3 `catchAll`), or `Stream.retry` must preserve the intended domain semantics;
do not turn a required failure into an empty stream.

Tests must bound streams, advance `TestClock` (`effect/testing`) for scheduled producers, and interrupt or scope
background consumers.

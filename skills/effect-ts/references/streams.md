# Streams and Backpressure

Streams are lazy and may be infinite. Before consuming one, determine its termination, backpressure, error, and resource
semantics.

## Bound Consumption

Never collect a stream that may be infinite without a bound. Use `Stream.take`, `Stream.takeUntil`, a domain termination
condition, or an Effect timeout. Prefer `runForEach`, `runFold`, or another incremental consumer when the whole result
does not need to be retained.

## Preserve Backpressure and Chunking

Streams are pull-based; avoid converting them to eager arrays merely for familiar collection APIs. Use `mapEffect` or
`flatMap` when a transformation is effectful, and choose concurrency explicitly. Batch with `grouped` or `groupedWithin`
only when the downstream system benefits from the chosen size or time window.

## Own Resources and Failures

Use `Stream.acquireRelease`, `Stream.scoped`, or `Stream.ensuring` for resources and cleanup. A consuming Scope must
outlive the stream. Recovery with `catchTag`, `catchAll`, or `retry` must preserve the intended domain semantics; do not
turn a required failure into an empty stream.

Tests must bound streams, advance `TestClock` for scheduled producers, and interrupt or scope background consumers.

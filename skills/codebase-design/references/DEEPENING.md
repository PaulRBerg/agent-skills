# Deepening

This reference explains how to deepen a cluster of shallow modules safely, given its dependencies. It assumes the
vocabulary in [SKILL.md](../SKILL.md) — **module**, **interface**, **seam**, **adapter**.

## Dependency categories

When assessing a candidate for deepening, classify its dependencies. The category determines how to test the deepened
module across its seam.

### 1. In-process

Pure computation, in-memory state, no I/O. If merging reduces total complexity, this is a strong deepening candidate.
Test through the new interface directly. No adapter is needed.

### 2. Local-substitutable

Dependencies that have local test stand-ins (PGLite for Postgres, in-memory filesystem). You can deepen the module if
the stand-in exists. Tests run the deepened module with the stand-in running in the test suite. The seam is internal.
There is no port at the module's external interface.

### 3. Remote but owned (Ports & Adapters)

Your own services across a network boundary (microservices, internal APIs). Define a **port** (interface) at the seam.
The deep module owns the logic. The transport is an injected **adapter**. Tests use an in-memory adapter. Production
uses an HTTP/gRPC/queue adapter.

Recommendation shape: _"Define a port at the seam, implement an HTTP adapter for production and an in-memory adapter for
testing, so the logic sits in one deep module even though it's deployed across a network."_

### 4. True external (Mock)

Third-party services (Stripe, Twilio, etc.) that you do not control. The deepened module takes the external dependency
as an injected port. Tests provide a mock adapter.

## Seam discipline

- Do not introduce a port unless at least two adapters are justified (typically production + test). A single-adapter
  seam is just indirection.
- Do not expose internal seams through the interface just because tests use them.

## Testing strategy: replace, don't layer

- Once tests at the deepened module's interface exist, old unit tests on shallow modules become waste. At that point,
  delete those old tests.
- Write new tests at the deepened module's interface. The **interface is the test surface**.
- Tests assert on observable outcomes through the interface, not internal state.
- Tests should survive internal refactors — they describe behaviour, not implementation. If a test has to change when
  the implementation changes, it tests past the interface.

## Finding deepening candidates

- Prioritize recently changed hot spots. Inspect `git log --oneline` for recurring files and areas. Deepening pays off
  where change concentrates.
- Look for friction signals:
  - Understanding one concept requires moving between many small modules.
  - Interfaces are nearly as complex as their implementations.
  - Pure functions were extracted for testability while the real bugs hide in how callers use them. Locality is missing.
  - Code is untestable through its current interface.
- Apply the deletion test to each suspect: "Would deleting it concentrate complexity, or just move it?" Treat
  "concentrates" as the signal to deepen.

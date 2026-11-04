# Swift Profile

Load when the diff touches Swift source, SwiftPM manifests, Xcode targets or settings, Apple app resources,
entitlements, or tests. Cover iOS and macOS, including SwiftUI, UIKit, AppKit, and app extensions.

## Repository Context

Read the target's instructions, package manifest, Xcode settings, formatter configuration, and validation recipes.
Establish its deployment targets, Swift language mode, concurrency checking, default actor isolation, and enabled
upcoming features. Compiler version alone does not establish concurrency behavior.

Preserve the existing observation model, dependency boundaries, and test framework. Do not migrate `ObservableObject` to
`@Observable`, introduce an architecture framework, or move platform APIs into a portable package as incidental polish.

## Checks

- `SW-001` Reachable trap or lost error (`HIGH`): Force unwraps, `try!`, forced casts, indexing, or discarded errors
  turn fallible input into a crash or false success. Prove the failed invariant instead of flagging syntax alone.
- `SW-002` Isolation or reentrancy defect (`HIGH`): Mutable state crosses an actor boundary without protection, or an
  assumption becomes invalid across `await`. Check UI isolation and `Sendable` contracts against the target's settings.
  Require evidence for `@unchecked Sendable` and `nonisolated(unsafe)` safety. Do not silence diagnostics by weakening
  concurrency checks.
- `SW-003` Task lifetime or stale result (`HIGH`): Unowned tasks, ignored cancellation, or late callbacks outlive their
  operation and overwrite newer state. Trace replacement requests, disconnect, scene suspension, and extension exit.
  Cancellation alone does not prove that an old result cannot publish or recreate a cleared cache.
- `SW-004` Resource or callback leak (`HIGH`): Retain cycles, observers, timers, subscriptions, streams, or file handles
  outlive their owner. A continuation can resume twice or never resume during cancellation, timeout, or completion
  races. Check the actual ownership graph before adding weak captures.
- `SW-005` Main actor stall (`HIGH`): Synchronous network, disk, decoding, or CPU work blocks UI progress. `async` and
  `Task {}` do not by themselves move work off the main actor. Preserve structured ownership when moving expensive work.
- `SW-006` View identity or state ownership (`HIGH`): SwiftUI recreates an owned model, misses an observable update, or
  assigns unstable list identity. Check bindings, task IDs, navigation, and state shared across scenes or macOS windows.
  Keep side effects out of `body` evaluation and preserve UIKit/AppKit delegate lifetimes when bridging views.
- `SW-007` Persistence or credential boundary (`HIGH`): Cache keys mix connections or queries, partial writes corrupt
  durable state, or logout leaves sensitive data accessible. Check Keychain namespaces, access groups, file protection,
  backup policy, and resource bounds where changed. Keep credentials out of preferences, bundles, and diagnostics.
- `SW-008` Platform integration mismatch (`HIGH`): API availability, target membership, resources, or extension
  embedding disagree with supported platforms. Check entitlements, shared containers, permission denial, and extension
  completion. For macOS file access, check sandbox permissions, bookmark renewal, and balanced security-scoped resource
  access.
- `SW-009` Purchase state error (`HIGH`): When StoreKit changes, inspect verification, pending/cancelled results,
  revocation, restoration, and transaction finishing. Trace entitlement delivery before acknowledging success.
- `SW-010` Interaction regression (`MEDIUM`): Changed controls lose VoiceOver labels, text scaling, keyboard focus,
  reduced motion, or usable loading/error states. Check macOS window resizing and commands when those surfaces change.

## Evidence and Verification

- Show the triggering input, ownership path, or sequence across suspension points. Distinguish compiler-proven safety
  from runtime assumptions. Use controlled async completions to test late results and cancellation without timing
  sleeps.
- Use repository recipes and the configured formatter on changed files. If the repo uses the toolchain formatter,
  `xcrun swift-format lint --strict <files>` checks explicit files. Do not introduce a second formatting tool.
- Run targeted Swift Testing or XCTest coverage through the existing runner. `swift test --filter <test>` is suitable
  for a compatible Swift package. Package tests do not compile or verify the containing app and its extensions.
- Build each affected app or extension target with the repository's Xcode recipe and supported destination. Verify a
  native macOS target when changed. An iOS simulator build does not prove macOS compilation or device behavior.
- When UI or platform integration changes, exercise the affected flow on the applicable simulator, device, or Mac.
  Follow repository rules for simulator ownership and isolated build outputs. Do not reset unrelated devices.
- Report signing, Keychain sharing, sandbox access, and StoreKit behavior as unverified when checks did not exercise
  them. Compilation alone does not establish installation, permission behavior, or successful purchases.

## References

- [Swift concurrency](https://docs.swift.org/swift-book/LanguageGuide/Concurrency.html)
- [Concurrency settings and execution](https://developer.apple.com/videos/play/wwdc2025/268/)
- [SwiftUI model ownership](https://developer.apple.com/documentation/swiftui/managing-model-data-in-your-app)
- [macOS sandbox file access](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox)

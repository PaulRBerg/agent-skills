# Kotlin Profile

Load when the diff touches Kotlin source or scripts, Android resources or manifests, Gradle configuration, or tests.
Cover Android Views and Jetpack Compose as applicable. Apply language checks to JVM-only core modules too.

## Repository Context

Read the target's instructions, Gradle wrapper, version catalog, module configuration, and validation recipes. Establish
the Kotlin/AGP versions, JDK and JVM targets, Android SDK levels, build variants, and configured lint or format tools.

Preserve existing module boundaries, dependency wiring, and test frameworks. Keep Android APIs out of portable core
modules. Do not add a DI framework, formatter, or architecture migration as incidental polish.

## Checks

- `KT-001` Reachable boundary failure (`HIGH`): `!!`, unchecked casts, Java platform types, `lateinit`, or indexing fail
  for a reachable input or lifecycle state. Error fallbacks hide failure as valid empty data or success.
- `KT-002` Coroutine ownership or cancellation (`HIGH`): Jobs outlive their intended scope, failures disappear, or broad
  catches and `runCatching` swallow `CancellationException`. Check parent-child ownership and explicit owners for work
  that must outlive a screen. Preserve cancellation and cleanup in callback adapters.
- `KT-003` Main thread stall (`HIGH`): Blocking I/O, CPU work, or `runBlocking` freezes the UI. `suspend` alone does not
  move work off the caller's dispatcher. Check dispatcher selection and cancellation during blocking or long loops.
- `KT-004` Flow or state race (`HIGH`): Concurrent updates lose data, late results overwrite a new request, or mutable
  state escapes its owner. In-place mutation can prevent observable updates. Check StateFlow exposure, atomic updates,
  sharing/replay behavior, and account-scoped cache invalidation against the intended contract.
- `KT-005` Compose or collection lifecycle (`HIGH`): Recomposition repeats effects, effect keys retain stale inputs, or
  listeners survive disposal. Check `LaunchedEffect`, `DisposableEffect`, and `remember` ownership. Android UI
  collection must respect the relevant lifecycle, for example through `collectAsStateWithLifecycle` or
  `repeatOnLifecycle`.
- `KT-006` Recreation or resource leak (`HIGH`): Rotation, navigation, or process death loses required state or repeats
  an action. Distinguish `remember`, saved state, ViewModel lifetime, and durable storage. Check retained Activity/View
  references, receivers, cursors, streams, and billing connections. Match cleanup to each resource's owner.
- `KT-007` Android security boundary (`HIGH`): Exported components, intents, deep links, or content URIs accept
  untrusted input or grant excessive access. Check permissions, URI grants, PendingIntent mutability, network security,
  backup rules, and secret storage where changed. Ordinary preferences and DataStore are not credential encryption.
- `KT-008` Build or API mismatch (`HIGH`): Kotlin, AGP, Gradle, JDK, Compose tooling, or JVM targets disagree. Android
  APIs exceed `minSdk` without guards. Manifest merging, resources, or R8 rules break an affected variant or reflective
  call. Keep pure JVM compilation distinct from Android app compilation.
- `KT-009` Purchase state error (`HIGH`): When Play Billing changes, inspect verification, pending/cancelled results,
  entitlement refresh, restoration, and acknowledgement or consumption. A launched billing flow does not prove payment.
- `KT-010` Interaction regression (`MEDIUM`): Changed controls lose TalkBack semantics, font scaling, keyboard focus,
  touch targets, or usable loading/error states. Check back navigation, insets, and configuration changes when affected.

## Evidence and Verification

- Show the input, coroutine schedule, lifecycle transition, or build variant that triggers the defect. Treat safe
  assertions and intentional application-owned work as valid when their invariants hold.
- Use repository recipes, the checked-in Gradle wrapper, and configured style checks. Do not invent ktlint or detekt
  tasks when the project does not configure them. Scope any formatter writes to changed files.
- Run the affected modules' tests. A JVM `:core:test` task does not cover Android app sources. Use the actual app
  variant's unit tests, lint, and assembly tasks where present, such as `:app:testDebugUnitTest`, `:app:lintDebug`, and
  `:app:assembleDebug`. Verify task names and variants before running them.
- For changed coroutine behavior, use existing test dispatchers and controlled completions. Share the test scheduler
  when using `runTest`. Add an injection seam only when needed to prove the changed behavior deterministically.
- Exercise changed UI, permission, persistence, and lifecycle behavior through the project's device or emulator tests.
  Rotation does not prove recovery after process death. JVM tests and successful assembly do not prove device behavior.
- When shrinker rules or reflective contracts change, validate the affected minified variant. Report live billing,
  signing, and installation as unverified unless exercised. Do not publish a build as validation.
- Preserve shared emulators and build resources. Prefer the repository's resource controls or `--no-daemon` for ad hoc
  Gradle runs. Clean up only resources started for this validation.

## References

- [Android coroutine practices](https://developer.android.com/kotlin/coroutines/coroutines-best-practices)
- [Compose effect lifecycles](https://developer.android.com/develop/ui/compose/side-effects)
- [Android state restoration](https://developer.android.com/topic/libraries/architecture/saving-states)
- [Kotlin structured concurrency](https://kotlinlang.org/docs/coroutines-basics.html)

# Tool Finder Reference

Use secondary comparisons to discover candidates or corroborate material claims. Match primary sources and criteria to
the ecosystem:

| Ecosystem                | Primary evidence and decision criteria                                                                                                                                                                                                          |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| JavaScript/TypeScript    | npm, package docs, repository, changelog, advisories. Criteria: types, ESM fit, bundle and dependency cost                                                                                                                                      |
| Python                   | PyPI, package docs, repository. Criteria: supported Python, typing, stdlib alternative, native-extension cost                                                                                                                                   |
| Rust                     | crates.io, docs.rs, repository. Criteria: safety, compile impact, `no_std` need, documentation                                                                                                                                                  |
| Go                       | pkg.go.dev, standard library docs, repository. Criteria: stdlib fit, dependencies, Go support, cancellation                                                                                                                                     |
| Swift/iOS                | Apple Developer and Swift documentation, Swift Package Index, upstream manifest, releases, and advisories. Criteria: deployment target, Swift/Xcode compatibility, concurrency, resource bundles, license, and build cost                       |
| Kotlin/Android           | Android Developers, Kotlin/JetBrains documentation, Google Maven or Maven Central, upstream releases, and advisories. Criteria: Android versus Multiplatform targets, minSdk, Kotlin/Gradle/AGP compatibility, Compose fit, and dependency cost |
| CLI                      | official repository and package metadata. Criteria: install path, structured output, startup, platform support                                                                                                                                  |
| VS Code                  | Marketplace or Open VSX, repository, changelog. Criteria: permissions, activation cost, editor compatibility                                                                                                                                    |
| Databases/infrastructure | product and cloud-provider docs, migration and operations guidance. Criteria: workload, backup, scaling, operational cost                                                                                                                       |
| Agent skills             | [skills.sh](https://skills.sh), vendor docs, GitHub topics, GitHub code search for `path:SKILL.md`, and web search. Criteria: valid metadata, portability, token use, license, bundled-script risk                                              |

If evidence is otherwise close, prefer:

- stronger TypeScript support, then a smaller bundle, then recency.
- the Python or Go standard library, then typed/current Python support or fewer dependencies with idiomatic
  cancellation.
- safer, better-documented Rust.
- native Apple or Android APIs when they meet the requirements without an external dependency.
- CLIs with simpler installation and structured output.
- agent skills with progressive disclosure and fewer execution risks.

Treat a credible advisory, unsupported platform, stale or missing documentation, disruptive release churn, excess
dependencies or bundle size, weak maintenance, excessive editor permissions, or unsafe bundled scripts as a material red
flag only when evidence and the user's environment make it consequential.

## Mobile tools and localization

Identify the presentation framework and shared-code boundaries before comparing mobile libraries. Distinguish SwiftUI,
UIKit, Android Jetpack Compose, and Compose Multiplatform. Do not propose a platform migration to solve a local tooling
need.

For Swift localization, compare
[Apple String Catalogs](https://developer.apple.com/documentation/xcode/localizing-and-varying-text-with-a-string-catalog)
and native generated symbols before adding a generator or runtime library. Check resource-bundle lookup, plural rules,
format arguments, language overrides, and fallback behavior. Verify both Xcode builds and command-line SwiftPM workflows
when the project uses both. Xcode resource processing and symbol generation do not prove `swift test` compatibility.

For Android localization, first check XML string/plural resources and
[Compose resource APIs](https://developer.android.com/develop/ui/compose/resources). Check per-app language preferences,
locale fallback, formatted values, and right-to-left layouts. For existing Compose Multiplatform projects, also compare
[multiplatform resources](https://kotlinlang.org/docs/multiplatform/compose-localize-strings.html).

Distinguish resource management and type-safe access from translation services. A library does not supply reviewed
translations. Evaluate remote services only when the task needs them, including recurring cost and data disclosure.

## Installation guidance

For a private JavaScript package (`"private": true`), install into dependencies (`ni package-name`, never `ni -D`). For
that private package, when the repository has a package manager, translate the command to it.

For Swift, follow the project's existing Swift Package Manager or Xcode package setup. Give the verified package URL,
version requirement, product name, and target wiring. For Kotlin, follow the existing Gradle Kotlin DSL or version
catalog. Give verified coordinates and add them to the correct module or Multiplatform source set. Preserve platform
constraints and repository version-age rules. For a native option, give the configuration or resource migration steps
instead of an install command.

Suggest an official installer script only after the user accepts its risk. Confirm the target package and environment
before giving the exact command.

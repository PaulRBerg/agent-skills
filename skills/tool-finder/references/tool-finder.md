# Tool Finder Reference

Use secondary comparisons to discover candidates or corroborate material claims. Match primary sources and criteria to
the ecosystem:

| Ecosystem                | Primary evidence and decision criteria                                                                                                                                                   |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| JavaScript/TypeScript    | npm, package docs, repository, changelog, advisories; types, ESM fit, bundle and dependency cost                                                                                         |
| Python                   | PyPI, package docs, repository; supported Python, typing, stdlib alternative, native-extension cost                                                                                      |
| Rust                     | crates.io, docs.rs, repository; safety, compile impact, `no_std` need, documentation                                                                                                     |
| Go                       | pkg.go.dev, standard library docs, repository; stdlib fit, dependencies, Go support, cancellation                                                                                        |
| CLI                      | official repository and package metadata; install path, structured output, startup, platform support                                                                                     |
| VS Code                  | Marketplace or Open VSX, repository, changelog; permissions, activation cost, editor compatibility                                                                                       |
| Databases/infrastructure | product and cloud-provider docs, migration and operations guidance; workload, backup, scaling, operational cost                                                                          |
| Agent skills             | [skills.sh](https://skills.sh), vendor docs, GitHub topics, GitHub code search for `path:SKILL.md`, and web search; valid metadata, portability, token use, license, bundled-script risk |

If evidence is otherwise close, prefer: stronger TypeScript support then smaller bundle then recency; Python or Go
standard library, then typed/current Python support or fewer dependencies with idiomatic cancellation; safer,
better-documented Rust; simpler-install, structured-output CLIs; and agent skills with progressive disclosure and fewer
execution risks.

Treat a credible advisory, unsupported platform, stale or missing documentation, disruptive release churn, excess
dependencies or bundle size, weak maintenance, excessive editor permissions, or unsafe bundled scripts as a material red
flag only when evidence and the user's environment make it consequential.

## Installation guidance

For a private JavaScript package (`"private": true`), install into dependencies (`ni package-name`, never `ni -D`);
translate to the repository's package manager when it has one.

Suggest an official installer script only after the user accepts its risk. Confirm the target package and environment
before giving the exact command.

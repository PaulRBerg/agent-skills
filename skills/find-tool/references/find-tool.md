# Find Tool Reference

Detailed search, scoring, and reporting guidance for the `find-tool` skill.

## Search Strategy

Adapt search queries to the ecosystem and include the current year when recency matters.

### npm / JavaScript / TypeScript

- `"best npm package for [task]" 2025 2026`
- `"[task] javascript typescript library comparison"`
- `"[task] npm trending"`
- npm registry package pages
- npm trends
- Bundlephobia or package-size equivalents
- GitHub repositories, releases, issues, and advisories

### Python

- `"best python library for [task]" 2025 2026`
- `"[task] python package comparison pypi"`
- `"popular [task] python"`
- PyPI package pages
- pepy.tech download stats
- GitHub repositories
- Libraries.io

### Rust

- `"best rust crate for [task]" 2025 2026`
- `"[task] rust library comparison"`
- `"popular [task] rust crates.io"`
- crates.io
- lib.rs
- GitHub repositories
- Blessed.rs and other curated lists

### Go

- `"best go package for [task]" 2025 2026`
- `"[task] golang library comparison"`
- `"popular [task] go module pkg.go.dev"`
- pkg.go.dev
- Awesome Go
- go.libhunt.com
- GitHub repositories

### CLI / System Tools

- `"best [task] cli tool" 2025 2026`
- `"[task] cli tool comparison"`
- `"modern alternative to [old-tool]"`
- GitHub repositories
- Homebrew formulae and analytics
- Maintained command-line tool lists

### VSCode Extensions

- `"best vscode extension for [task]" 2025 2026`
- `"[task] vscode extension comparison"`
- `"vscode marketplace [task]"`
- VS Code Marketplace
- Open VSX Registry
- GitHub repositories
- Extension changelogs

### Agent Skills

- `"best agent skill for [task]"`
- `site:skills.sh [task]`
- `site:github.com SKILL.md [task]`
- `"[task] claude code skill"`
- `"[task] cursor skill"`
- `"[task] copilot skill"`
- `"awesome agent skills" [task]`

Never rely on a single registry for skills. Combine skills.sh, agentskills.io, GitHub topic search
(`topic:agent-skills`, `topic:claude-skills`), GitHub code search for `path:SKILL.md`, and generic web search.
Registries miss skills published only on GitHub or in blog posts.

### Databases / Infrastructure

- `"best [task] database" 2025 2026`
- `"[task] vs [alternative] comparison"`
- `"[task] production use cases"`
- Official docs and product pages
- Cloud provider docs
- Migration guides
- Operations and failure-mode writeups

## Key Information Sources

Prioritize sources by ecosystem:

| Ecosystem             | Primary sources                                                                                                                                            |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| JavaScript/TypeScript | npm registry, npm trends, Bundlephobia, GitHub repositories, changelogs, security advisories                                                               |
| Python                | PyPI, pepy.tech, GitHub repositories, Libraries.io, package docs                                                                                           |
| Rust                  | crates.io, lib.rs, GitHub repositories, Blessed.rs, docs.rs                                                                                                |
| Go                    | pkg.go.dev, standard library docs, Awesome Go, go.libhunt.com, GitHub repositories                                                                         |
| CLI tools             | GitHub repositories, Homebrew analytics, package manager metadata, maintained CLI lists                                                                    |
| VSCode extensions     | VS Code Marketplace, Open VSX, GitHub repositories, extension changelogs                                                                                   |
| Agent skills          | skills.sh, agentskills.io, GitHub `topic:agent-skills`, GitHub `topic:claude-skills`, GitHub `path:SKILL.md`, vendor docs, generic web and blog references |
| All ecosystems        | GitHub stars/activity/issues, security advisories, StackOverflow discussions, Reddit, official benchmarks                                                  |

## Evaluation Criteria

Start from the user's decision criteria. When they are unstated, use these as evidence prompts rather than a scoring
formula:

| Criterion            | Evidence to collect                                               |
| -------------------- | ----------------------------------------------------------------- |
| Adoption             | Downloads, installs, GitHub stars, community size, production use |
| Maintenance          | Last release, recent commits, issue response, active maintainers  |
| Security and quality | Known vulnerabilities, audits, test/CI quality, license           |
| Documentation and DX | Docs quality, examples, API clarity, error messages               |
| Performance and cost | Benchmarks, bundle size, startup time, memory, operating cost     |

### Ecosystem-Specific Criteria

| Ecosystem             | Additional criteria                                                                                   |
| --------------------- | ----------------------------------------------------------------------------------------------------- |
| JavaScript/TypeScript | Native types, ESM, tree-shaking, minified/gzipped size, Bun/Deno compatibility, dependency count      |
| Python                | Type hints, supported Python versions, package size, native extension overhead                        |
| Rust                  | Unsafe usage, compile-time impact, binary size, `no_std` support                                      |
| Go                    | Go version support, dependency count, standard library alternative, generics fit, `context.Context`   |
| CLI tools             | Install method, startup time, structured output, plugin ecosystem, cross-platform support             |
| VSCode extensions     | Install count, rating, activation events, extension size, VS Code/Cursor compatibility, permissions   |
| Agent skills          | Valid `SKILL.md`, portability, bundled-script dependencies and sandbox risk, token footprint, license |
| Databases             | Query performance, scaling, backup/restore, operational complexity, managed hosting options           |

## Tie-Breakers

- JavaScript/TypeScript: choose better TypeScript support, then smaller bundle size, then more recent update.
- Python: choose stdlib first when adequate, then typed packages with current Python support.
- Rust: choose safer and better-documented crates unless performance data clearly favors another.
- Go: choose the standard library first, then packages with fewer dependencies and idiomatic cancellation support.
- CLI tools: choose the tool with simpler installation and better structured output when capability is comparable.
- Agent skills: choose the skill with clearer progressive-disclosure metadata and fewer bundled execution risks when
  capability is comparable.

## Output Shapes

Choose the smallest shape that makes the decision legible. A concise recommendation with one alternative is often
enough. When several options share comparable fields, adapt this structure:

````markdown
### 🏆 Pick: `tool-name`

<One sentence naming the decisive reason.>

### 📦 Install

```bash
[ecosystem-appropriate install command]
```

| Criterion            | Why it wins                | Evidence                 |
| -------------------- | -------------------------- | ------------------------ |
| [material criterion] | [decision-relevant reason] | [current primary source] |

### 🥈 Closest alternative: `alternative-tool`

[Why it is viable and the tradeoff that keeps it second.]

### ⚠️ Caveats

[Only material caveats; omit this section when empty.]
````

Omit the criteria table when one sentence carries the evidence. Do not force a fixed number of strengths, alternatives,
or popularity statistics; include only fields that affect this decision.

GitHub columns in tables should use Markdown links, e.g. `[sindresorhus/execa](https://github.com/sindresorhus/execa)`.

## Optional Comparison Tables

Use the table that matches the ecosystem.

### npm / JavaScript / TypeScript

| Package | GitHub | Stars | Downloads/week | Size | TS Support | Last Update |
| ------- | ------ | ----- | -------------- | ---- | ---------- | ----------- |

### Python

| Package | GitHub | Stars | Downloads/month | Typing | Py Version | Last Update |
| ------- | ------ | ----- | --------------- | ------ | ---------- | ----------- |

### Rust

| Crate | GitHub | Stars | Downloads | Unsafe | Compile Time | Last Update |
| ----- | ------ | ----- | --------- | ------ | ------------ | ----------- |

### Go

| Package | GitHub | Stars | Imported By | Go Version | Deps | Last Update |
| ------- | ------ | ----- | ----------- | ---------- | ---- | ----------- |

### CLI Tools

| Tool | GitHub | Stars | Install Method | Performance | Platform | Last Update |
| ---- | ------ | ----- | -------------- | ----------- | -------- | ----------- |

### VSCode Extensions

| Extension | GitHub | Stars | Installs | Rating | Size | Last Update |
| --------- | ------ | ----- | -------- | ------ | ---- | ----------- |

### Agent Skills

| Skill | GitHub | Stars | Installs (skills.sh) | Host Agents | SKILL.md Size | Last Update |
| ----- | ------ | ----- | -------------------- | ----------- | ------------- | ----------- |

`Installs` is a popularity signal, like npm downloads, not an install instruction.

## Red Flags

Call out any red flag that is material to the user's decision:

- Abandoned tools with no updates in 12+ months.
- Agent skills with no updates in 6+ months when the surrounding ecosystem is moving quickly.
- Known CVEs, advisories, or unresolved security issues.
- Heavy dependency trees or bundle bloat.
- Frequent major versions or breaking changes.
- Poor, missing, or outdated documentation.
- Platform limitations that conflict with the user's environment.
- Known performance bottlenecks or high resource use.

Ecosystem-specific red flags:

| Ecosystem    | Red flags                                                                                    |
| ------------ | -------------------------------------------------------------------------------------------- |
| npm          | Bundle bloat, missing TypeScript support, CommonJS-only package when ESM matters             |
| Python       | No type hints, Python 2 or stale Python version support, native extension surprises          |
| Rust         | Excessive unsafe code, long compile times, weak docs                                         |
| Go           | Heavy dependency trees, missing `go.mod`, unmaintained vendored dependencies                 |
| CLI          | Slow startup, poor errors, no structured output, fragile installer scripts                   |
| VSCode       | Heavy activation, excessive permissions, conflicts with popular extensions, no Cursor fit    |
| Agent skills | Vague `SKILL.md` description, hardcoded vendor paths, unrestricted bundled shell, no license |

## Installation Commands

Use the package manager appropriate to the ecosystem and repository.

### JavaScript / TypeScript

The user prefers the `ni` utility:

```bash
ni package-name
ni -D dev-package
nun package-name
nr script-name
nlx package-name
```

For private packages (`"private": true`), use only `ni package-name`, not `ni -D`.

### Python

Prefer the project's package manager. Common commands:

```bash
uv add package-name
uv pip install package-name
poetry add package-name
pip install package-name
```

### Rust

```bash
cargo add crate-name
cargo install binary-name
```

### Go

```bash
go get github.com/org/package
go install github.com/org/binary@latest
```

### CLI Tools on macOS

Prioritize Homebrew when the formula is official or well-maintained:

```bash
brew install tool-name
cargo install tool-name
go install github.com/org/tool@latest
```

Use installer scripts only when they are official and the user accepts the risk.

### VSCode Extensions

```bash
code --install-extension publisher.extension-name
cursor --install-extension publisher.extension-name
```

## Edge Cases

### Multiple Tools Are Equal

Pick the option with the lowest switching cost and best maintenance signal. Say when the choice is close and name the
deciding factor.

### No Good Package Exists

If searches reveal no suitable package:

1. State that clearly.
2. Suggest building a small custom solution.
3. Provide a starter approach or code sketch when useful.
4. Reference near-miss packages as inspiration, not recommendations.

### Standard Library Is Enough

Prefer no dependency when the standard library or platform primitive is adequate. Examples:

- Python 3.11+ `tomllib` for TOML reading.
- Go `log/slog` for structured logging in ordinary workloads.
- Browser `URL` and `Intl` APIs for many frontend parsing/formatting tasks.

## Example Routing

### JavaScript Default

User: "I need to validate email addresses."

Process:

- Detect JavaScript/TypeScript from context unless repo evidence says otherwise.
- Search current npm/package comparisons.
- Check npm downloads, GitHub maintenance, TypeScript support, bundle size, and security advisories.
- Recommend one package or a platform primitive with alternatives.

### CLI Tool

User: "I need a better alternative to grep."

Process:

- Detect CLI tool.
- Compare ripgrep, ag, ack, and any current alternatives.
- Verify installation methods, speed claims, platform support, and output ergonomics.
- Recommend `ripgrep` when evidence still supports it.

### Agent Skill

User: "Find me an agent skill for writing conventional commits."

Process:

- Detect agent skill request.
- Search generic web, skills.sh, agentskills.io, GitHub topics, and GitHub `path:SKILL.md`.
- Compare install counts, GitHub stars, update recency, host-agent compatibility, token footprint, license, and
  bundled-script risk.
- Provide source links and adoption guidance without assuming the user's host-agent install command.

### Go

User: "I need structured logging in Go."

Process:

- Check whether `log/slog` satisfies the task before recommending external packages.
- Compare `log/slog`, `zap`, and `zerolog` using performance needs, dependency cost, and maintenance.
- Recommend the standard library unless hot-path requirements justify a dependency.

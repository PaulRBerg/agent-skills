---
argument-hint: "[paths] [--simplify] [--review] [--with-profile <name>] [--skip-profile <name>]"
disable-model-invocation: false
name: code-polish
user-invocable: true
description:
  "Polish changed code when the user explicitly asks, or when an active workflow requests post-implementation
  simplification and risk-profiled review over a fixed file scope."
---

# Code Polish

Resolve scope once, make only high-confidence simplifications, fix evidenced defects by risk, and verify the final
state.

## Modes

- `--simplify`: simplify only.
- `--review`: review and fix only.
- Neither or both: simplify, then review the simplified result.
- `--with-profile <name>` / `--skip-profile <name>`: add or suppress review profiles; skip wins.

## Fixed Scope

1. Require a Git repository.
2. Use explicit paths, patterns, ranges, natural-language targets, or a supplied `resolved-scope` block when present.
   Otherwise use only files modified in this session; if session history is unavailable, use all uncommitted tracked and
   untracked files.
3. Exclude lockfiles, generated outputs, vendored code, minified bundles, and large data snapshots from manual review
   unless explicitly requested. Validate relevant excluded outputs through their generator, schema, or invariants.
4. Emit one authoritative `resolved-scope` block and optional `excluded-scope` block. Do not broaden or recompute scope
   later. Stop if it is empty.

## Simplify

Preserve public contracts, inputs, outputs, side effects, error behavior, performance-sensitive characteristics,
telemetry, and operational guards. Apply only changes with a concrete comprehension or defect-risk benefit:

- flatten avoidable control-flow nesting;
- clarify misleading names or dense transforms;
- remove real duplication when the abstraction reduces total complexity;
- tighten local types and contracts without broad churn;
- remove only dead code caused by this session's edits.

Do not split by line count, perform architecture cleanup, convert sync/async APIs, add speculative configurability, or
replace readable duplication with a one-use abstraction. A no-op is a valid result.

## Review and Fix

Judge the diff against the user's request. Prioritize `CRITICAL → HIGH → MEDIUM → LOW`:

- **CRITICAL**: exploitable security, data loss, or critical outage path.
- **HIGH**: behavior, error-path, boundary, or performance defect affecting core behavior.
- **MEDIUM**: resource leak, complexity hotspot, test gap, over-scoped change, speculative complexity, or weak success
  criterion likely to cause defects.
- **LOW**: localized clarity or style issue with a real maintenance cost.

Every finding must cite a verified location, triggering input/state, failure mode, blast radius, and evidence in the
changed code. Merge duplicates and apply the smallest defensible fix. When intent is ambiguous, stop or record the
assumption instead of guessing.

Select every applicable profile and read it once:

| Surface                                                       | Profile                 |
| ------------------------------------------------------------- | ----------------------- |
| auth, secrets, crypto, external input/network, unsafe parsing | `security`              |
| env, config, timeouts, retries, pools, limits                 | `configuration`         |
| Go behavior, concurrency, context, errors                     | `go`                    |
| TypeScript types, modules, packages, async behavior           | `typescript`            |
| Python services, scripts, async, packaging, data IO           | `python`                |
| shell, CI, deploy, installers, quoting                        | `shell`                 |
| CSV/JSON/YAML/binary, schemas, migrations, generated data     | `data-formats`          |
| naming and intent clarity                                     | `naming` unless skipped |

Profiles live at `references/profiles/<name>.md`. Missing selected profiles are a stop condition.

## Verification and Report

Run the narrowest formatter/lint, targeted tests, typecheck, and invariant checks that prove the final touched behavior.
Broaden only for shared contracts. Name skipped checks and why.

Report `Scope`, `Simplifications` when run, `Review Findings and Fixes` when run, `Verification`, and `Residual Risks`.
Findings include severity, location, impact, evidence, fix, and confidence. A residual risk states the assumption,
consequence if wrong, and how to check it. Completion requires fixed scope, traceable edits/findings, and validation
evidence.

Stop when behavior parity or required high-risk validation cannot be established, or a fix requires an unrequested
public-contract change or larger redesign.

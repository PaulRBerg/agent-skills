set allow-duplicate-variables
set allow-duplicate-recipes
set shell := ["bash", "-euo", "pipefail", "-c"]
set unstable

# ---------------------------------------------------------------------------- #
#                                  VARIABLES                                   #
# ---------------------------------------------------------------------------- #

prettier := "bunx --no-install prettier"
prettier_cache := ".cache/prettier/.prettier-cache"
prettier_globs := "\"**/*.{md,json,jsonc,yaml,yml}\""
evm_atlas_generator := "scripts/generate-evm-atlas.ts"
evm_atlas_generator_test := "scripts/generate-evm-atlas.test.ts"
publish_skills_script := "scripts/publish-skills.ts"
publish_skills_test := "scripts/publish-skills.test.ts"
codex_handoff_runner_test := "tests/codex-handoff/test-run-codex-handoff.sh"
codex_handoff_wave_test := "tests/codex-handoff/test-watch-codex-wave.py"

# ---------------------------------------------------------------------------- #
#                                 ENTRYPOINTS                                  #
# ---------------------------------------------------------------------------- #

[group("meta")]
@default:
    just --list
alias d := default

# ---------------------------------------------------------------------------- #
#                                    SETUP                                     #
# ---------------------------------------------------------------------------- #

# Install Husky git hooks for this checkout
[group("setup")]
@hooks-install:
    bun run prepare
alias hi := hooks-install

# Install local developer dependencies
[group("setup")]
@install-deps:
    bun install --frozen-lockfile
alias id := install-deps

# ---------------------------------------------------------------------------- #
#                                    CHECKS                                    #
# ---------------------------------------------------------------------------- #

# Check documentation and configuration formatting
[group("checks")]
@prettier-check +globs=prettier_globs:
    {{ prettier }} \
        --check \
        --cache \
        --cache-location {{ prettier_cache }} \
        --log-level warn \
        --no-error-on-unmatched-pattern \
        {{ globs }}
alias pc := prettier-check

# Format documentation and configuration
[group("checks")]
@prettier-write +globs=prettier_globs:
    {{ prettier }} \
        --write \
        --cache \
        --cache-location {{ prettier_cache }} \
        --log-level warn \
        --no-error-on-unmatched-pattern \
        {{ globs }}
alias pw := prettier-write

# Run staged-file checks
[group("checks")]
@pre-commit:
    sh .husky/pre-commit

# Run every Python test script
[group("checks")]
@python-test:
    for test_file in tests/*/test*.py; do uv run "$test_file"; done

# Run every legacy shell test script
[group("checks")]
@shell-test:
    for test_file in tests/*/test-*.sh; do bash "$test_file"; done

# Run Bats tests recursively, defaulting to tests/
[group("checks")]
[positional-arguments]
@bats-test *paths:
    if [ "$#" -eq 0 ]; then bats --recursive tests; else bats --recursive "$@"; fi

# Lint repository shell scripts and tests
[group("checks")]
[positional-arguments]
@shell-check *paths:
    if [ "$#" -eq 0 ]; then shellcheck skills/*/scripts/*.sh tests/*/*.sh tests/*/*.bats; else shellcheck "$@"; fi

# Run every test suite, the TypeScript check, and the evm-atlas freshness check
[group("checks")]
@test: python-test shell-test bats-test publish-skills-test evm-atlas-test typescript-check evm-atlas-check

# Exercise the Codex handoff runner and wave watcher
[group("checks")]
@codex-handoff-test:
    bash {{ codex_handoff_runner_test }}
    uv run python {{ codex_handoff_wave_test }}

# Regenerate evm-atlas references from crypto-registry's canonical chain JSON + atlas overlays
[group("checks")]
@evm-atlas-generate:
    bun run {{ evm_atlas_generator }} --write
alias eag := evm-atlas-generate

# Check evm-atlas generated references are current
[group("checks")]
@evm-atlas-check:
    bun run {{ evm_atlas_generator }} --check
alias eac := evm-atlas-check

# Exercise the evm-atlas generator against fixture registries
[group("checks")]
@evm-atlas-test:
    bun test {{ evm_atlas_generator_test }}
alias eat := evm-atlas-test

# Refresh atlas-overlays.json routeMesh flags through the routemesh CLI (network call)
[group("checks")]
@evm-atlas-discover-routemesh:
    bun run {{ evm_atlas_generator }} --discover-routemesh
alias eadr := evm-atlas-discover-routemesh

# Check skill invocation metadata, README skill tables, and dependencies for drift
[group("checks")]
@skill-check:
    ai-skillet doctor --root .
alias sc := skill-check

# Apply safe fixes for skill invocation metadata, README skill tables, and dependencies
[group("checks")]
@skill-fix:
    ai-skillet doctor --root . --fix-safe
alias sf := skill-fix

# Check source-owned global skill installations and CLI metadata for drift
[group("checks")]
@publish-skills-check *args:
    bun run {{ publish_skills_script }} check {{ args }}
alias psc := publish-skills-check

# Exercise deterministic skill planning, apply guards, and command batching
[group("checks")]
@publish-skills-test:
    bun test {{ publish_skills_test }}
alias pst := publish-skills-test

# Type-check Bun TypeScript helper scripts without emitting JavaScript
[group("checks")]
@typescript-check:
    bunx --no-install tsc --noEmit
alias tsc := typescript-check

# ---------------------------------------------------------------------------- #
#                                   PUBLISH                                    #
# ---------------------------------------------------------------------------- #

# Plan, claim every target repository, and apply catalog skill publication; pass --skill NAME to scope
[group("publish")]
[positional-arguments]
[script("bash")]
publish-skills *args:
    set -euo pipefail
    work_dir=$(mktemp -d "${TMPDIR:-/tmp}/publish-skills.XXXXXX")
    trap 'rm -rf "$work_dir"' EXIT
    plan_file="$work_dir/plan.json"
    claims_file="$work_dir/claims.tsv"
    tab=$(printf '\t')

    bun run {{ publish_skills_script }} plan --json "$@" >"$plan_file"
    if [ "$(jq -r '.version' "$plan_file")" != 2 ]; then
        echo "error: expected publish-skills plan JSON version 2" >&2
        exit 1
    fi
    if [ "$(jq -r '.clean' "$plan_file")" = true ]; then
        echo "No publish-skills drift; nothing to publish."
        exit 0
    fi
    head=$(jq -r '.head' "$plan_file")
    echo "Plan head: $head"

    # One row per claim scope, keyed by the physical repository root.
    jq -r '.repos[] | .root as $root | .paths[] | [$root, .scope, .path] | @tsv' "$plan_file" |
        while IFS="$tab" read -r root scope claim_path; do
            canonical_root=$(cd "$root" && pwd -P)
            printf '%s\t%s\t%s\n' "$canonical_root" "$scope" "$claim_path"
        done >"$claims_file"
    root_count=$(cut -f1 "$claims_file" | sort -u | wc -l | tr -d ' ')

    start_args=()
    bundle_args=()
    while IFS="$tab" read -r root scope claim_path; do
        if [ "$scope" = recursive ]; then
            start_args+=(--recursive "$claim_path")
            bundle_args+=(--recursive "$root/$claim_path")
        else
            start_args+=("$claim_path")
            bundle_args+=("$root/$claim_path")
        fi
    done <"$claims_file"

    if [ "$root_count" -eq 0 ]; then
        echo "No target repository paths to claim; the helper's process lock covers CLI metadata cleanup."
    else
        if [ "$root_count" -eq 1 ]; then
            claim_root=$(cut -f1 "$claims_file" | sort -u)
            claim_command=(ai-coord start 'publish catalog skills' "${start_args[@]}")
        else
            claim_root=$PWD
            claim_command=(ai-coord bundle start 'publish catalog skills' "${bundle_args[@]}")
        fi
        if claim_output=$(cd "$claim_root" && "${claim_command[@]}"); then claim_rc=0; else claim_rc=$?; fi
        case "$claim_rc:$claim_output" in
            0:READY*) printf '%s\n' "$claim_output" ;;
            *)
                printf '%s\n' "$claim_output" >&2
                echo 'Target claims are not READY: run `ai-coord wait`, then rerun `just publish-skills`.' >&2
                exit 1
                ;;
        esac
    fi

    bun run {{ publish_skills_script }} apply --expected-head "$head" "$@"
    if [ "$root_count" -gt 0 ]; then
        echo 'Next: commit and push the reported global paths per repository, then run `ai-coord done` once.'
    fi

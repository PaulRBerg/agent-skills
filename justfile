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
skill_invocation_script := "scripts/sync-invocation-policy.ts"
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

# Run every repository test suite
[group("checks")]
@test: python-test shell-test bats-test

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

# Refresh atlas-overlays.json routeMesh flags through the routemesh CLI (network call)
[group("checks")]
@evm-atlas-discover-routemesh:
    bun run {{ evm_atlas_generator }} --discover-routemesh
alias eadr := evm-atlas-discover-routemesh

# Check SKILL.md invocation fields against agents/openai.yaml
[group("checks")]
@skill-invocation-check:
    bun run {{ skill_invocation_script }}
alias sic := skill-invocation-check

# Update agents/openai.yaml invocation policy from SKILL.md
[group("checks")]
@skill-invocation-fix:
    bun run {{ skill_invocation_script }} --fix
alias sif := skill-invocation-fix

# Check skill dependency declarations
[group("checks")]
@skill-dependencies-check:
    ai-skillet doctor --root . --dependencies-only
alias sdc := skill-dependencies-check

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
@typescript-check:
    bunx --no-install tsc --noEmit
alias tsc := typescript-check

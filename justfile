set allow-duplicate-variables
set allow-duplicate-recipes
set shell := ["bash", "-euo", "pipefail", "-c"]
set unstable

# ---------------------------------------------------------------------------- #
#                                  VARIABLES                                   #
# ---------------------------------------------------------------------------- #

prettier := "bunx --no-install prettier"
prettier_globs := "\"**/*.md\""
evm_atlas_generator := "scripts/generate-evm-atlas.mjs"
skill_invocation_script := "scripts/sync-invocation-policy.mjs"
publish_skills_script := "scripts/publish-skills.mjs"
publish_skills_test := "scripts/test-publish-skills.mjs"
commit_paths_test := "skills/commit/scripts/test-commit-paths.sh"
codex_handoff_runner_test := "skills/codex-handoff/scripts/test-run-codex-handoff.sh"
codex_handoff_wave_test := "skills/codex-handoff/scripts/test-watch-codex-wave.py"

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

# Check Markdown formatting
[group("checks")]
@prettier-check +globs=prettier_globs:
    {{ prettier }} \
        --check \
        --cache \
        --log-level warn \
        --no-error-on-unmatched-pattern \
        {{ globs }}
alias pc := prettier-check

# Format Markdown
[group("checks")]
@prettier-write +globs=prettier_globs:
    {{ prettier }} \
        --write \
        --cache \
        --log-level warn \
        --no-error-on-unmatched-pattern \
        {{ globs }}
alias pw := prettier-write

# Run staged-file checks
[group("checks")]
@pre-commit:
    sh .husky/pre-commit

# Exercise isolated-index atomic commits and shared-index reconciliation
[group("checks")]
@commit-paths-test:
    bash {{ commit_paths_test }}

# Exercise the Codex handoff runner and wave watcher
[group("checks")]
@codex-handoff-test:
    bash {{ codex_handoff_runner_test }}
    uv run python {{ codex_handoff_wave_test }}

# Regenerate evm-atlas references from crypto-registry's canonical chain JSON + atlas overlays
[group("checks")]
@evm-atlas-generate:
    node {{ evm_atlas_generator }} --write
alias eag := evm-atlas-generate

# Check evm-atlas generated references are current
[group("checks")]
@evm-atlas-check:
    node {{ evm_atlas_generator }} --check
alias eac := evm-atlas-check

# Refresh atlas-overlays.json routeMesh flags through the routemesh CLI (network call)
[group("checks")]
@evm-atlas-discover-routemesh:
    node {{ evm_atlas_generator }} --discover-routemesh
alias eadr := evm-atlas-discover-routemesh

# Check SKILL.md invocation fields against agents/openai.yaml
[group("checks")]
@skill-invocation-check:
    node {{ skill_invocation_script }}
alias sic := skill-invocation-check

# Update agents/openai.yaml invocation policy from SKILL.md
[group("checks")]
@skill-invocation-fix:
    node {{ skill_invocation_script }} --fix
alias sif := skill-invocation-fix

# Check source-owned global skill installations and CLI metadata for drift
[group("checks")]
@publish-skills-check *args:
    node {{ publish_skills_script }} check {{ args }}
alias psc := publish-skills-check

# Exercise deterministic skill planning, apply guards, and command batching
[group("checks")]
@publish-skills-test:
    node --test {{ publish_skills_test }}
alias pst := publish-skills-test

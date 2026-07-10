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
    nlx husky
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
    nlx lint-staged

# Regenerate evm-atlas generated references from crypto-registry + atlas overlays
[group("checks")]
@evm-atlas-generate:
    node {{ evm_atlas_generator }} --write
alias eag := evm-atlas-generate

# Check evm-atlas generated references are current
[group("checks")]
@evm-atlas-check:
    node {{ evm_atlas_generator }} --check
alias eac := evm-atlas-check

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

# ---------------------------------------------------------------------------- #
#                                  PUBLISHING                                  #
# ---------------------------------------------------------------------------- #

# Publish staged skills, install them into their declared targets, then commit and push ~/.agents
[group("publishing")]
[script("bash")]
sync:
    set -euo pipefail

    repo_root="$(git rev-parse --show-toplevel)"
    agents_root="$HOME/.agents"

    # ccc lives in this stable, chezmoi-managed file (also sourced by ~/.zshrc).
    user_dir="${XDG_CONFIG_HOME:-$HOME/.config}/prb"
    test -f "$user_dir/agents.sh" || { echo "missing $user_dir/agents.sh (needed for ccc)" >&2; exit 1; }
    # shellcheck disable=SC1090
    source "$user_dir/agents.sh"

    # 1. Commit agent-skills: stage what you want first; this commits exactly the
    #    staged index with an AI message. Dirty but nothing staged -> stop.
    if ! git -C "$repo_root" diff --cached --quiet; then
        ( cd "$repo_root" && ccc --staged )
    elif [ -n "$(git -C "$repo_root" status --porcelain)" ]; then
        echo "agent-skills has changes but nothing staged." >&2
        echo "Stage what you want to publish (git add ...), then rerun 'just sync'." >&2
        git -C "$repo_root" status --short >&2
        exit 1
    fi

    # 2. Publish so 'skills add' (install-all) fetches the latest from GitHub.
    git -C "$repo_root" push
    sleep 5  # give GitHub a moment to serve the pushed commit

    # 3. install-all needs a clean ~/.agents tree; refuse to guess about churn.
    test -d "$agents_root" || { echo "missing $agents_root" >&2; exit 1; }
    if [ -n "$(git -C "$agents_root" status --porcelain)" ]; then
        echo "~/.agents has uncommitted changes; commit or clean them first, then rerun 'just sync'." >&2
        git -C "$agents_root" status --short >&2
        exit 1
    fi

    # 4. Install latest skills into their declared targets.
    cd "$agents_root"
    just install-all PaulRBerg/agent-skills

    # 5. Commit the sync result (AI message via ccc) and push.
    ccc
    git -C "$agents_root" push
alias s := sync

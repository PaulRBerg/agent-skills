set allow-duplicate-variables
set allow-duplicate-recipes
set shell := ["bash", "-euo", "pipefail", "-c"]
set unstable

@default:
    just --list
alias d := default

[group("checks")]
[doc("Install Husky git hooks for this checkout")]
@hooks-install:
    nlx husky
alias hi := hooks-install

@install-deps: install-uv
alias id := install-deps

@install-uv:
    curl -LsSf https://astral.sh/uv/install.sh | sh
alias iu := install-uv

@mdformat-check:
    uvx --with mdformat-gfm --with mdformat-frontmatter mdformat --check .
alias mc := mdformat-check

@mdformat-write:
    uvx --with mdformat-gfm --with mdformat-frontmatter mdformat .
alias mw := mdformat-write

[group("checks")]
[doc("Run staged-file checks")]
pre-commit:
    nlx lint-staged
alias pc := pre-commit

[group("skills")]
[script("bash")]
[doc("Move skills/<skill> to shelved/<skill>")]
shelve skill:
    set -euo pipefail
    skill='{{ skill }}'
    case "$skill" in ''|*[!a-z0-9_-]*) printf '{{ RED }}invalid skill name: %s{{ NORMAL }}\n' "$skill" >&2; exit 64;; esac
    if [ -n "$(git status --porcelain)" ]; then
        printf '{{ RED }}working tree has uncommitted changes; commit or stash first{{ NORMAL }}\n' >&2
        git status --short >&2
        exit 1
    fi
    test -d "skills/$skill" || { printf '{{ RED }}missing skills/%s{{ NORMAL }}\n' "$skill" >&2; exit 1; }
    test ! -e "shelved/$skill" || { printf '{{ RED }}already exists: shelved/%s{{ NORMAL }}\n' "$skill" >&2; exit 1; }
    mkdir -p shelved
    mv "skills/$skill" "shelved/$skill"
    git add -A
    git commit -m "chore: shelve $skill skill"
    printf '{{ GREEN }}Shelved and committed %s.{{ NORMAL }}\n' "$skill"
alias sh := shelve

[group("checks")]
[doc("Check SKILL.md invocation fields against agents/openai.yaml")]
skill-invocation-check:
    node scripts/sync-invocation-policy.mjs
alias sic := skill-invocation-check

[group("checks")]
[doc("Update agents/openai.yaml invocation policy from SKILL.md")]
skill-invocation-fix:
    node scripts/sync-invocation-policy.mjs --fix
alias sif := skill-invocation-fix

# Publish staged skills, install into ~/.agents, sync ~/.claude, commit+push there
[doc("Stage your changes first; commits them (ccc --staged), pushes, installs in ~/.agents, syncs ~/.claude, commits+pushes there")]
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

    # 4. Install latest skills, then refresh ~/.claude symlinks.
    cd "$agents_root"
    just install-all PaulRBerg/agent-skills
    just sync-claude

    # 5. Commit the sync result (AI message via ccc) and push.
    ccc
    git -C "$agents_root" push
alias s := sync

[group("skills")]
[script("bash")]
[doc("Move shelved/<skill> to skills/<skill>")]
unshelve skill:
    set -euo pipefail
    skill='{{ skill }}'
    case "$skill" in ''|*[!a-z0-9_-]*) printf '{{ RED }}invalid skill name: %s{{ NORMAL }}\n' "$skill" >&2; exit 64;; esac
    if [ -n "$(git status --porcelain)" ]; then
        printf '{{ RED }}working tree has uncommitted changes; commit or stash first{{ NORMAL }}\n' >&2
        git status --short >&2
        exit 1
    fi
    test -d "shelved/$skill" || { printf '{{ RED }}missing shelved/%s{{ NORMAL }}\n' "$skill" >&2; exit 1; }
    test ! -e "skills/$skill" || { printf '{{ RED }}already exists: skills/%s{{ NORMAL }}\n' "$skill" >&2; exit 1; }
    mkdir -p skills
    mv "shelved/$skill" "skills/$skill"
    git add -A
    git commit -m "chore: unshelve $skill skill"
    printf '{{ GREEN }}Unshelved and committed %s.{{ NORMAL }}\n' "$skill"
alias u := unshelve

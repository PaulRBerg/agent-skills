set allow-duplicate-variables
set allow-duplicate-recipes
set shell := ["bash", "-euo", "pipefail", "-c"]
set unstable

@default:
    just --list

@install-deps: install-uv

@install-uv:
    curl -LsSf https://astral.sh/uv/install.sh | sh

[group("skills")]
[script("bash")]
[doc("Move skills/<skill> to shelved/<skill>")]
shelve skill:
    set -euo pipefail
    skill='{{ skill }}'
    case "$skill" in ''|*[!a-z0-9_-]*) printf '{{ RED }}invalid skill name: %s{{ NORMAL }}\n' "$skill" >&2; exit 64;; esac
    test -d "skills/$skill" || { printf '{{ RED }}missing skills/%s{{ NORMAL }}\n' "$skill" >&2; exit 1; }
    test ! -e "shelved/$skill" || { printf '{{ RED }}already exists: shelved/%s{{ NORMAL }}\n' "$skill" >&2; exit 1; }
    mkdir -p shelved
    mv "skills/$skill" "shelved/$skill"
    printf '{{ GREEN }}Shelved %s.{{ NORMAL }} {{ YELLOW }}Remove it from README.md if it was listed.{{ NORMAL }}\n' "$skill"

[group("skills")]
[script("bash")]
[doc("Move shelved/<skill> to skills/<skill>")]
unshelve skill:
    set -euo pipefail
    skill='{{ skill }}'
    case "$skill" in ''|*[!a-z0-9_-]*) printf '{{ RED }}invalid skill name: %s{{ NORMAL }}\n' "$skill" >&2; exit 64;; esac
    test -d "shelved/$skill" || { printf '{{ RED }}missing shelved/%s{{ NORMAL }}\n' "$skill" >&2; exit 1; }
    test ! -e "skills/$skill" || { printf '{{ RED }}already exists: skills/%s{{ NORMAL }}\n' "$skill" >&2; exit 1; }
    mkdir -p skills
    mv "shelved/$skill" "skills/$skill"
    printf '{{ GREEN }}Unshelved %s.{{ NORMAL }} {{ YELLOW }}Bring it up to current rules and add it to README.md.{{ NORMAL }}\n' "$skill"

# Commit and push, sync skills to ~/.agents, commit again
[group("sync")]
[script("zsh")]
[doc("Commit and push here, install skills in ~/.agents, commit there")]
sync:
    source ~/.zshrc 2>/dev/null

    # Commit in agent-skills repo
    ccc

    # Push so the install below pulls the fresh commit (skills installs from GitHub)
    git push

    # Give GitHub a moment to serve the pushed commit
    sleep 5

    # Switch to ~/.agents
    cd ~/.agents
    echo "📂 Changed directory to ~/.agents"

    # Commit uncommitted changes if any
    if [[ -n "$(git status --porcelain)" ]]; then
        ccc
    fi

    # Install skills from agent-skills repo
    just install-all PaulRBerg/agent-skills

    # Commit the installed skills
    ccc
alias s := sync

@mdformat-check:
    uvx --with mdformat-gfm --with mdformat-frontmatter mdformat --check .
alias mc := mdformat-check

@mdformat-write:
    uvx --with mdformat-gfm --with mdformat-frontmatter mdformat .
alias mw := mdformat-write

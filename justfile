set allow-duplicate-variables
set allow-duplicate-recipes
set shell := ["bash", "-euo", "pipefail", "-c"]
set unstable

@default:
    just --list

@install-deps: install-uv

@install-uv:
    curl -LsSf https://astral.sh/uv/install.sh | sh

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

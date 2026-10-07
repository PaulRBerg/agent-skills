# See https://github.com/PaulRBerg/devkit/blob/main/just/base.just
import "./node_modules/@prb/devkit/just/base.just"
import "./node_modules/@prb/devkit/just/npm.just"

# ---------------------------------------------------------------------------- #
#                                 DEPENDENCIES                                 #
# ---------------------------------------------------------------------------- #

# https://github.com/jqlang/jq
jq := require("jq")

# ---------------------------------------------------------------------------- #
#                                    RECIPES                                   #
# ---------------------------------------------------------------------------- #

# Default recipe
@default:
    just --list

# Build the project
@build:
    just clean
    just tsc-build
alias b := build

# Clean the dist directory
@clean:
    bunx del-cli dist
    echo "✅ Cleaned build files"

# Run tests
test *args:
    bun vitest run --hideSkippedTests {{args}}
alias t := test

# Run tests with UI
test-ui *args:
    bun vitest --hideSkippedTests --ui {{args}}
alias tui := test-ui

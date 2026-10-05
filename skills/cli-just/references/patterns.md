# Justfile Patterns & Conventions

Match the existing justfile's banner, grouping, recipe order, aliases, and default behavior. For a new standalone file,
start from the linked [standalone template](../examples/standalone.just) or [devkit template](../examples/devkit.just)
for its task-runner shape. Put aliases next to their target. Keep an explicit `just --list` default for older or
established layouts.

Use small recipes and private prerequisites. Offer check/write pairs for operations that change state. When practical,
run the check path before the write path. Group list output only when it improves the repository's existing
organization.

Use `[no-cd]`, a working directory, or an OS guard only when the recipe's execution contract needs it.

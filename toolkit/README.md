# Toolkit

The CLI and local-app subtree of [agent-skills](../README.md): agent-oriented command-line tools and local applications
for shared-working-tree coordination, notifications, commits, skill catalogs, and task handoffs.

- [ai-commit](commit/): prepare and commit immutable Git snapshots safely in shared working trees.
- [ai-coord](coord/): coordinate parallel Codex and Claude Code agents.
- [ai-handoff](handoff/): create and archive agent task handoffs.
- [ai-notify](notify/): deliver desktop notifications for Claude Code and Codex CLI.
- [ai-skillet](skillet/): inspect and maintain agent-skill catalogs.
- [Coordination dashboard](apps/coord-dashboard/): local live view of ai-coord state and a read-only task-handoff
  viewer, running at [https://coord.localhost](https://coord.localhost) (handoffs at
  [https://coord.localhost/handoffs](https://coord.localhost/handoffs)).

For contribution guidance and validation, see [AGENTS.md](AGENTS.md).

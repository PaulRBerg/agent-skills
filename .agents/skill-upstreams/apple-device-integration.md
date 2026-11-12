# apple-device-integration

- **Relationship:** confirmed fork of Apple's `device-interaction` skill, exported from Xcode 27.2, build `27B52`, on
  2026-11-06. The installed Apple export is the primary source.
- **Official source:** `xcrun agent skills export --output-dir <absolute-temporary-directory>` writes
  `device-interaction/SKILL.md`. See
  [Apple's Xcode agent documentation](https://developer.apple.com/documentation/xcode/giving-external-agents-access-to-xcode).
- **Tracking repository:**
  [raven/xcode-agent-skills](https://github.com/raven/xcode-agent-skills/tree/main/skills/device-interaction) mirrors
  Apple's exports. It is a third-party mirror, not an Apple repository.
- **Last reconciled mirror commit:** `a335ce50fa28a9b81d4a0c64b4fba2ee7eb4bf99`.
- **Base evidence:** the mirror's `skills/device-interaction/SKILL.md` matches the official export except for the order
  of the `name` and `description` frontmatter fields. The Markdown body is byte-identical.
- **Official export SHA-256:** `fa6cc5115c15f43fd5b2718c8705bec6cc0d6932309bf0df47ede28c0592b044`.
- **Mirror base SHA-256:** `584d29b72201066f2acca85de958eb477d4f25e87e5c795d4ba870625f6173bc`.
- **Apple Git revision:** unknown. Xcode exports a skill bundle, not a public Apple Git commit.

## Mapping and Local Behavior

Map the mirror's `skills/device-interaction/SKILL.md` to the local `skills/apple-device-integration/SKILL.md`. The local
`agents/openai.yaml` is catalog metadata and has no upstream mapping.

Preserve these local changes:

- Catalog compatibility, coordination exemption, description, and invocation metadata.
- User-instruction priority and target-repository rules for builds, fixtures, privacy, device ownership, and cleanup.
- Task-owned simulator checks, returned-device verification, and task authorization for physical devices and live
  services.
- Native subagent delegation in Claude Code and Codex, with parent-owned session lifecycle and no application-source
  edits by the verification agent.
- Parent-session fallback when Xcode does not approve a delegated verifier's connection, and verifier reporting that
  applies to the parent session and to a delegated verifier.
- `DeviceInteractionSynthesize`, matching the live MCP catalog, instead of the export's `DeviceEventSynthesize`.
- Safe runtime diagnostics instead of instructions to add temporary print statements.
- The completion report and the distinction between simulator verification and physical-device installation.

## Safe Update Boundary

Compare mirror changes from `a335ce50fa28a9b81d4a0c64b4fba2ee7eb4bf99` over the mapped entrypoint only. Before accepting
a new mirror baseline, export the skill from the installed Xcode and verify the changed guidance against that official
bundle and the live MCP catalog. A mirror update alone does not establish a new Apple baseline. Preserve local metadata
and repository-specific build and device ownership requirements.

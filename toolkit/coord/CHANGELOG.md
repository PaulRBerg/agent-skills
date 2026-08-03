# Changelog

## Unreleased

- Allow literal in-repository symlink scopes without dereferencing their targets.
- Preserve literal scope identity and prevent claims from moving across repositories.
- Make concurrent first-run database initialization safe.
- Honor client configuration roots and validate complete hook definitions.
- Replace hook configuration files atomically while preserving symlink targets and permissions.
- Reject malformed lifecycle and legacy records without polluting coordination state.

## 0.1.0

- Initial release.

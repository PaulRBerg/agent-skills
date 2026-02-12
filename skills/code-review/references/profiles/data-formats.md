# Data Formats Profile

Load when code parses or emits structured data.

## Checks

- `DF-001` CSV formula injection (`HIGH`): cells starting with formula tokens exported unsanitized.
- `DF-002` Unsafe YAML handling (`CRITICAL`): unsafe loader on untrusted YAML.
- `DF-003` Schema-free parsing (`HIGH`): JSON/YAML accepted without structural validation.
- `DF-004` Numeric precision loss (`HIGH`): large identifiers/amounts coerced into unsafe number types.
- `DF-005` Binary parser trust (`HIGH`): no length/magic-byte/offset validation.
- `DF-006` Encoding ambiguity (`MEDIUM`): implicit charset assumptions can corrupt data or bypass checks.

## Evidence Expectations

- Show malformed payload and resulting failure or exploit condition.

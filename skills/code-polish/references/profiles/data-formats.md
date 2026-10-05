# Data Formats Profile

Load when the diff touches structured-data parsing or emission.

## Checks

- `DF-001` CSV formula injection (`HIGH`): Exported cells start with formula tokens and are not sanitized.
- `DF-002` Unsafe YAML handling (`CRITICAL`): An unsafe loader processes untrusted YAML.
- `DF-003` Schema-free parsing (`HIGH`): Parsing accepts JSON/YAML without structural validation.
- `DF-004` Numeric precision loss (`HIGH`): Conversion puts large identifiers/amounts into unsafe number types.
- `DF-005` Binary parser trust (`HIGH`): Parsing does not validate length/magic-byte/offset.
- `DF-006` Encoding ambiguity (`MEDIUM`): Implicit charset assumptions can corrupt data or bypass checks.

## Evidence Expectations

- Show the malformed payload and the resulting failure or exploit condition.

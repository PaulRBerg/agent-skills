# Number Formatting Profile

Load when the diff changes human-facing numeric display: quantities, money, percentages, rates, units, or compact K/M/B
formats.

## Rules

Apply these defaults only when polishing human-facing values within the fixed scope. Explicit user requirements and
deliberate repository or domain contracts take precedence. Inspect shared formatters, their tests, and their consumers
before changing behavior. Reuse the shared formatter instead of scattering `Intl` or `toFixed` calls.

Use commas for thousands grouping and a dot for the decimal mark in English displays by default. Trim insignificant
trailing zeros, normalize negative zero, and usually show no more than two decimal places for ordinary values and
percentages. For example, show `0.00` as `0`, `12.5000` as `12.5`, and `12345.678` as `12,345.68`. Do not impose a
blanket precision cap.

For quantities with `0 < abs(value) < 0.01`, keep leading fractional zeros through up to two nonzero digits. For
example, show `0.0001234` as `0.00012` and keep `-0.00102` as `-0.00102`. This rule does not mean significant-digit
rounding. For rates with an absolute value below one percent, preserve meaningful digits under the existing domain
policy for significant or nonzero digits.

Money usually rounds to cents without padding insignificant zeros. Preserve deliberate fixed precision for invoices and
accounting, precision-bearing measurements, progress widgets, and animated counters. A domain contract may round a
monetary sub-cent total to zero, but do not lose tiny token or rate information. Keep existing compact K/M/B formats,
thresholds, and letter case. Prefer full grouped values when no project convention applies. Preserve units, fraction or
percent scaling, signed changes, domain-specific floor or ceiling rules, and the distinction between missing values and
zero.

Round only for presentation. Preserve exact decimal or `BigInt` calculations, sorting, risk thresholds, inputs, wire and
storage values, CSV output, and copyable exact values. Never coerce an exact balance to `Number`. Verify changed
behavior with representative zero, tiny positive and negative values (including internal zeros), large values, rounding
and grouping boundaries, missing values, and relevant domain exceptions. Use proportional existing tests. Do not add
verification machinery or blanket tests for prose.

Prefer the platform formatter or an established decimal library over hand-rolled rounding, grouping, or underflow
handling. Add a dependency only through the repository's dependency workflow.

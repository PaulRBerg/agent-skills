# PDF Forms

Choose the branch from the document structure; do not guess from appearance.

## Inspect

```sh
uv run "<skill-dir>/scripts/form.py" inspect "input.pdf"
```

The JSON reports whether an AcroForm exists, whether XFA is present, and each field's fully qualified name, type,
current and default values, options, flags, page, and rectangle.

Route by result:

- AcroForm without XFA or signatures: fill known fields.
- Flat page with no AcroForm: use coordinate overlays after rendering and calibration.
- XFA or signature fields: stop. This helper intentionally does not preserve or generate those workflows.

## Fill an AcroForm

Create a JSON object keyed by the exact field names returned by `inspect`:

```json
{
  "person.name": "Ada Lovelace",
  "preferences.language": "Romanian",
  "topics": ["tax", "finance"]
}
```

Fill to a new output:

```sh
uv run "<skill-dir>/scripts/form.py" fill "input.pdf" "values.json" "filled.pdf"
```

Use `--flatten` only for a final, non-editable delivery. Flattening draws field appearances, removes widget annotations,
and removes the AcroForm dictionary. Verify a rendered output because stored values alone do not prove visibility.

The helper rejects unknown fields, XFA, signatures, input/output aliasing, and an existing output unless `--force` is
explicitly supplied.

## Overlay a flat form

Render the page first and calibrate in PDF points from the lower-left corner. Page numbers are one-based. Create a JSON
array:

```json
[
  { "page": 1, "x": 126, "y": 618, "text": "Ada Lovelace", "font_size": 10 },
  { "page": 2, "x": 90, "y": 144, "text": "București" }
]
```

Apply it to a new output:

```sh
uv run "<skill-dir>/scripts/form.py" overlay "input.pdf" "placements.json" "filled-flat.pdf"
```

The default font is `/System/Library/Fonts/Supplemental/Arial.ttf`, which supports Romanian text. `font_size` defaults
to 10. Placements are single-line text; add separate entries instead of relying on wrapping.

Render every affected page after overlaying. Check baseline, clipping, diacritics, rotation, and whether the visual
position still matches at ordinary and high zoom. The helper preserves page count and dimensions but cannot determine
whether coordinates are semantically correct.

## Validate

For either branch:

1. Require the helper's success JSON and qpdf integrity.
2. Re-run `inspect` for editable AcroForms and compare intended values.
3. Render every affected page and visually inspect field appearances or overlay placement.
4. Confirm page count and dimensions match the input.
5. Keep the untouched input alongside the validated output.

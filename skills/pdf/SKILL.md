---
argument-hint: "[file ...]"
compatibility: Requires macOS, uv, Poppler, qpdf, Ghostscript, OCRmyPDF with Tesseract language data, and img2pdf.
name: pdf
description:
  "Use when PDF files are the primary input or output: read, compare, reconcile, extract text/tables/images, OCR scans,
  fill forms, split, merge, rotate, rename, compress, or convert between PDF and images. Optimized for private
  financial, tax, legal, and health documents on macOS."
---

# PDF

Process PDFs locally on macOS with exact extraction, source preservation, deliberate tool routing, and structural plus
semantic validation.

## Invariants

1. Keep document contents local unless the user explicitly authorizes an upload or external disclosure. Package and
   language-data downloads do not authorize document disclosure.
2. Preserve every original PDF byte-for-byte. Write a sibling output, copy, or explicitly named destination unless the
   user authorizes destructive replacement.
3. Preserve monetary values, identifiers, dates, signs, and displayed precision as strings. Use `decimal.Decimal` for
   arithmetic; never infer missing rows or silently discard headers, footnotes, continuation lines, or boundary pages.
4. Inspect structure and representative renders before choosing a transformation. Use the smallest tool that preserves
   the required layout, forms, annotations, and image quality.
5. Validate every written PDF structurally and against task semantics. A command exiting successfully is not evidence
   that extracted rows, totals, page boundaries, form appearances, or visual layout are correct.
6. Keep reports concise for private financial, tax, legal, and health documents. Prefer counts, reconciliations, and
   file references over raw sensitive rows unless the rows materially support the task or the user asks for them.

## Profile First

Resolve the skill directory from this `SKILL.md`, then profile every unknown input:

```sh
uv run "<skill-dir>/scripts/profile.py" "<input.pdf>"
```

The helper emits schema-versioned JSON with integrity, encryption, page geometry/rotation, image counts, and per-page
text coverage without document text. Stop on `password_required`; password handling is outside this skill.

Render representative pages whenever layout, cropping, OCR quality, signatures, or form placement matters. Include the
first and last page, every structural boundary, and any page behind a discrepancy.

## Route by Evidence

| Need                                      | Preferred route                                                          |
| ----------------------------------------- | ------------------------------------------------------------------------ |
| Quick reading or page-aware extraction    | Host PDF reader when available, then `pdftotext -layout`                 |
| Coordinates, columns, or difficult tables | Poppler bounding boxes, then `pdfplumber` through `uv run`               |
| Image-only or materially incomplete text  | OCRmyPDF with Tesseract; default languages `eng+ron`                     |
| Merge, split, rotate, or integrity checks | qpdf                                                                     |
| Render pages or extract embedded images   | `pdftocairo` or `pdfimages`                                              |
| Convert ordered images into a PDF         | img2pdf                                                                  |
| Reduce size                               | qpdf lossless rewrite first; Ghostscript only for an accepted lossy pass |
| Inspect, fill, flatten, or overlay forms  | Read [references/forms.md](references/forms.md) first                    |

Read [references/recipes.md](references/recipes.md) only when exact commands for the selected extraction,
transformation, OCR, image, comparison, or compression branch are needed.

## Execute and Reconcile

1. Profile inputs and identify whether each page is digital, scanned, mixed, rotated, or image-heavy.
2. Extract or transform into a new path. For tabular documents, retain page provenance and parse continuations across
   page breaks before assigning rows.
3. Reconcile financial and evidentiary output with every available invariant: page and row counts, opening/closing
   balances, inflows/outflows, subtotals, displayed totals, date coverage, and source hashes when provenance matters.
4. For comparisons, extract both sources independently, enumerate overlapping and unique facts, and render the pages
   behind every material disagreement. Distinguish a real discrepancy from an extraction failure.
5. For split or rename work, establish an old-to-new map from stable content identifiers. Copy by default, preserve
   contextual boundary pages when needed, and verify the first and last page of every result.
6. Validate outputs with qpdf, expected page count/dimensions, text coverage, representative renders, and the task's
   semantic invariants. Retain OCR sidecars or extraction intermediates only when they are requested or useful evidence.

Completion requires preserved originals, intentional outputs, successful structural checks, semantic reconciliation, and
a concise report of paths and evidence. Lead read-only reports with `### 📄 PDF — 🔎 inspected, no files written`; use
`### 📄 PDF — ✅ updated` only after all required validation passes, and `### 📄 PDF — ⛔ not deliverable` when a
required check fails.

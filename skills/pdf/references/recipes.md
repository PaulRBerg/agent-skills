# PDF Recipes

Load only the branch needed for the current task. Keep every output distinct from its input and quote paths.

## Inspect and extract

Start with the bundled factual profile, then inspect raw tool output only as needed:

```sh
uv run "<skill-dir>/scripts/profile.py" "input.pdf"
pdfinfo -box "input.pdf"
qpdf --check "input.pdf"
pdfimages -list "input.pdf"
```

Preserve reading order where possible:

```sh
pdftotext -layout "input.pdf" "output.txt"
pdftotext -f 3 -l 7 -layout "input.pdf" "pages-3-7.txt"
pdftotext -f 1 -l 1 -x 36 -y 72 -W 540 -H 648 -layout "input.pdf" "crop.txt"
pdftotext -bbox-layout "input.pdf" "layout.html"
```

Use bounding boxes to diagnose column interleaving or logical half-pages. Escalate to `pdfplumber` only when the Poppler
output cannot preserve the required table structure:

```sh
uv run --with 'pdfplumber>=0.11.10,<0.12' python - "input.pdf" <<'PY'
import json
import sys

import pdfplumber

with pdfplumber.open(sys.argv[1]) as document:
    print(json.dumps([page.extract_tables() for page in document.pages], ensure_ascii=False))
PY
```

Treat extracted tables as candidates, not truth. Rebuild wrapped descriptions and continuations, retain page numbers,
and reconcile exact row counts and totals against the PDF.

## Compare documents

Extract each input independently with the same appropriate route. Compare normalized facts while retaining the original
strings and page provenance. Report:

- facts present in both documents;
- facts unique to each document;
- materially different values, dates, identifiers, qualifications, or footnotes;
- pages rendered to distinguish source differences from extraction errors.

Do not infer that missing extracted text means missing source content. Inspect the relevant render or OCR coverage
first.

## OCR scans

Use OCR only for image-only or materially incomplete pages. The default covers English and Romanian:

```sh
ocrmypdf --output-type pdf --skip-text -l eng+ron --sidecar "ocr.txt" "input.pdf" "ocr.pdf"
qpdf --check "ocr.pdf"
pdftotext -layout "ocr.pdf" "ocr-check.txt"
```

Add `--rotate-pages` or `--deskew` only when profiling or rendered pages show the need. Add `--clean` only after
accepting that image processing may alter visual evidence. Re-render affected pages after any of these options.

## Render and extract images

Render pages for visual comparison:

```sh
pdftocairo -png -r 200 "input.pdf" "page"
pdftocairo -f 4 -l 4 -png -r 300 "input.pdf" "page-4"
```

Extract embedded images without rasterizing whole pages:

```sh
pdfimages -list "input.pdf"
pdfimages -all "input.pdf" "image"
```

Combine ordered images without recompressing them unnecessarily:

```sh
img2pdf "page-01.png" "page-02.jpg" --output "combined.pdf"
qpdf --check "combined.pdf"
```

Confirm image order, orientation, page dimensions, and representative renders.

## Merge, split, and rotate

Merge in explicit order:

```sh
qpdf --empty --pages "part-1.pdf" "part-2.pdf" -- "merged.pdf"
```

Extract a range or retain a boundary page for context:

```sh
qpdf "input.pdf" --pages . 1-5 -- "part-1.pdf"
qpdf "input.pdf" --pages . 5-10 -- "part-2-with-boundary.pdf"
```

Rotate selected pages clockwise:

```sh
qpdf "input.pdf" "rotated.pdf" --rotate=+90:1,3
```

Check every result and inspect its first and last page:

```sh
qpdf --check "output.pdf"
pdfinfo "output.pdf"
pdftotext -f 1 -l 1 -layout "output.pdf" -
```

## Rename a corpus

Build a complete old-to-new map before writing. Derive names from stable content such as issuer, document type, account
suffix, and covered date range. Detect collisions and ambiguous documents first. Copy to the new names unless the user
explicitly authorizes renaming originals, then profile both sides and compare hashes when a byte-identical copy is
expected.

## Compress

Try a lossless structural rewrite first:

```sh
qpdf --object-streams=generate --recompress-flate --compression-level=9 "input.pdf" "lossless.pdf"
```

Use Ghostscript only when a smaller lossy output is acceptable:

```sh
gs -sDEVICE=pdfwrite -dCompatibilityLevel=1.7 -dPDFSETTINGS=/ebook -dNOPAUSE -dQUIET -dBATCH \
  -sOutputFile="compressed.pdf" "input.pdf"
```

Compare byte size only after qpdf integrity, page count/dimensions, text extraction, forms/annotations where relevant,
and representative renders pass. Keep the original and the smallest acceptable validated output.

## Final validation

At minimum, require:

```sh
qpdf --check "output.pdf"
pdfinfo -box "output.pdf"
pdftotext -layout "output.pdf" "output.txt"
```

Add domain checks: exact totals and balances for statements, field values and appearances for forms, first/last pages
for splits, order and dimensions for image conversions, and visual comparison for OCR or compression.

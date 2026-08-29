# ADR 0003 — Deterministic structure extraction

Status: accepted

Date: 2026-08-24

## Context

ManuscriptDock needs a trustworthy representation of a manuscript before it can evaluate
submission readiness or generate format-specific outputs. This first representation must not
depend on a language model, change the immutable source snapshot, or imply greater certainty
than the source format permits.

## Decision

- Rust reads the immutable workspace snapshot and owns all parsers and result persistence.
- TEX is parsed from commands and environments; DOCX is parsed from its WordprocessingML
  document; PDF uses deterministic page/text extraction with an explicit `limited` quality.
- PDF processing first classifies the document and pages, then uses the MIT-licensed
  `pdf-inspector` default Rust pipeline for native font, position, column, heading and table
  extraction. It falls back to the existing `pdf-extract` font/ToUnicode mapping and lopdf
  content-stream paths without discarding the classification evidence. Parser failure alone is not
  treated as evidence that the PDF is scanned; mixed PDFs retain page-level, object-specific
  recognition candidates.
- OCR is not the first or universal extraction path. Readable native objects win. Missing text is
  routed to Chinese/English text OCR, missing formulas require a formula recognizer, and missing
  table structure requires a table recognizer. Future fusion records page, bounding box, object
  type, producer and confidence; recognized content only fills gaps and never silently overwrites
  reliable native content.
- PDF titles combine the visible first-page text with document metadata, preferring the visible
  title when metadata contains only its prefix. Author extraction combines PDF Author metadata
  with conservative first-page name-line recognition and excludes affiliations, contacts and URLs.
  Embedded PDF bookmarks take precedence over text heuristics for the section hierarchy.
- Abstract extraction accepts English and Chinese headings, punctuation variants such as
  `ABSTRACT—`, DOCX styles and TEX environments. If a PDF omits the visible “Abstract” label, a
  bounded first-page prose block after authors and affiliations can become an explicit candidate;
  the report warns that the author must confirm it.
- The versioned report records its source content hash and source snapshot version. It includes
  title, author candidates, abstract presence and candidate text, keyword presence, section
  outline, figure/table counts, references, common declaration types, page count when available,
  word count, and warnings.
- Rust verifies the snapshot size and SHA-256 fingerprint again immediately before parsing.
- Results are written below the workspace `analysis` directory. File names include the analysis
  schema version and source-hash prefix; a JSONL audit event records every completed analysis.
- The WebView receives the report but never the source path or an unrestricted file handle.
- Parser limitations are warnings, not silently repaired content. OCR and AI suggestions remain
  separate later capabilities. Structure analysis v4 added author and abstract candidate fields;
  v5 adds the structured PDF classification, confidence, native extraction path, table/column pages,
  encoding state and pages needing object recognition.

## Consequences

DOCX and TEX can provide stronger structural evidence than PDF. A PDF result is useful for
triage but is never presented as equivalent to source-aware extraction. Future parser changes
must increment the analysis version when they change the persisted contract or interpretation.

# ManuscriptDock MVP Development Plan

Status: complete (M0–M3)

Last updated: 2026-08-24

## Outcome

The first executable MVP proves the product's most important technical promise: a user can move
from manuscript selection to a locally previewable readiness report while the WebView remains
outside the local filesystem trust boundary.

The implemented flow is:

1. The user chooses a local DOCX, PDF, or TEX file through a native desktop dialog.
2. Rust validates that it is a supported file under the local size limit.
3. Rust returns an explicit selected, cancelled, or rejected outcome. A selected outcome contains
   only display-safe metadata: file name, format, size, and modified time.
4. The UI confirms that validation happened locally and that no upload occurred.
5. Rust creates an immutable local workspace, extracts a versioned deterministic structure
   report from its snapshot, and records the operation in the local audit log.
6. Rust verifies and composes signed generic rules, then saves an explainable JSON report and
   escaped HTML preview without transmitting the manuscript.

The full local path is deliberately not returned to the WebView.

## MVP milestones

### M0 — executable foundation (complete)

- Tauri 2.x desktop shell
- React 18, TypeScript 5, and Vite 5 frontend
- Rust workspace with a UI-independent manuscript domain crate
- Minimal Tauri capability declaration and restrictive content security policy
- Unit and interface-state tests
- CI checks for frontend and Rust workspaces

### M1 — immutable import and workspace record (complete)

- Calculate a content hash in Rust
- Create a local project record without changing the source manuscript
- Store only explicit local references and derived metadata
- Record user-visible audit events
- Recover safely after restart

### M2 — deterministic structure extraction (complete)

- Create a versioned working snapshot
- Extract sections, tables, figures, references, and declarations
- Report unsupported or ambiguous structures without silently rewriting content
- Keep deterministic validation separate from AI suggestions

Implemented format boundaries:

- TEX: source commands and environments, with comment stripping and nested-brace handling.
- DOCX: WordprocessingML paragraph styles, text, drawings, and tables inside the package.
- PDF: page-aware text extraction marked as `limited`; scanned or non-extractable content emits
  a visible OCR warning rather than an invented result.

All reports bind to the immutable snapshot version and SHA-256 content fingerprint. They are
stored as versioned JSON under the Rust-owned workspace and contain no filesystem paths.

### M3 — format and submission readiness (complete)

- Execute composable rule packs
- Produce explainable findings with source locations
- Generate previewable output snapshots
- Prevent external transmission until an explicit approval flow exists

Implemented MVP boundary:

- Two Ed25519-signed, composable coverage-C packs provide the international structure base and
  initial-submission stage rules.
- Each finding records its rule, signed pack, classification, semantic source location, and
  passed/warning/blocked/author-confirmation state.
- Every evaluation creates an immutable versioned JSON report and self-contained HTML preview.
- There is no outbound transmission command in this MVP. The report explicitly records that no
  transmission occurred. Approval UI will be added together with the first real connector, not
  as a decorative confirmation with no action behind it.

OCR, configurable language models, protected PWC review agents, preprint publication, and
institutional synchronization are later capabilities. They are not hidden inside the first
desktop milestone.

M0–M3 now form the first executable local MVP. Journal selection, exact journal rule downloads,
editable fixes, document transformation, export to a user-selected directory, and external
connectors remain subsequent product slices.

## Acceptance criteria for M0

- A supported local file can be selected in the packaged desktop runtime.
- Cancelling the native dialog does not create an error.
- Unsupported files and files over 250 MB produce a recoverable message.
- The frontend receives no absolute or relative source path.
- No network request is required for manuscript selection or validation.
- Frontend tests cover idle, selected, cancelled, and error states.
- Rust tests cover supported and unsupported extension classification.
- The repository check command passes on Node 24 LTS and Rust 1.93.

## Acceptance criteria for M1–M3

- Original source content remains unchanged and the snapshot fingerprint is verified before use.
- Restart recovery skips corrupt records without disclosing local paths.
- TEX, DOCX, and PDF produce deterministic structure reports with honest format limitations.
- Modified rule-pack bytes fail Ed25519 verification before execution.
- Findings retain classification, signed rule provenance, and semantic source location.
- Every readiness run creates a distinct local JSON and HTML output snapshot plus an audit event.
- No external transmission API is exposed by the MVP.

## Toolchain policy

- Development and CI target Node.js 24 LTS with npm 11.
- Rust is pinned to 1.93 with `rustfmt` and `clippy`.
- Dependency lock files are committed once generated.
- Runtime fonts, icons, and UI assets are bundled locally; the MVP loads no remote UI asset.

The local machine may temporarily run a newer development Node release, but release and CI
results are defined by the pinned targets above.

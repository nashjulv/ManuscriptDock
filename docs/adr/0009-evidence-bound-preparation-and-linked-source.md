# ADR 0009 — Evidence-bound preparation and linked sources

- Status: Accepted; implemented in V0.50 (local validation, not a release).
- Date: 2026-09-08
- Related: [repair plan](../first-use-flow-remediation-plan.md), ADR 0002, ADR 0005.

## Context

Basic structure checks were gated by journal preparation, while a completed run could be mistaken for submission readiness. PDF users could inspect findings but lacked a truthful route to an editable manuscript. A source-format change must not overwrite an immutable PDF snapshot or pretend that a different document has the same version history.

## Decision

Rust projects the material checklist, pending tasks and final readiness from the current workspace evidence. Frontend navigation consumes that projection. Local general checks are always available. Final export still requires a current target, current official requirements, required files, explicit author decisions and a matching final check.

A report context hash binds the manuscript version and hash, effective structure, target, requirement snapshot, material bindings, decisions, declaration plan and rule selection. Evidence changes invalidate final readiness. Package assembly rechecks readiness before publication and verifies copied files. Repeated exports receive distinct names. This is a consistency guard, not a claim of transactional protection against arbitrary simultaneous filesystem mutation.

Author decisions use `compliant`, `noncompliant`, `unknown` and `not_applicable`. Conditional exemptions require a reason and cannot replace mandatory files. Legacy boolean records are retained as `legacy_reviewed` and require a fresh decision. Decisions are immutable records as well as a current catalog projection.

Structure corrections are immutable overlays bound to the source hash, parser version and previous review ID. Authors and numbered caption objects feed a single effective decomposition used by checks, materials and knowledge provenance. Original manuscript and decomposition records remain intact.

A PDF-to-DOCX/TEX association creates a separate workspace starting at v1. Rust writes one canonical link containing both workspace IDs, source hashes and the author's association confirmation; either workspace can resolve the other. The link does not prove scientific equivalence and does not transfer approvals. Target selection, declarations, files and final checks must be established for the new workspace.

Same-format revisions retain the existing workspace/version chain. A previous target can be explicitly reconfirmed for the current version. Still-current requirement evidence is rebound to a new snapshot; material files can be explicitly rebound after integrity and eligibility checks. Old author decisions and final readiness do not carry over automatically.

## Consequences and boundaries

- The app can check an offline PDF immediately and still block an incomplete publisher package accurately.
- Source associations are discoverable in both directions without mixing immutable histories.
- Schema versions: structure 9, readiness 3, official requirements 3, material projection 5. Older requirement snapshots remain readable but need refresh before final readiness.
- PDF recognition uses native text coordinates and numbered captions. OCR, complete layout reconstruction and arbitrary scanned-PDF acceptance remain outside this change.
- Public journal topic labels are exposed for explanation; protected recommendation scores and internal reasoning remain private.
- User-supplied content remains in its original language. Surrounding actions, decisions, errors and reports support zh-CN and en.

## Validation

See [V0.50 execution and validation record](../first-use-flow-v050-validation.md) for current automated checks, native observations and unverified platform cases.

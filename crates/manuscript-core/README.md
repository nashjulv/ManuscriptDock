# Manuscript Core

This directory contains the reusable Rust core that remains independent from the Tauri window layer.

Current responsibilities:

- local manuscript and snapshot storage;
- file import and type validation;
- deterministic TEX, DOCX, and PDF structure parsing, including PDF font-map fallback,
  metadata, first-page author and abstract candidates, and embedded bookmark hierarchy;
- rule-pack validation and deterministic checks;
- Ed25519 trust-anchor enforcement and composable rule inheritance;
- versioned readiness JSON and escaped HTML preview snapshots;
- schema-versioned academic knowledge-body snapshots, independently versioned AI review reports,
  and validated cross-body assertion protocols;
- audit events and an explicit no-transmission MVP policy.

UI state, WebView components, and provider-specific presentation logic do not belong here.

The M0–M3 MVP validates a selected manuscript, creates and verifies an immutable snapshot,
extracts its deterministic structure, executes signed generic submission rules, and persists
local results. PDF analysis distinguishes a readable embedded text layer from a genuine OCR
candidate but does not run OCR. Document transformation, exact journal packs, external model
calls, PWC services, and outbound publication remain outside this crate's current executable scope.

The first knowledge-body slice generates a truthful local single-body snapshot from an immutable
workspace. It defines but does not fabricate AI review history or external research relationships;
those objects appear only when exact versions and assertion basis are available.

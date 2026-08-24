# ADR 0002 — Local Workspace Storage

Status: accepted

Date: 2026-08-24

## Context

ManuscriptDock must recover local projects after restart without giving the WebView arbitrary
filesystem access. Source manuscripts must remain unchanged, and every derived operation must
start from an identifiable snapshot.

## Decision

- Native file selection returns an opaque, one-time selection identifier and safe metadata.
  The selected source path remains only in a Rust-owned in-memory map.
- Creating a workspace consumes that pending selection through a narrow Tauri command.
- Rust copies the source into the application data directory under
  `workspace/projects/<workspace-id>/source/` and marks the copied snapshot read-only.
- The copy is hashed while it is written. The SHA-256 digest identifies the exact local input
  used by later stages.
- Each project contains a versioned `manifest.json` and appendable `audit.jsonl`. Neither file
  stores the original absolute source path.
- Project identifiers are UUIDs. Any identifier accepted by a future command must parse as a
  UUID before it is used to resolve a local path.
- Catalog loading isolates corrupt records: a broken project is skipped with a safe warning
  rather than preventing all other workspaces from opening.

## Consequences

- The original user file is never rewritten.
- The WebView cannot convert an arbitrary string into filesystem access.
- Disk use increases because the MVP preserves a local source snapshot per workspace.
- A future storage migration must preserve manifest schema versions and audit history.
- A database may later index the catalog, but manifests remain the portable source of truth for
  a workspace.

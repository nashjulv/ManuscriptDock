# ADR 0004 — Signed rule packs and readiness snapshots

Status: accepted

Date: 2026-08-24

## Context

Submission requirements must evolve independently from application branches, remain traceable to
the version used for a particular manuscript, and never allow an untrusted WebView or modified
local data file to redefine a professional check.

## Decision

- Rules are JSON data packs with schema version, stable ID, semantic version, layer, coverage
  level, submission stage, inheritance, source label, and typed rules.
- Every pack is verified with Ed25519 before parsing and execution. The desktop core contains
  only the public trust anchor. The private key used for the bundled MVP packs is not stored in
  the repository.
- Pack inheritance is composed in Rust. Missing dependencies, cycles, duplicate pack IDs, and
  duplicate rule IDs reject the whole evaluation instead of producing partial conclusions.
- Rule outcomes use four explicit states: passed, warning, blocked, and author confirmation.
  They retain the rule ID, pack ID, classification, message, and semantic source location.
- Every run creates a new immutable output directory containing a versioned JSON report and a
  self-contained, escaped HTML preview. The report binds to the source hash and snapshot version.
- The MVP has no outbound submission or publication command. Reports record
  `externalTransmission: not_performed`; future connectors require a separate approval design
  and audit decision before they can be enabled.

## Consequences

The MVP truthfully offers coverage level C: generic initial-submission preparation, not verified
journal-specific compliance. A release signing ceremony, protected production key, revocation
strategy, rollback policy, and PWC update transport are required before remote rule updates ship.


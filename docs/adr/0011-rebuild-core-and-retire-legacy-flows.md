# ADR 0011 — Rebuild the two-task product and retire legacy flows

- Status: accepted and implemented in V0.54
- Date: 2026-09-14
- Follow-up to: [ADR 0010](0010-local-journal-package-focus.md)

## Context

The user explicitly permits rewriting all prior code and interactions and deleting anything no
longer useful. The existing journal target requires a recommendation reference, and the large
workspace/UI modules mix submission preparation with knowledge-body, attestation and remote
model workflows. Preserving their interfaces would undermine the independent two-task design.

## Decision

- Rebuild UI routes, command DTOs, target selection and application services around journal
  matching and target-specific package preparation. A catalog-selected target has no mandatory
  recommendation history.
- Retain only demonstrated value: the desktop/toolchain foundation, narrow privilege boundary,
  immutable source integrity, useful parsing, rules, identity matching and selected UI primitives.
  Their current file layout, public APIs and state types need not be retained.
- Delete unused product modules, command registrations, dependencies, styles, assets and tests
  of retired behavior. Do not retain obsolete runtime code behind permanent feature flags.
- Preserve user manuscripts and exports independently of source-code compatibility. Because the
  earlier product version was not released, do not scan, expose, import or migrate its task data.
  Existing disk files are left untouched but are not part of the new runtime.
- Keep manual journal maintenance as repository file edits shipped with the application. No
  maintenance UI, cloud requirements service or author-side AI setup is added.
- Organize detailed execution in docs/implementation and plan product/reference/archive/release
  document separation. Current and historical claims must remain distinguishable.

## Relationship to existing decisions

This refines ADR 0010: preservation of data and integrity does not require preservation of old
code, IPC interfaces, page structure or complete legacy object models. Tauri/Rust and stable
installation identity remain; old lifecycle and knowledge-body behavior do not constrain the new
implementation. Existing ADRs remain historical evidence rather than a demand to retain retired
features. Any obsolete implementation can be removed within the approved product scope.

## Consequences

The change may replace large modules rather than patch them. Tests verify useful invariants and
new user outcomes; preserving test counts or obsolete contracts is not a goal. File safety and
real generated-document validation remain necessary, while legacy migration tests and code are
removed. This decision does not authorize deletion of user data, credentials, unrelated project
work or installation changes.

V0.54 implements this decision without a separate version increment: the early-task UI, IPC,
domain models, migration storage and tests are gone. No commit, push, deployment or real user-file
cleanup is part of this implementation record.

## References

- [Implementation index](../implementation/README.md)
- [Architecture](../implementation/architecture.md)
- [Removal and data-boundary plan](../implementation/rewrite-and-removal.md)
- [Work packages](../implementation/delivery-plan.md)
- [Documentation structure](../documentation-structure.md)

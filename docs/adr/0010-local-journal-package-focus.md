# ADR 0010 — Local journal matching and submission-package focus

- Status: accepted for product direction; implementation pending
- Date: 2026-09-14
- Current executable version: V0.52 (unchanged by this documentation change)

Implementation strategy is refined by [ADR 0011](0011-rebuild-core-and-retire-legacy-flows.md): useful integrity and user data remain, while obsolete code, interfaces and interactions may be fully replaced or deleted.

## Context

The author-facing product is narrowed to two independent tasks: recommend journals, and organize
a submission package for a selected journal. A known target must bypass recommendation. Local
files and folders are opened rather than presented as uploads. Public journal requirements may
be collected with AI assistance, then manually verified and maintained before client distribution.

## Decision

- Keep Tauri, Rust ownership of privileged operations, immutable sources and local audit records.
- Replace the lifecycle navigation with two task entries and progressive disclosure. Internal
  versions, checks and evidence remain available without becoming mandatory user steps.
- Keep public-rule discovery, AI draft extraction and human verification as team work. Edit
  structured rule files directly in the repository and ship them with the client. Do not build
  a maintenance UI, admin service, PWC integration or dedicated updater for the first release.
- Treat other submission websites as attributed reference material. Verified official evidence
  and permitted reuse are required before their assertions become published mandatory rules.
- Bundle verified journal rules using the existing integrity mechanism; update them through
  client releases. Independent rule packages and hot updates are deferred. Unknown,
  conflicting or revoked requirements cannot silently produce a journal-compliant claim.
- Process manuscripts locally. The new core distribution must remove remote manuscript-model
  command paths as well as their UI. Public-data maintenance must never consume author projects.
- Organize only the files required for the selected article type and submission stage. Keep
  publisher files separate from author tools and internal records, including ZIP contents.
- Preserve existing user records and historical transmissions; do not reset their provenance or
  claim that past external transmission never happened.

## Relationship to earlier decisions

ADR 0001, 0002, 0004, 0005, 0008 and the integrity contracts in ADR 0009 remain applicable.
This decision supersedes the author-facing stage sequence in ADR 0006 and the inclusion of
author-configured remote models from ADR 0007 in the new product scope. Their existing records
and historical implementation descriptions remain valid; this ADR does not delete their data.

The knowledge-body roadmap and PWC/AI/publication service designs are retained as deferred
background, not prerequisites or the root goal of the newly scoped product. Legacy UI task
tracks yield to the new interaction design; visual tokens and accessibility rules still apply.

## Consequences

The desktop experience is smaller, while rule maintenance and reliable document output become
the principal investment. A signature proves package integrity, not rule correctness. A file
export is not a real submission. Three-minute messaging remains an internal, bounded timing
target for recommendation and preparation lists, not an end-to-end marketing promise.

No application code, version metadata, user data or installed application changes with this ADR.
Future delivery must validate both tasks in zh-CN and en, on native Windows and macOS, including
offline behavior, failures, generated files and privacy observations.

## References

- [Product overview](../product-design-overview.md)
- [Implementation plan and acceptance](../local-journal-package-plan.md)
- [Interaction design](../ui-design-direction.md)
- [Rule system](../submission-rule-system.md)

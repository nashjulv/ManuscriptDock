# ManuscriptDock Development Log

## 2026-09-01 — V0.13 local source identity and version visibility

- Added schema v4 `SourceIdentityVersion` to preserve source-declared title, authors, affiliations,
  email, ORCID and correspondence details alongside the exact `ArtifactVersion` and source fragment.
- Removed identity metadata from the semantic Include/Exclude decision path: source-declared identity
  remains visible locally before and after knowledge-body finalization, while Claim-like candidates
  still require item-level author review.
- Added a dedicated identity-and-version card and made the spatial identity node summarize actual
  author and contact counts instead of showing only a stable ID placeholder.
- Kept the Rust network boundary unchanged: the complete local identity object is omitted from the
  default model projection, and its author names are added to question redaction.

## 2026-08-31 — Prerelease version baseline V0.12

- Established `V0.12` as the current visible product version and `0.12.0` as its SemVer package and installer equivalent.
- Added the version after the product name in the desktop top bar, landing identity and native window title while preserving the stable application name and bundle identifier for in-place upgrades.
- Recorded the default prerelease cadence: each tested, user-visible product update increments by `0.01`, retaining two display digits and carrying in base ten.
- Added a repository-level version policy and future-agent instruction so subsequent updates keep UI, npm, Cargo, Tauri and documentation versions synchronized.

This log records completed feature slices, their trust-boundary implications, and the checks
run before each commit. It contains no real manuscripts or identifiable review material.

## 2026-08-30 — DeepSeek empty-answer recovery

- Traced the reported empty primary answer to DeepSeek V4's default high-effort thinking mode combined
  with the former 1200-token output cap. The official model ID and API endpoint were already correct.
- Disabled thinking mode only for the official `api.deepseek.com` Chat Completions endpoint and raised
  the final-answer allowance to 2400 tokens. Other OpenAI-compatible providers receive no DeepSeek-only
  request fields.
- Made response parsing accept visible text returned as a string or text-part array, tolerate nullable
  content, and distinguish output-limit and reasoning-only failures. Private `reasoning_content` is
  never displayed or persisted as an answer.

### Verification

- Synthetic Rust coverage verifies endpoint scoping, request serialization, string and part-array
  answers, nullable content, output-limit diagnostics, and non-disclosure of reasoning text.
- `npm run check`: passed TypeScript, frontend tests, production build, Rust tests, rustfmt, and Clippy
  with warnings denied.

## 2026-08-30 — PDF brand asset and Chinese glyph correction

- Replaced the manual cover's temporary vector approximation with the same crayon-textured PNG icon
  shipped by the Tauri desktop application.
- Replaced the fenced-code rendering of the eight-stage Chinese workflow with a native numbered flow
  table. The old Courier run had no Chinese glyphs and displayed the stage names as black squares.
- Added a reproducible PDF build script that reads the maintained Markdown manual and the current
  desktop icon, while preserving the A4 layout, linked contents, outline bookmarks, and local-first
  visual system.

### Verification

- Compiled and executed `scripts/build_user_manual_pdf.py` with the bundled Python 3.12 runtime.
- Rendered all 12 pages at 1.7x resolution and visually checked the cover, workflow, contents, tables,
  page boundaries, and final page.
- `pypdf` confirmed 12 non-empty pages, 26 outline entries, every workflow stage, and no black-square
  or Unicode replacement characters in extracted text.

## 2026-08-30 — Standalone user manual and cross-model privacy closure

- Added a standalone author-facing manual covering installation, the complete eight-stage workflow,
  journal matching, model setup, workspace management, privacy boundaries, troubleshooting, and the
  current MVP limitations.
- Closed a privacy gap found while verifying the manual: the knowledge-body model projection no longer
  contains the extracted author list. Known author names, email addresses, phone-like numbers, long
  identifiers, and local paths are redacted from projected text and the outbound question at the Rust
  network boundary; the original question remains only in the local immutable ledger.
- Retained the separately authorized institution-policy rule: an institution name may be sent for that
  extraction task, while the author name, source URL, contacts, identifiers, and manuscript body remain
  excluded.
- Exported the manual as a 12-page A4 PDF with a branded cover, linked two-page table of contents,
  PDF outline bookmarks, page headers, page numbers, and print-safe tables. The final render contains no
  blank pages or orphaned list items.

### Verification

- `npm run check`: passed TypeScript, 15 frontend tests, the Vite production build, 59 core tests,
  9 desktop Rust tests, rustfmt, and Clippy with warnings denied.
- `cargo xwin check --workspace --target x86_64-pc-windows-msvc`: passed the Windows x64 desktop
  dependency graph with the shared privacy redaction path.
- `npm run tauri -- build --debug --no-bundle`: produced the updated macOS debug executable.
- Rendered every PDF page to PNG and visually checked the cover, contents, tables, Chinese typography,
  page boundaries, and final page; `pypdf` also confirmed 12 non-empty pages, metadata, and 26 outline
  entries.

## 2026-08-30 — Institution requirement model extraction and private directory boundary

- Added an optional author-supplied institution-policy field to journal matching. After per-call consent,
  the model may receive the institution name to scope the rule. Rust removes the author name, source URL,
  email, phone-like numbers, identifiers, and manuscript content at the network boundary; the remaining
  request contains the institution, discipline, manuscript purpose, and redacted policy.
- Added a constrained JSON extraction prompt with prompt-injection resistance, strict CCF tier
  vocabulary, partition bounds, confidence, conditions, and ambiguity warnings. Model memory and
  unsupported inference cannot create an official rule.
- Saved extracted evidence as a new immutable profile containing the source-text SHA-256, source
  type, extraction model, normalized conditions, verification status, and audit event; the original
  policy text is not persisted in the recommendation record.
- Kept internal prompts and scoring implementation out of the customer UI while retaining visible
  rule application status. Raw institution evaluation directories are not exposed in the interface.
- Added an internal partition-data readiness boundary. A partition condition remains excluded until
  a legally sourced, versioned data adapter is available; the model cannot guess the partition.
- Official pages reviewed on 2026-08-30 still described the partition service as continuing, exposed
  a licensed API for subscribing institutions, and retained copyright and non-redistribution terms.
  No official stop-update-and-open-redistribution notice was found, so no protected table was bundled.
- Clarified that matching computes the best current-fit submission set rather than lowering opportunity
  by identity. Institution rank and adviser fame receive no direct score. Current local readiness is a
  structural signal, not an innovation judgment; a versioned PWC review is required before substantive
  scholarly strength can influence journal-level recommendations.
- Defined the version-driven quality loop: every immutable manuscript revision must be re-evaluated on
  the same quality dimensions. A higher version number earns no points by itself; only traceable
  improvements can raise a dimension and unlock better matched or reach journals, while regressions may
  lower it. The local MVP currently re-evaluates structural readiness; PWC review will later supply
  versioned innovation, evidence-strength, and method-reliability signals.

### Verification

- `npm run check`: passed TypeScript, 15 frontend tests, the Vite production build, 59 core tests,
  8 desktop Rust tests, rustfmt, and Clippy with warnings denied.
- `cargo xwin check --workspace --target x86_64-pc-windows-msvc`: passed the Windows x64 desktop
  dependency graph.
- `npm run tauri -- build --debug --no-bundle`: produced the updated macOS debug executable.
- Local browser review at a 375px viewport confirmed visible labels, 44px consent targets, and no
  horizontal overflow (`clientWidth = scrollWidth = 360`).

## 2026-08-30 — Versioned submission context and institution-rule boundary

- Added a required local submission profile before journal matching: author name, institution,
  faculty/specialty, manuscript purpose, and submission-completion deadline. Each distinct profile is
  an immutable JSON record and is referenced by the recommendation run.
- Made every profile and preference change invalidate the visible result and produce a newly bound
  run. Author name changes attribution only; no author-identity or institutional-prestige scoring is
  permitted.
- Added specialty, purpose, and deadline feasibility to deterministic scoring. Deadline feasibility
  measures ManuscriptDock's internal submission-preparation allowance and does not claim to predict
  review, acceptance, publication, or indexing dates.
- Added the PWC institution-rule evidence contract and reserved the highest single weight, 24%, for
  verified official rules. Missing or candidate-only sources are excluded and force a clearly marked
  provisional shortlist; verified eligibility rules can block a candidate without hiding it.
- Defined the future discovery boundary: search only after author authorization, prioritize official
  institution/graduate-school/research-office/faculty pages, never treat search snippets as evidence,
  and require source, scope, validity, original excerpt, verification, and rule-set version before
  scoring.
- Reworked the bilingual form and result cards with visible labels, 44px controls, narrow-screen
  reflow, institution-rule status, remaining days, and specialty/purpose/timing components.

### Verification

- `npm run check`: passed TypeScript checks, 15 frontend interaction tests, Vite production build,
  rustfmt, 57 manuscript-core tests, 4 desktop Rust tests, and Clippy with warnings denied.
- `cargo xwin check --workspace --target x86_64-pc-windows-msvc`: passed the complete Windows x64
  desktop dependency graph with the profile persistence and scoring contracts.
- `npm run tauri -- build --debug --no-bundle`: produced the macOS debug desktop executable.
- Browser review verified the two-column desktop form hierarchy and a 375px viewport with one-column
  preference controls, 44px inputs, no horizontal overflow (`clientWidth = scrollWidth = 360`), and
  a visible provisional institution-rule state.

## 2026-08-30 — Five-part single-paper KnowledgeBody architecture

- Upgraded newly created `AcademicKnowledgeBodySnapshot` records to schema v2 while retaining read
  and hash compatibility for existing schema-v1 records.
- Reorganized the single body into identity/version, knowledge/boundary/evidence, capability
  contracts, interaction/runtime, and validation/rights/reputation without deleting the existing
  Claim five-tuple or ten versioned manuscript objects.
- Added three explicit capability contracts for local source traceability, runtime-dependent
  evidence-bounded questions, and planned method-applicability checks. Every contract declares input,
  output, preconditions, refusal conditions, evidence sources, availability, and its own version.
- Defined author-configured models as replaceable runtime coordinators with per-call authorization;
  the model projection now includes the five-part architecture and cannot invent unavailable
  capabilities.
- Separated immutable content snapshots from independently evolving ReputationRecord references and
  added explicit RightsPolicy and validation-record references. AIReviewReport remains empty until a
  real PWC professional review exists.
- Replaced the eight-object single-body plot with a double-boundary spatial view: stable body identity,
  immutable content snapshot, central rotating Claim dodecahedron, and five labeled/versioned service
  regions. Dashed runtime and dotted reputation links communicate replaceability and independent state
  without relying on color alone.
- Extended the author question target list to capability contracts and rights/reputation while keeping
  the two-body and network assertion protocols unchanged.

### Verification

- `npm run check`: passed TypeScript checks, 15 frontend interaction tests, Vite production build,
  rustfmt, 52 manuscript-core tests, 4 desktop Rust tests, and Clippy with warnings denied.
- `cargo xwin check --workspace --target x86_64-pc-windows-msvc`: passed the complete Windows x64
  desktop dependency graph with the schema-v2 knowledge body.
- `npm run tauri -- build --debug --no-bundle`: produced the macOS debug desktop executable.
- Browser verification at 900 × 700 and 375 × 760 confirmed that all five spatial regions remain
  legible and the narrow layout has no horizontal overflow or clipped left-side node.

## 2026-08-30 — Classification-first PDF extraction and recognition routing

- Evaluated Pandoc and kept it out of PDF ingestion because it has no PDF input reader; retained it
  only as a future converter between author-confirmed structured formats.
- Added the MIT-licensed `pdf-inspector 1.17.0` default Rust pipeline before the existing
  `pdf-extract` and lopdf fallbacks.
- Classified each PDF before interpreting extracted content and retained classification confidence,
  table pages, column pages, encoding problems, and page-level recognition candidates across fallback
  paths in the versioned structure-v5 report.
- Used layout-aware Markdown headings and table delimiters for deterministic section normalization and
  table counts while preserving PDF output as `limited` evidence.
- Defined object-level routing: native text, formula regions, and table structure are preferred; only
  missing or unreliable text goes to Chinese/English OCR, formulas to a formula recognizer, and tables
  to a table-structure recognizer. Future fusion must retain page, bounds, producer, confidence, and
  author-confirmation state and may not overwrite reliable native content.
- Fixed the first text-OCR profile as mixed Simplified/Traditional Chinese plus English, while keeping
  OCR visibly unavailable until the PDFium, ONNX Runtime, model, checksum, and cross-platform packaging
  requirements are actually delivered.

### Verification

- `npm run check`: passed TypeScript checks, 15 frontend interaction tests, Vite production build,
  rustfmt, 51 manuscript-core tests, 4 desktop Rust tests, and Clippy with warnings denied.
- `cargo xwin check --workspace --target x86_64-pc-windows-msvc`: passed the complete Windows x64
  dependency graph, including the new default Rust `pdf-inspector` path.
- `npm run tauri -- build --debug --no-bundle`: produced the macOS debug desktop executable with
  the structure-v5 parser.
- The synthetic PDF regression verifies layout-aware extraction, `TextBased` classification, title,
  author and abstract recovery; the Markdown regression verifies heading normalization and table count.

## 2026-08-30 — Local computer/AI journal matching

- Adapted the PWC dynamic-fit proposal into a bounded local MVP between version management and
  attestation: a recommendation run binds manuscript hash/version, preferences, algorithm version,
  catalog version, score details, provenance, limitations, and transfer state.
- Added a verified computer/AI candidate snapshot based on the independent CCF 2025 domestic
  T1/T2/T3 directory and CCF international AI A/B/C directory, with official journal homepages.
- Implemented deterministic bilingual topic detection, article-type detection, manuscript-readiness
  scoring, preference adjustment, stable run IDs, and separate top-three domestic/international lists.
- Persisted each distinct run under the workspace analysis directory and appended an auditable
  `journal_recommendations_computed` event without network access, model calls, or manuscript transfer.
- Added an eighth workflow stage with author-adjustable topic, article type, language, target strategy,
  and open-access controls; recalculation produces a new run instead of overwriting prior evidence.
- Kept school/institution rules explicitly unconfigured and excluded from scoring. The UI and product
  specification state that fit scores are not acceptance probabilities and that journal websites must
  be verified before submission.

### Verification

- `rustup run stable cargo test --workspace`: passed 52 tests, including deterministic recommendation,
  bounded scores, six-result output, and topic-adjustment behavior.
- `npm run test --workspace @manuscriptdock/desktop`: passed 15 interaction tests, including domestic/
  international result rendering and recalculation after an author adjustment.
- `npm run typecheck --workspace @manuscriptdock/desktop`: passed.
- `npm run build --workspace @manuscriptdock/desktop`: passed the production build.

## 2026-08-30 — Unified folded-manuscript brand icon

- Replaced the temporary boxed `M` with the approved folded-manuscript paper-plane mark in the
  persistent product bar and browser entry point.
- Established one editable SVG source with the `#A6CE39` wing, `#5B5956` crayon strokes, hidden
  `M`, white canvas, and two manuscript lines.
- Scaled the artwork to approximately 82% of the desktop icon canvas and centered its actual visual
  bounds so Dock, Finder, and installer surfaces retain consistent clear space.
- Moved the two manuscript lines upward by 24 source pixels to improve the internal vertical balance
  without changing their spacing, weight, or length.
- Regenerated the complete Tauri icon family from that source for macOS, Windows, Linux, Android,
  and iOS packaging.
- Kept the visual mark decorative beside the existing `投稿舱 ManuscriptDock` accessible name so
  assistive technology does not announce the brand twice.

### Verification

- `xmllint --noout` passed for the design-system, WebView, and Tauri SVG sources.
- `npm run test --workspace @manuscriptdock/desktop`: passed.
- `npm run typecheck --workspace @manuscriptdock/desktop`: passed.
- `npm run build --workspace @manuscriptdock/desktop`: passed.
- `npm run tauri --workspace @manuscriptdock/desktop -- build --bundles app`: produced a fresh
  `ManuscriptDock.app`; its bundled `Resources/icon.icns` checksum matches the generated source
  `icon.icns` exactly.

## 2026-08-29 — Bilingual homepage brand statement

- Added a prominent, always-visible Chinese/English brand statement above the homepage import area.
- Separated the Chinese product name, positioning, and slogan from their English counterparts with
  independent lines, typographic hierarchy, and language metadata.
- Standardized the top bar identity as `投稿舱 ManuscriptDock` in both interface locales.
- Reframed the “传输可见” principle around author choice: the author decides whether to connect,
  use a model, or send work externally; action-specific disclosure remains part of each workflow.
- Preserved the PWC-derived neutral palette, PingFang-light typography, and `#A6CE39` trust accent.

### Verification

- `npm run test --workspace @manuscriptdock/desktop`: passed frontend interaction tests, including
  bilingual brand visibility, language metadata, and the revised transfer-control copy.
- `npm run typecheck --workspace @manuscriptdock/desktop`: passed.
- `npm run build --workspace @manuscriptdock/desktop`: passed.

## 2026-08-24 — Native desktop installer pipeline

- Enabled Tauri bundling with stable product metadata and platform-specific configuration.
- Defined a universal macOS 11+ App/DMG target for Apple Silicon and Intel Macs.
- Defined a bilingual Windows x64 NSIS setup target that installs per-user and obtains WebView2 only
  when the host does not already provide it.
- Marked Windows release builds as GUI-subsystem applications so launching ManuscriptDock does not
  open a companion console window.
- Added reproducible local scripts and a manually triggerable macOS/Windows native-runner workflow.
- Kept signing credentials outside the repository and documented the Developer ID, notarization,
  Authenticode, and SmartScreen boundary for public distribution.

### Verification

- `npm run check`: passed 11 frontend interaction tests, 41 Rust core tests, production frontend
  build, formatting, and warning-free Clippy.
- `ManuscriptDock_0.1.0_universal.dmg`: valid DMG checksum; embedded executable contains both
  `arm64` and `x86_64`; app identifier `com.manuscriptdock.desktop`; minimum macOS 11.0.
- `ManuscriptDock_0.1.0_x64-setup.exe`: Tauri NSIS verification passed; bundled app is an AMD64
  PE with the Windows GUI subsystem rather than the console subsystem.
- SHA-256: macOS `204ebc94e1cc243c67adc4fdffd35f3e3b6290106fc80fc642db9d446f3623d0`;
  Windows `ecf371496a098d7d1180502a1c8d2b52e415e94b91d609e9f6cfb647f6ccb41f`.

## 2026-08-24 — M0 executable foundation and safe manuscript selection

### Delivered

- Initialized the npm and Cargo workspaces for Tauri 2.x, React 18, TypeScript 5, Vite 5,
  and Rust 1.93.
- Added a minimal Tauri window with a restrictive content security policy and only the default
  core window capability.
- Added a Rust-owned native file picker for DOCX, PDF, and TEX files.
- Added deterministic format, file-type, and 250 MB size validation in `manuscript-core`.
- Added an explicit `selected`, `cancelled`, or `rejected` command outcome. Expected user errors
  are data rather than IPC exceptions.
- Limited the WebView response to file name, format, kind, size, and modified time. The source
  path is not part of the serialized contract.
- Added the first OpenAI-inspired academic workspace screen with local-only messaging,
  accessible focus states, dark mode, reduced-motion support, and responsive layout.
- Added a local vector application icon and generated platform icon assets through the Tauri CLI.
- Added Node/Rust toolchain pins, dependency lock files, and CI.

### Verification

- `npm run typecheck`: passed.
- `npm run test`: 4 frontend state tests passed.
- `npm run build`: Vite production build passed.
- `cargo fmt --all --check`: passed with Rust 1.93.
- `cargo test --workspace`: 5 Rust tests passed, including synthetic-file metadata, size-limit,
  extension, and serialized IPC-contract coverage.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `npm run tauri -- build --debug --no-bundle`: passed and produced the local debug executable.
- Browser visual QA at the default desktop viewport and 375 × 812: no horizontal overflow,
  46 px primary action height, balanced mobile heading, and no console warnings or errors.
- `npm run check`: final integrated repository check passed after all M0 code and documentation
  changes.

### Environment note

The workstation currently exposes Node 25.6, which is not the release target and therefore
emits an engine warning. CI and release development target Node 24 LTS as pinned by the repo.
Local Rust verification used the installed stable toolchain, which is exactly Rust 1.93.0.

## 2026-08-24 — M1 immutable import and local workspace recovery

### Delivered

- Added a Rust-only pending-selection vault. The WebView receives a one-time opaque identifier,
  never the selected source path.
- Added atomic local workspace creation under the operating system application-data directory.
- Copied and hashed source content in one pass, detected size changes during import, and marked
  the completed source snapshot read-only.
- Added schema-versioned manifests, UUID workspace identifiers, SHA-256 content fingerprints,
  and a `workspace_created` JSONL audit event without absolute paths.
- Added restart recovery through a catalog command that sorts recent workspaces and safely skips
  corrupt records.
- Added UI states for creating a workspace, confirming the immutable snapshot, and showing recent
  local projects.
- Recorded the storage boundary in ADR 0002.

### Verification

- `npm run test`: 6 frontend tests passed, including workspace creation and restart recovery.
- `cargo test --workspace`: 8 Rust tests passed, including snapshot copying, content hashing,
  path non-disclosure, catalog recovery, and corrupt-record isolation.
- `npm run check`: type checking, frontend build, rustfmt, all tests, and Clippy passed.
- `npm run tauri -- build --debug --no-bundle`: passed and rebuilt the desktop executable.

## 2026-08-24 — M2 deterministic manuscript structure extraction

### Delivered

- Added Rust-owned TEX, DOCX, and PDF structure extraction over the immutable source snapshot.
- Extracted title, abstract and keyword presence, section hierarchy, figures, tables, references,
  common submission declarations, word count, and PDF page count where available.
- Marked PDF results as `limited` and surfaced font/layout and possible OCR limitations without
  silently repairing or rewriting content.
- Added versioned JSON analysis artifacts bound to the source snapshot version and SHA-256
  fingerprint, plus append-only `structure_analyzed` audit events.
- Rejected manifest path traversal before resolving any source snapshot.
- Revalidated snapshot size and SHA-256 content before each analysis, rejecting unexpected local
  modification even if file permissions were bypassed.
- Added a desktop command and UI for running extraction, reviewing structure and warnings, and
  reopening a recovered workspace. The WebView still receives no filesystem path.
- Recorded parser and trust-boundary choices in ADR 0003.

### Verification

- `npm run test`: 7 frontend tests passed, including recovered-workspace analysis rendering.
- `cargo test --workspace`: 14 Rust tests passed, including TEX, DOCX XML, PDF inference,
  versioned persistence, audit append, snapshot tamper detection, and path-traversal coverage.
- `npm run check`: type checking, frontend build, rustfmt, all tests, and Clippy passed.
- `npm run tauri -- build --debug --no-bundle`: passed and rebuilt the local desktop executable.
- Browser semantic and layout QA at 1280 × 720 passed with no horizontal overflow and a 46 px
  primary action target.

## 2026-08-24 — M3 signed readiness rules and local preview snapshots

### Delivered

- Added two data-defined rule-pack layers for generic manuscript structure and the initial
  submission stage, with inheritance rather than journal-specific code branches.
- Added Ed25519 verification against a Rust-owned public trust anchor before rule parsing or
  execution. The temporary signing private key was destroyed and was never added to the repo.
- Rejected tampered signatures, missing or cyclic inheritance, duplicate identities, duplicate
  rules, and unsupported field/operator combinations as whole-evaluation failures.
- Added explainable passed, warning, blocked, and author-confirmation findings with semantic
  source locations and signed rule provenance.
- Added immutable per-run output snapshots containing a versioned JSON report and escaped,
  self-contained HTML preview, plus a `readiness_evaluated` audit event.
- Added a desktop readiness screen with outcome counts, rule detail, signature status, coverage-C
  disclosure, and an explicit “no external transmission” boundary.
- Recorded the durable design in ADR 0004 and synchronized the rule-system product document.

### Verification

- `npm run test`: 7 frontend workflow tests passed, including the full recovered-workspace path
  through structure extraction and readiness rendering.
- `cargo test --workspace`: 21 Rust tests passed, including signature verification and tamper
  rejection, inheritance-cycle and duplicate-rule rejection, rule classification, HTML escaping,
  and output snapshot persistence.
- `npm run check`: type checking, frontend build, rustfmt, all tests, and Clippy passed.
- `npm run tauri -- build --debug --no-bundle`: passed and rebuilt the local desktop executable.

## 2026-08-24 — MVP repository handoff audit

### Delivered

- Updated the root, core-crate, document index, and repository-structure guides to describe the
  completed M0–M3 implementation instead of the earlier M0-only state.
- Added the 0.1.0 development MVP status, trust boundary, verification baseline, and explicit
  post-MVP limitations.
- Confirmed that designated historical research and concept files have no changes after the
  initial repository commit.
- Confirmed that no build directory, dependency directory, TypeScript build info, environment
  file, PEM private key, or generic key file is tracked.

### Verification

- Final `npm run check`: passed with 7 frontend tests and 21 Rust tests.
- Final `git diff --check`: passed.
- The latest desktop build remains `target/debug/manuscriptdock-desktop`; build output is ignored
  and is not part of the Git history.

## 2026-08-24 — Academic knowledge body goal alignment

### Product decision

- Reaffirmed that automatic formatting and submission readiness are trust-building entry points,
  while the author-controlled academic knowledge body is the fundamental product objective.
- Mapped the immutable source, deterministic structure, signed assessments, audit trail, and
  output projections from M0–M3 onto the future knowledge-body model.
- Defined K1 local modeling, K2 author-controlled AI access, and K3 publication/PWC networking as
  the continuous roadmap.
- Added a product guardrail: future features must state how they consume or enrich the same
  knowledge body and must not create an untraceable parallel data silo.

## 2026-08-24 — Paperpal competitive response and market evidence brief

### Product and market decision

- Recorded Paperpal's publicly promoted 30+ check baseline and the distinction between standard
  marketing totals and journal-configured check counts.
- Defined an approximately 42-rule deterministic parity target across metadata, structure,
  references, figures/tables, disclosures, and submission-package integrity.
- Kept language advice, similarity signals, and professional scientific review in separate trust
  layers rather than inflating deterministic rule counts.
- Defined the ManuscriptDock differentiation around local-first trust, actionable versioned
  outputs, transparent signed rules, China/global coverage, and academic knowledge-body
  continuity.
- Added approved positioning language, prohibited claims, official source links, and evidence
  metrics required before external promotion.

## 2026-08-24 — Knowledge body as the platform service object

### Product decision

- Distinguished human users and paying customers from the durable object managed by the platform:
  authors retain ownership and decisions, while services continuously operate on the academic
  knowledge body.
- Defined mandatory write-back behavior for formatting, rules, AI suggestions, PWC review,
  provenance evidence, submission, revision, and publication.
- Defined purpose-limited external projections instead of whole-workspace transfer.
- Connected the service model to lifecycle state, ManuscriptDock/PWC responsibilities, business
  models, knowledge-body quality metrics, market language, and feature admission questions.

## 2026-08-24 — macOS and Windows compatibility re-audit

### Verification

- Re-ran the full repository gate: 7 frontend tests and 21 Rust tests passed together with
  TypeScript, Vite, rustfmt, and Clippy checks.
- Rebuilt and launched the native Apple Silicon executable without an immediate crash.
- Cross-compiled the complete Tauri desktop application into an Intel macOS x86_64 executable.
- Compiled `manuscript-core` and all of its test targets for Windows MSVC. The complete Tauri
  workspace reached Windows resource generation and then stopped because the macOS host does not
  provide `llvm-rc`; this is recorded as incomplete rather than a Windows pass.
- Rechecked 1180 × 780, 760 × 620, and 1366 × 768 layouts with no horizontal overflow, a 46 px
  primary action, complete semantic labels, and no browser console errors.
- Recorded the remaining native Windows, installer/signing, keyboard-navigation, and light-theme
  muted-text contrast gaps in the platform compatibility audit.

## 2026-08-24 — M4 reliable PDF embedded-text recognition

### Delivered

- Diagnosed a 107-page local QA case where the PDF visibly contained selectable text but lopdf's
  basic extractor returned no words and incorrectly suggested OCR.
- Added a pure-Rust enhanced font/ToUnicode extraction path with the existing basic parser as a
  fallback. OCR is now recommended only when both deterministic paths return no readable text.
- Added PDF metadata-title decoding, including UTF-16 strings, while preferring a longer visible
  title when metadata contains only a prefix.
- Made embedded PDF bookmarks the primary section hierarchy and tightened fallback heading,
  title, abstract, and keyword heuristics to avoid equations, contents leaders, and body mentions.
- Incremented deterministic structure analysis to v2, preserving prior v1 artifacts rather than
  overwriting them.

### Verification

- Added tests for PDF metadata/UTF-16 titles, visible-title preference, and conservative numbered
  heading inference; all 23 Rust tests passed.
- Re-ran the affected 107-page local QA workspace without changing its source snapshot: title
  improved from empty to `Memory in the Age of AI Agents: A Survey`, word count from 0 to 64,137,
  and section hierarchy from 0 to 72 embedded bookmarks. The previous OCR warning disappeared.

## 2026-08-24 — PWC-informed UI visual baseline

### Product and design decision

- Reviewed the PWC personal workspace publish center and original-submission flow as a read-only
  interaction and visual reference.
- Adopted its flat workbench, visible step progress, grouped forms, thin dividers, and
  operation-plus-preview rhythm without copying PWC branding, global site navigation, identity
  prompts, blockchain language, or platform-wide controls.
- Replaced the earlier permanent three-column concept with a 52 px task rail and responsive
  operation/evidence split. Manuscript, version, and attachment navigation moves into a side
  drawer instead of consuming a permanent column.
- Defined the ManuscriptDock neutral paper palette, trust-green primary action, typography,
  spacing, component, feedback, accessibility, and macOS/Windows adaptation rules.
- Kept the academic knowledge body visible as the lifecycle destination while preventing the
  future network from overwhelming the MVP submission workflow.

### Verification

- `git diff --check`: passed.
- `npm run check`: passed with 7 frontend tests, 23 Rust tests, TypeScript/Vite build,
  rustfmt, and Clippy.
- The visual specification includes explicit form labels, feedback for operations over 300 ms,
  keyboard focus, 44 px hit targets, color-independent statuses, and 760/960/1180/1440 px desktop
  validation widths.

## 2026-08-24 — M5 PWC-informed desktop workbench

### Delivered

- Replaced the centered single-card workspace with a flat 56 px product bar, persistent stage
  rail, operation pane, and read-only evidence pane while retaining the quiet import start page.
- Connected immutable import, deterministic structure extraction, and signed readiness checks to
  real stage transitions instead of presenting them as one growing results card.
- Added honest target-journal and automatic-formatting placeholders, a version-aware local package
  view, and a knowledge-body view that reflects which real artifacts currently exist.
- Added responsive operation/evidence tabs below 960 px, persistent stage state, explicit local and
  external-transmission boundaries, consistent SVG icons, reduced-motion support, visible focus,
  skip navigation, and 44 px interaction targets.
- Preserved prior trust guarantees: the WebView still receives no manuscript path, analysis remains
  deterministic, and no model, PWC, submission, or publishing transmission was introduced.

### Verification

- Expanded the frontend suite from 7 to 8 tests, including stage navigation, evidence-pane
  semantics, planned-capability disclosure, knowledge-body visibility, and responsive pane state.
- `npm run check`: passed with TypeScript, Vite, 8 frontend tests, 23 Rust tests, rustfmt, and
  Clippy `-D warnings`.
- `npm run build`: TypeScript and the Vite production build passed.
- `npm run tauri -- build --debug --no-bundle`: rebuilt the native macOS debug executable.
- Launched the rebuilt executable without an immediate process failure.

## 2026-08-24 — M6 strict PWC publish-page visual alignment

### Delivered

- Replaced the remaining centered landing treatment with the measured PWC publish-page shell:
  56 px product bar, 48 px task rail, 574 px operation column, and fluid guidance/evidence column.
- Matched the reference palette (`#FAFAFA`, `#F5F5F5`, `#F0F0F0`, `#1C1B19`, `#4A4845`,
  `#7A7872`, `#EBEBEB`, and `#E0E0E0`), typography, thin borders, radii, card spacing, and flat
  surfaces. Removed the competing decorative and dark-theme treatments from the MVP baseline.
- Standardized all green primary actions and current-step accents on `#A6CE39`. Kept PWC's dark
  circular guidance steps so green remains a deliberate action signal rather than decoration.
- Preserved ManuscriptDock's own product identity, local-only boundary, immutable-source language,
  auditable transmission model, and visible academic knowledge-body destination.

### Verification

- Compared PWC and ManuscriptDock in the same 1280 × 720 in-app browser viewport. The local page
  matched the measured 56 / 48 / 574 shell, 64 px guidance cards, 172 px dual-track cards, 28 px
  primary buttons, PWC font stack, and all specified color values without horizontal overflow.
- Browser semantic inspection confirmed the skip link, banner, labelled navigation, one H1,
  import region, local-work principles, and five-step guidance landmark.
- `npm run check`: passed with TypeScript, Vite production build, 8 frontend tests, 23 Rust tests,
  rustfmt, and Clippy `-D warnings`.
- `npm run tauri -- build --debug --no-bundle`: rebuilt the native macOS debug executable.

## 2026-08-24 — M6.1 structure-action contrast

- Changed only the “开始结构提取” action, including its processing state and both render paths,
  to use white text and iconography on the existing `#A6CE39` background.
- Added a frontend assertion for the dedicated contrast treatment; other primary-button text colors
  remain unchanged.

## 2026-08-24 — M6.2 PWC icon and readability polish

- Reworked the task-rail glyphs to the same Lucide-style visual language used by PWC: 24 px
  coordinate system, 20 px rendered size, 2 px rounded strokes, and semantically appropriate
  upload, document, globe, formatting, review, package, and knowledge symbols.
- Increased the interface type scale by approximately 1 px across navigation, actions, guidance,
  operation content, evidence, metadata, and output views while preserving the 48 / 574 shell.
- Increased primary and secondary desktop actions from 28 px / 4 px corners to 30 px / 6 px
  corners. The structure-extraction action retains its dedicated white text and icon treatment.
- Rechecked the 1280 × 720 landing view with no horizontal overflow.
- `npm run check` passed with TypeScript, Vite production build, 8 frontend tests, 23 Rust tests,
  rustfmt, and Clippy `-D warnings`.
- Rebuilt and launched the native macOS debug application without an immediate process failure.

## 2026-08-24 — M6.3 icon uniqueness and unified typography

- Audited the task-rail SVG paths and found that the source-file and structure stages shared the
  same document glyph. Replaced the structure glyph with a distinct Lucide-style list-structure
  symbol and added a frontend uniqueness assertion covering every landing-rail icon.
- Unified every `#A6CE39` primary button's text and icon color on `#4A4846`, removing the temporary
  white structure-action exception from both component code and tests.
- Applied PingFang Light at weight 300 to every application surface, including headings, labels,
  buttons, technical metadata, and preview chrome. Added PingFang HK, Microsoft YaHei UI Light,
  and Segoe UI Light as platform fallbacks; immutable source-page glyphs remain untouched.
- Browser verification at 1280 × 720 confirmed six distinct rail icons, the exact green-button
  foreground color, full PingFang Light computed styles, and no horizontal overflow.
- `npm run check` passed with TypeScript, Vite production build, 8 frontend tests, 23 Rust tests,
  rustfmt, and Clippy `-D warnings`.
- `npm run tauri -- build --debug --no-bundle` rebuilt the native macOS debug executable.

## 2026-08-24 — M6.4 Chinese and English interface

### Delivered

- Added a compact “中文 / EN” language switch to the product bar, with Chinese as the first-run
  default and the user's choice persisted locally across restarts.
- Localized the complete product interface, including navigation, actions, workflow stages,
  operation and evidence panes, status feedback, empty states, submission guidance, knowledge-body
  views, and accessibility labels.
- Added English mappings for deterministic Rust-engine messages that reach the WebView: built-in
  rule labels and findings, PDF/DOCX extraction warnings, workspace catalog warnings, selection
  errors, and other recoverable local-processing errors.
- Preserved manuscript-derived content in its source language. Filenames, paper titles, section
  headings, citations, and author text are never silently translated by the interface switch.
- Kept one language visible at a time so the established PWC-informed 56 / 48 / 574 workbench stays
  quiet and readable instead of duplicating every label inline.

### Verification

- Added frontend coverage for language switching, the document language attribute, local
  persistence across remounts, dynamic error translation, and English rendering of signed-rule
  findings and sources.
- In-app browser inspection at 1280 × 720 confirmed the full English landing path, selected language
  state, semantic labels, and no horizontal overflow.
- `npm run check` passed with TypeScript, Vite production build, 9 frontend tests, 23 Rust tests,
  rustfmt, and Clippy `-D warnings`.
- `npm run tauri -- build --debug --no-bundle` rebuilt the native macOS debug executable.

## 2026-08-24 — M6.5 initial spatial knowledge-body view

> Superseded by M6.6 after correcting the product's Claim semantics.

### Delivered

- Replaced the flat knowledge-object cross with an initial spatial model centered on a slowly
  rotating wireframe cube and a clearly labelled Claim concept anchor.
- Added three current version tendrils for the immutable source snapshot, deterministic structure
  version, and local submission version. Each tendril branches into its own attributes and
  verifiable data, while unavailable objects remain explicitly pending rather than fabricated.
- Kept the visualization truthful: the current MVP does not extract formal Claim nodes, so the
  center is labelled as a concept anchor and never substitutes the manuscript title for a claim.
- Implemented the view in lightweight HTML, SVG, and CSS without WebGL or a new graphics runtime.
  Only the cube moves; version labels and data stay fixed and readable.
- Added a complete bilingual accessible summary and stopped the animation under
  `prefers-reduced-motion`.

### Verification

- Extended the knowledge-stage frontend test to verify the Claim anchor, cube, three version
  clusters, and honest pending states.
- Isolated browser rendering at 1280 × 720 confirmed the three clusters do not overlap, the Claim
  remains legible over the cube, the 32-second rotation is active, and no horizontal overflow is
  introduced.
- `npm run check` passed with TypeScript, Vite production build, 9 frontend tests, 23 Rust tests,
  rustfmt, and Clippy `-D warnings`.
- `npm run tauri -- build --debug --no-bundle` rebuilt the native macOS debug executable.

## 2026-08-24 — M6.6 corrected Claim five-tuple spatial model

### Product correction

- Corrected the fundamental knowledge-body unit to `Claim = <proposition, conditions, evidence,
  sources, status>`. Versions belong independently to these five elements; they are not tendrils
  or top-level knowledge-body branches.
- Removed the cube, version tendrils, and attribute-branch model introduced in M6.5 from the active
  product specification and interface.

### Delivered

- Replaced the center cube with a mathematically projected wireframe dodecahedron built from 20
  vertices and 30 edges. It rotates slowly in three-dimensional projection around the Claim.
- Connected the center directly to five stationary element spheres: Proposition, Conditions,
  Evidence, Sources, and Status. Each sphere displays its own version number and current state.
- Used explicit `v0` empty versions for Claim elements that have not yet been formally established;
  retained the real immutable source version and the current knowledge-body construction status.
- Updated the operation pane to show the exact Claim five-tuple instead of source, structure,
  review, and output artifacts as if they were Claim elements.
- Preserved bilingual labels, a complete screen-reader summary, and a fixed dodecahedron projection
  when reduced motion is enabled.

### Verification

- Updated frontend coverage to assert the exact five-tuple, 30 dodecahedron edges, five element
  spheres, and the independent `v0 / v1` labels.
- Isolated browser rendering at 1280 × 720 confirmed five straight connections, five non-overlapping
  spheres, a legible rotating Claim center, and no horizontal overflow.
- `npm run check` passed with TypeScript, Vite production build, 9 frontend tests, 23 Rust tests,
  rustfmt, and Clippy `-D warnings`.
- `npm run tauri -- build --debug --no-bundle` rebuilt the native macOS debug executable.

## 2026-08-24 — M7 built-in publication-standards catalog

### Delivered

- Added 16 selectable rule packs covering three current Chinese standards, COPE / CRediT
  transparency, Elsevier, Springer Nature, Wiley, IEEE, and eight mainstream reporting-guideline
  profiles.
- Added official-source URLs, bilingual catalog metadata, verification dates, B/C coverage labels,
  and explicit applicability descriptions to every selectable pack.
- Added a desktop target-step selector that composes applicable standards while keeping general
  initial-submission rules active. Changing a selection invalidates the old readiness result.
- Extended deterministic extraction for funding, ethics approval, registration, author
  contributions, AI use, and consent-for-publication declarations, plus common section aliases.
- Changed user-facing “signature” language to “source and integrity”; Ed25519 remains an expandable
  technical detail instead of the primary product concept.
- Upgraded readiness reports to v2 with bilingual findings, actual pack provenance, official
  sources, verification dates, and local integrity status.
- Generated the new signatures with an ephemeral catalog key, deleted the private key after use,
  and retained only the public trust anchor and signatures in the repository.

### Product boundary

- The catalog is standards- and publisher-level coverage, not a claim of complete support for every
  journal. Journal instructions remain authoritative and future A-level journal packs contain only
  verified differences.
- Official reporting checklists remain at their official sources. ManuscriptDock checks detectable
  structure and creates author-confirmation items; it does not reproduce a checklist or score study
  quality.

### Verification

- Frontend tests cover catalog loading, selection, bilingual rendering, selected-ID IPC, and the
  revised source-and-integrity language.
- Rust tests cover all bundled signatures, catalog filtering, selected dependency composition,
  unknown-pack rejection, tamper rejection, and extended declaration recognition.
- `npm run check` passed with TypeScript, Vite production build, 9 frontend tests, 27 Rust tests,
  rustfmt, and Clippy `-D warnings`.
- `npm run tauri -- build --debug --no-bundle` rebuilt the native macOS debug executable.
- Browser inspection at 1280 × 720 and 820 × 720 confirmed that the shared application shell has
  no horizontal overflow and switches to the narrow layout at the intended breakpoint.

## 2026-08-24 — M8 local manuscript version library

### Delivered

- Upgraded the workspace manifest to schema v2 while retaining read compatibility with existing
  schema v1 workspaces.
- Added immutable manuscript revisions with sequential version numbers, SHA-256 deduplication,
  parent-version provenance, optional author notes, and same-source-format enforcement.
- Added safe restoration: selecting an old version creates a new current version and never moves,
  deletes, or overwrites history.
- Added deterministic version comparison for title, word count, figures, tables, sections,
  declarations, and full content fingerprints. Comparison performs no AI call or external transfer.
- Added a bilingual author-facing version stage between Source and Structure. Its single primary
  action, immutable timeline, progressively disclosed restore action, and read-only evidence pane
  follow the PWC-derived ManuscriptDock visual system.
- Added a page-specific version-library design specification, product design document, and ADR.
  The product language intentionally avoids Git internals while retaining repository-grade
  provenance underneath.

### Product boundary

- Source-format conversion remains a derived submission output, not a manuscript revision.
- The current diff is deterministic structure-level evidence, not a paragraph or word redline.
- Named branches, merging, working-copy watchers, milestone tags, repository export, and PWC sync
  remain explicit follow-on capabilities.

### Verification

- Rust coverage includes version creation, duplicate rejection, structural comparison, safe
  restoration, source-format mismatch, immutable snapshot integrity, and legacy manifest reading.
- Frontend coverage exercises revision selection, version notes, saving, timeline refresh,
  automatic comparison, and the existing bilingual submission workflow.
- `npm run check` passed with TypeScript, Vite production build, 10 frontend tests, 32 Rust tests,
  rustfmt, and Clippy `-D warnings`.

## 2026-08-24 — Product decision: submission revision desk

### Documented

- Defined a focused Submission Revision Desk between deterministic checks and the local version
  library. It covers submission-specific metadata, declarations, document blocks, references, and
  generated materials without positioning ManuscriptDock as a Word or Overleaf replacement.
- Defined format-specific safety boundaries: structured round-trip for supported DOCX content,
  reviewable patches for LaTeX, read-only evidence for PDF, and author-confirmed OCR candidates for
  scanned PDF.
- Defined revision sets as reviewable changes with before/after content, reason, basis, source, and
  acceptance state. Accepted changes create immutable manuscript versions and never overwrite the
  source or history.
- Connected revision provenance to the academic knowledge body while keeping manuscript versions
  distinct from independently versioned Claim five-tuple elements.

### Status boundary

- This entry records an approved product direction, not delivered application functionality.
- The current MVP still requires external editing followed by import into the implemented local
  manuscript version library.

## 2026-08-24 — M9 signed publisher submission elements

### Delivered

- Extended the signed Elsevier, Springer Nature, Wiley, and IEEE coverage-B packs with structured
  submission elements for author identity, manuscript content, declarations, ethics, and files.
- Added a trusted Rust aggregation API that composes selected packs, deduplicates shared elements,
  retains every publisher source, and rejects invalid element metadata.
- Added the first operational Submission Revision Desk screen. It groups elements progressively,
  distinguishes future structured-edit fields from author-confirmation items, and shows verified
  publisher provenance in the evidence pane.
- Renamed the former placeholder Format stage to Revision while retaining the stable internal stage
  identifier for compatibility.
- Re-signed only the four changed packs with an ephemeral offline key. The private key and temporary
  signing directory were deleted; only signatures and the public trust anchor remain.

### Product boundary

- Publisher elements are a B-level preparation baseline, not journal-level compliance claims.
- This slice exposes and traces submission elements; applying field changes to DOCX/TEX and saving
  revision sets as new manuscript versions remains the next slice.

### Verification

- Rust tests cover signature validation, element aggregation, deduplication, editable-field mapping,
  and multi-publisher provenance.
- Frontend tests cover publisher selection, element loading, grouped bilingual rendering, source
  evidence, and continuation into deterministic checks.

## 2026-08-24 — M10 structured submission revision and version provenance

### Delivered

- Added a trusted revision draft for the current immutable manuscript version. TEX exposes title,
  abstract, and keywords; DOCX exposes a safely located Title-style title; PDF remains read-only.
- Added deterministic TEX and DOCX writers. TEX replaces only balanced commands or the abstract
  environment; DOCX rewrites only the located title text while preserving every other ZIP entry.
- Added before/after review in the evidence pane and one primary action to save accepted author
  edits as a new immutable manuscript version.
- Persisted a `revision.json` beside the generated version with revision ID, base/output versions,
  before/after values, author-edit basis, acceptance state, time, and no-transmission record.
- Added a `revision_applied` audit event and rejected stale base versions, unavailable fields, empty
  values, oversized values, PDF writes, and unchanged revisions.

### Product boundary

- DOCX abstract, keyword, declaration, and paragraph round-trip remain read-only until their
  structural locations can be preserved safely.
- This slice records author edits only. Deterministic auto-fixes and AI candidates will use separate
  basis and acceptance states in later slices.
- Saving invalidates previous structure and readiness results; automatic rerun remains follow-on
  work so the product never presents a stale report as current.

### Verification

- Rust tests cover TEX field replacement, DOCX title replacement with XML escaping and unrelated
  entry preservation, immutable version creation, revision-set persistence, audit, and re-analysis.
- Frontend tests cover field loading, live before/after evidence, save payload, new-version feedback,
  and the existing publisher/check workflow.

## 2026-08-24 — Persistent workspace navigation

### Delivered

- Promoted “My Workspace / 我的工作台” to a permanent application-level icon at the top of the
  left task rail on both the landing screen and every manuscript stage.
- Added a restrained divider between the application-level workspace entry and manuscript-level
  workflow stages, preserving the existing 48px PWC-informed rail.
- Returning to My Workspace now keeps the current manuscript available in the recent workspace
  list without opening a file picker or performing external transmission.

### Verification

- Frontend coverage checks the selected home state, visibility inside an active manuscript, direct
  return behavior, and continued visibility of the local workspace after returning home.

## 2026-08-24 — Versioned knowledge-body network model

### Delivered

- Added the first schema-versioned `AcademicKnowledgeBodySnapshot` in the Rust trusted core with
  exact ArtifactVersion, Claim, Scope, Method, Result, EvidenceRelation, SourceAnchor,
  AIReviewReport, Provenance, and KnowledgeBodySnapshot references.
- Promoted `AIReviewReport` to an independent version history. A snapshot can pin an exact report
  such as v2 while retaining v1; report upgrades neither overwrite history nor advance Claim.
- Added eight first-class cross-body protocols: CitationAssertion, ClaimRelationAssertion,
  EvidenceRelation, MethodRelationAssertion, ReproductionAssertion, AlignmentAssertion,
  VersionRelation, and ClassificationAssignment.
- Rejected assertions with missing basis, mismatched protocol objects, zero versions, or identical
  source and target object versions.
- Rebuilt the knowledge evidence view around the supplied visual references: one body, two bodies,
  and network levels; circular knowledge-body boundaries; compact Claim/Anchor/Method nodes; and
  green assertion diamonds. Higher levels stay disabled until real objects and assertions exist.
- Replaced the single-body five-tuple projection with an immutable KnowledgeBodySnapshot boundary,
  a rotating Claim dodecahedron, and eight independently versioned surrounding objects. The Claim
  five-tuple remains an internal semantic constraint instead of standing in for the entire body.

### Trust boundary

- Current workspaces expose one truthful local body and no fabricated AI review or external
  relationship. The five-body K-A through K-E network remains a synthetic UI and protocol fixture.
- Deterministic submission checks remain separate from professional `AIReviewReport` objects.

### Verification

- Rust tests pin AIReviewReport v2 while retaining v1, verify all protocol mappings, and confirm
  that a new local snapshot invents neither reviews nor cross-body assertions.
- Frontend tests exercise all three visual levels with synthetic K-A through K-E bodies and six
  assertion types, while preserving the existing 30-edge rotating Claim dodecahedron.

## 2026-08-24 — Knowledge-object label foreground layer

- Raised every surrounding-object name, version, and state label above the sphere highlight,
  shadow, border, and relation-diamond surfaces.
- Kept the existing spatial positions and version demonstration style unchanged.

## 2026-08-24 — Structure recognition v4: authors and abstract candidates

### Delivered

- Added author candidates and abstract candidate text to the versioned structure-report contract.
- TEX now reads author commands and abstract environments. DOCX reads author/abstract styles with
  conservative front-matter fallback. PDF combines Author metadata with first-page name lines.
- Expanded explicit abstract markers to English and Chinese punctuation variants, including
  `ABSTRACT—` and spaced Chinese headings.
- Added a bounded fallback for PDFs whose abstract has no visible heading: consecutive prose after
  authors and affiliations becomes a candidate and always carries an author-confirmation warning.
- The operation pane now shows detected authors as a first-class readiness item; the evidence pane
  displays the author list and a bounded abstract preview.

### Verification

- Synthetic tests cover multi-author TEX, DOCX Author styles, PDF metadata authors, symbol-prefixed
  author lines, bilingual markers, and unlabelled first-page abstracts without confusing a subtitle
  or affiliation for an author.
- A user-scoped local validation reran the existing 107-page PDF without transmission. Structure v4
  retained the title, identified 47 author names, and recovered a 2,268-character unlabelled abstract
  candidate with an explicit confirmation warning. No manuscript content entered the repository.

## 2026-08-24 — End-to-end seven-step manuscript lifecycle

### Delivered

- Replaced the module-oriented eight-screen rail with the author task path: Import, Check, Revise,
  Version, Attest, Submit, and Knowledge Body.
- Consolidated structure extraction, rule selection, and itemized readiness findings into one
  progressive Check stage.
- Added lifecycle recovery for the current manuscript fingerprint. Reopening a workspace now
  restores matching structure, readiness, attestation, submission, and finalized knowledge-body
  records without rerunning work or borrowing artifacts from another version.
- Made structured revision save, re-extract, and rerun the same rules before continuing to Version.
  Importing or restoring another version invalidates all current downstream state while preserving
  historical files and records.
- Added author-confirmed local attestation records bound to manuscript version/hash and the current
  readiness report. Each record has its own SHA-256, is read-only, is verified again on recovery,
  and explicitly does not claim blockchain notarization or scientific truth.
- Added explicit-folder export of a submission handoff containing the manuscript, JSON findings,
  HTML preview, attestation, and manifest. Export never overwrites an existing package.
- Added author-confirmed manual submission records for journal/platform target and optional receipt;
  ManuscriptDock records but does not impersonate the external submission system.
- Added immutable knowledge-body finalization after submission, binding the snapshot to the exact
  attestation and submission record without network publication.

### Verification

- Rust lifecycle coverage completes check, attestation, export, submission, and knowledge-body
  finalization; recovers all current records; rejects missing confirmations; detects changed record
  content; and proves a new manuscript head has no inherited downstream state.
- Frontend coverage now follows the consolidated check flow, verifies revision plus automatic
  recheck, creates attestation, exports the handoff, records submission, restores a finalized
  knowledge body, and exercises the three spatial knowledge views.
- TypeScript, Vite, rustfmt, Rust tests, Clippy, and the Tauri desktop build remain release gates.

## 2026-08-24 — Author-confirmed discipline index and knowledge-body hash

### Delivered

- Added a bilingual 12-item `ManuscriptDock Discipline Index v1.0` catalog. Knowledge-body
  finalization now requires the author to choose a primary discipline; no model or title-based
  inference runs in this release.
- Persisted the choice as a versioned, author-confirmed `ClassificationAssignment`. Reclassification
  keeps its stable assignment ID, advances the version, and creates a new immutable knowledge-body
  record instead of rewriting history.
- Extended the knowledge-body SHA-256 payload to cover the classification, snapshot, attestation,
  and submission references. The final operation pane now displays the full 64-character hash,
  discipline code and labels, index scheme, protocol version, author-confirmed status, and record ID.
- Kept legacy records without classification readable. They open in an explicit completion state
  where the author can add a discipline and create a new hashed record.
- Reserved future model support for evidence-backed candidate classifications only; author
  confirmation remains required before a candidate can become current.

### Verification

- Rust coverage rejects unknown codes, verifies catalog stability, restores classified records,
  detects tampering, and confirms reclassification advances the assignment and changes the hash.
- Frontend coverage verifies the finalize action remains disabled until the author selects a
  discipline, sends the confirmed code through the Rust boundary, and renders the full hash and
  `ClassificationAssignment` on completion.

## 2026-08-25 — Knowledge-body dialogue and author-controlled model routing

### Delivered

- Added the dialogue desk below the spatial knowledge-body view, with author questions classified
  as recognition, question, or challenge and targeted to an exact knowledge object.
- Added exactly three model slots: one primary and two ordered fallbacks. Rust owns HTTPS endpoint
  validation, bounded requests, response parsing, and failover; localhost services may use HTTP.
- Stored API keys in macOS Keychain or Windows Credential Manager and kept plaintext credentials out
  of settings files, IPC responses, workspaces, audit events, errors, and dialogue records.
- Sent only an author-confirmed minimum KnowledgeBody projection, never the source file or path.
- Persisted immutable, hash-verified inquiry and answer records bound to the current knowledge-body
  record, hash, and snapshot. Model failure leaves a truthful unanswered local inquiry.
- Reserved a separate external-reader surface for recognition, questions, and challenges without
  exposing an unauthenticated network endpoint or inventing feedback.

### Verification

- Frontend coverage configures all three visible slots, saves a synthetic primary Key through the
  narrow command contract, asks a Claim challenge, renders the recorded answer, and verifies the
  external surface remains reserved.
- Rust coverage verifies dialogue hashes and record recovery, rejects invalid or tampered records,
  isolates old dialogue after reclassification, validates the exact three-slot contract, and rejects
  insecure remote HTTP endpoints.
- `npm run check` passed 13 frontend tests, 45 Rust tests, TypeScript, Vite, rustfmt, and warning-free
  Clippy. The native macOS debug application rebuilt successfully.
- `cargo xwin check --workspace --target x86_64-pc-windows-msvc` passed with the repository's Windows
  SDK toolchain, including Credential Manager and the HTTPS model client.

### Protocol correction

- Fixed the desktop command contract so the two fallback roles use the explicit stable wire names
  `fallback_1` and `fallback_2`. Added bidirectional serialization assertions to prevent the Rust
  enum names and TypeScript command payload from drifting again.

## 2026-08-25 — Per-manuscript archive and permanent deletion

### Delivered

- Added a separate Manage action to every manuscript in Recent Workspaces, preserving the full-row
  Open action without nested interactive controls.
- Added a reversible Archived view. Archive and restore move the complete local workspace between
  validated collections and append `workspace_archived` or `workspace_restored` audit events.
- Added an explicit inline confirmation before permanent deletion, with truthful scope covering
  manuscript versions, analysis, attestation, submission, knowledge-body, and dialogue records.
- Restricted Rust management commands to exact UUID directories that are not symbolic links and
  whose manifest identity matches the requested workspace. Existing destinations are never
  overwritten.
- Kept external source manuscripts and separately exported submission packages outside the delete
  boundary. The UI retains success or failure feedback even after the final row is removed.

### Verification

- Frontend coverage archives a synthetic manuscript, opens the Archived view, restores it, cancels
  a deletion, and then confirms permanent deletion. All 14 workflow tests pass.
- Rust coverage verifies archive and restore audit history, rejects deletion without author
  confirmation, deletes only the exact active or archived synthetic workspace, and proves an
  injected symbolic link cannot change or remove its external target. All 47 Rust core and desktop
  tests pass.

## 2026-08-25 — Actionable model enablement and DeepSeek preset

### Delivered

- Diagnosed the local unavailable state without reading or exposing a credential: the saved
  DeepSeek slot used the documentation website as its base URL and no Key was available to the app.
- Separated an author's Enabled preference from the actual Ready state. Every slot now reports a
  concrete missing field, wrong endpoint, missing Key, unsaved readiness, or enabled state.
- Made the no-model composer action open Model Settings directly and explain the exact recovery
  path instead of presenting a disabled “Configure first” button.
- Added a one-click DeepSeek preset using the current official OpenAI-compatible base URL and model,
  while keeping API Key entry author-controlled. Both UI and Rust reject the DeepSeek documentation
  host as an API endpoint.
- Required every enabled slot to have either a newly entered or already stored Key before settings
  can be saved. Clearing a Key and enabling the same slot is rejected.

### Verification

- Frontend coverage opens settings from the unavailable composer, applies the DeepSeek preset,
  remains blocked until a Key is entered, saves the complete wire payload, and completes a
  synthetic knowledge-body question.
- Rust coverage rejects documentation URLs and verifies the new-or-existing credential rule without
  touching a real credential store. All 14 frontend workflow tests and 48 Rust tests pass.

## 2026-08-25 — Actionable model-provider failure messages

- Confirmed against the provider's official error-code reference that the observed DeepSeek HTTP
  402 response means insufficient account balance rather than a ManuscriptDock connection failure.
- Replaced bare 400/401/402/403/404/422/429/5xx statuses with concise recovery guidance while
  retaining the numeric status for support. Failover behavior is unchanged.
- Added bilingual display for the most common billing, authorization, and rate-limit summaries.
- Rust regression coverage now verifies billing, authentication, rate-limit, and unknown-status
  messages. All 49 Rust tests pass.
## 2026-08-31 — One decomposition for knowledge-body candidates and submission outputs

### Delivered

- Replaced the current-version structure file with a versioned, SHA-256-verified
  `decomposition manifest` that retains deterministic semantic candidates and text/table/figure
  extraction coverage without storing local source paths.
- Added the explicit `candidate` state between `pending` and `established`. Extracted Claim,
  Scope, Method, Result, and Evidence passages now carry source labels, modality, confidence, and
  stable candidate IDs; absent content alone remains `pending v0`.
- Made readiness evaluation reuse the persisted decomposition instead of parsing the manuscript a
  second time. Submission exports now include that exact decomposition manifest and bind its ID and
  hash in `submission-manifest.json`.
- Made the knowledge-body preview available immediately after decomposition. Immutable lifecycle
  finalization remains separate and still requires attestation, submission registration, and an
  author-selected discipline.
- Added the same semantic extraction projection to author-authorized model questions, with an
  explicit rule that candidates may be summarized only with their unconfirmed status preserved.

### Verification

- `npm run check` passed 15 frontend workflow tests, 60 core tests, 11 desktop Rust tests,
  TypeScript, the production Vite build, rustfmt, and warning-free Clippy.
- Core tests verify deterministic text/table/figure candidate extraction, immutable decomposition
  storage, source-fragment IDs, shared decomposition hashes in knowledge snapshots and submission
  exports, and legacy workspace migration.
- `cargo xwin check --workspace --target x86_64-pc-windows-msvc` passed for the Windows x64 target.
- `npm run tauri --workspace @manuscriptdock/desktop -- build --debug --no-bundle` rebuilt the native
  macOS desktop executable, which was launched for local verification.
- The candidate review surface follows the existing PWC-derived ManuscriptDock design system:
  progressive disclosure, explicit text states rather than color-only meaning, readable metadata,
  and a one-column compact layout below 720 px.

## 2026-08-31 — Author-confirmed semantic knowledge and readable spatial legend

### Delivered

- Added a real item-level author review flow for every Claim, Scope, Method, Result, and Evidence
  candidate. Each item must be explicitly included or excluded before finalization, followed by a
  separate author attestation.
- Made Rust reject missing, duplicate, stale, or incomplete candidate decisions. Included candidates
  become `established`; excluded candidates remain non-established, and all decisions are covered by
  the immutable knowledge-body hash.
- Replaced the version-only spatial legend with manuscript-specific content: the Claim summary sits
  above the dodecahedron, while a traceable five-item legend shows semantic summaries, confirmation
  state, source fragment, and confidence.
- Confirmed that the stale desktop logo came from the older `/Applications/ManuscriptDock.app`
  installation rather than the repository assets; the installed app must be replaced by the newly
  bundled application to update the macOS Dock icon.

### Verification

- Frontend coverage exercises include/exclude controls, the separate author confirmation, finalization
  payload, readable spatial summaries, and confirmed states.
- Core lifecycle coverage rejects incomplete reviews and verifies that included candidates persist as
  `established` with `authorConfirmed: true`.

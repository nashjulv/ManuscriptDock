# Manuscript Core

This crate contains the reusable Rust core for the V0.54 local journal-matching and submission-package workflow. It remains independent from the Tauri window layer.

Current responsibilities:

- immutable local DOCX snapshots, versioned project manifests, revision checks, and atomic records;
- bounded DOCX inspection without executing macros or fetching external relationships;
- a signed built-in journal catalog and composable pilot rule packs;
- deterministic hard-constraint filtering, a verification pool, and at most three recommendation roles;
- author-confirmed facts and a context-bound preparation projection;
- byte-preserving manuscript copies, author-review DOCX/XLSX files, publisher-only ZIP creation, file hashes, transactional export, and local receipts.

The WebView receives presentation DTOs and opaque selection tokens. It does not receive the internal staging directory or gain arbitrary filesystem/network access. Tauri owns native dialogs and delegates storage, parsing, rules, preparation, and package work to this crate.

Remote manuscript-model calls, knowledge-body/PWC flows, live journal-site fetching, universal DOCX transformation, anonymous-manuscript rewriting, and publication transmission are not runtime responsibilities of this crate.

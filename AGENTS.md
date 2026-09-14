# ManuscriptDock Repository Instructions

## Product

- Public product name: `ManuscriptDock`
- Chinese name: `投稿舱`
- Current prerelease display version: `V0.54`
- Product direction: local journal matching and target-specific submission-package preparation.
- Desktop stack: Tauri 2.x, React 18, TypeScript 5, Vite 5, Rust/Cargo

## Versioning

- Show the prerelease version immediately after the product name on primary identity surfaces: `投稿舱 ManuscriptDock V0.54`.
- After each user-visible product update, increment the display version by `0.01` using two digits after the decimal (`V0.12` → `V0.13`; `V0.19` → `V0.20`).
- Keep package and installer metadata SemVer-compatible (`V0.54` maps to `0.54.0`).
- Preserve the stable package/product identifier `ManuscriptDock` so an update replaces the existing installation rather than creating a second application.

## Architecture Boundaries

- The WebView is an untrusted presentation layer and must not receive unrestricted filesystem or network access.
- Rust owns local storage, document processing, permissions, audit, and outbound network policy.
- Source manuscripts are immutable inputs. Generated targets and submissions are versioned snapshots.
- The new core product has two tasks: recommend journals and prepare a package for a chosen journal. Users with a target can skip recommendation. Follow `docs/implementation/README.md` for detailed execution, `docs/local-journal-package-plan.md` for scope, and `docs/ui-design-direction.md` for simple local-file interaction language.
- The user authorizes rewriting prior code, APIs and interactions and deleting unused implementation. Keep only useful capabilities; do not preserve legacy flows, tests or modules merely for compatibility. Follow `docs/implementation/rewrite-and-removal.md`. Preserving user manuscripts and exports does not require keeping old knowledge-body or remote-model runtime code.
- Public journal requirements may be AI-assisted drafts, then manually verified and maintained directly in repository data files shipped with the client. Do not add a maintenance UI, admin backend, PWC dependency, or independent rule-update service for the first release.
- Remote manuscript-model calls and knowledge-body/PWC flows are outside the new core scope. Preserve existing user data and historical transmission records; the documentation change alone does not remove current runtime capabilities.
- Journal support uses composable, signed rule packs rather than journal-specific application branches.

## Historical Materials

The following paths contain early research or concept material. Treat them as read-only unless the user explicitly requests changes:

- `demo/`
- `output/`
- `tmp/`
- `学术知识体调研资料.md`
- `docs/browser-embedded-desktop-architecture.md`
- `docs/product-design-submission-lifecycle.md`

New product decisions belong in the current ManuscriptDock documents linked from `docs/README.md`. Detailed engineering plans live in `docs/implementation/`; document ownership and the planned product/reference/archive/release layout are defined in `docs/documentation-structure.md`.

## Repository Practices

- Preserve unrelated user changes.
- Do not commit secrets, real unpublished manuscripts, or identifiable review materials.
- Use synthetic fixtures for tests.
- Keep deterministic checks separate from AI suggestions.
- Make external transmission and destructive actions explicit and auditable.
- Use `apply_patch` for hand-authored file changes.
- Add executable build commands only when the corresponding toolchain files exist.

## Internationalization Review

- The supported interface locales are Simplified Chinese (`zh-CN`) and English (`en`). Review every new or modified code path for internationalization impact before declaring the task complete, even when the change initially appears locale-neutral.
- Treat all user-visible content as in scope: React text, labels, placeholders, validation, notifications, errors, accessible names, document titles and metadata, Rust/Tauri messages, generated HTML or exported reports, rule-pack copy, and dynamic journal, provenance, or model-service data.
- Reuse the existing frontend helpers in `apps/desktop/src/i18n.tsx` (`useI18n().text`, `localize`, and `localizeBackendText`) instead of adding one-language UI strings. Preserve paired Chinese/English fields such as `label`/`labelEn`, `description`/`descriptionEn`, and `message`/`messageEn` when a contract or rule pack already uses that pattern.
- Do not rely on `localizeBackendText` silently returning an untranslated value. For new user-visible Rust errors or dynamic messages, prefer a stable machine-readable error code plus parameters; when retaining a string contract, add or update the English mapping in the same change and cover interpolated variants.
- Keep user-authored manuscript content and quoted source material in its original language. Localize the surrounding interface, provenance category, status, and explanatory copy rather than automatically translating research content.
- Use locale-aware APIs for dates, times, numbers, units, and plural-sensitive copy. Do not build new locale-sensitive sentences solely by concatenating fragments.
- The primary product identity intentionally remains bilingual as `投稿舱 ManuscriptDock V0.54`; do not remove either name as an internationalization cleanup.
- For any user-visible change, add or update focused tests for both `zh-CN` and `en`, including dynamic and failure states where relevant. A static landing-page language toggle test alone is not sufficient evidence for backend messages or exported artifacts.
- In the completion report, include an `Internationalization review` note stating which locales and surfaces were checked, what localization changed, and any known gap. If the code change has no user-visible or locale-sensitive effect, explicitly state that the review found no adaptation needed.
- These are review instructions for AI coding tools, not an enforcement mechanism. Do not add Git hooks, mandatory CI gates, or build-time blockers solely for this section unless the user explicitly requests them.

# ManuscriptDock Repository Instructions

## Product

- Public product name: `ManuscriptDock`
- Chinese name: `投稿舱`
- Category: local-first manuscript submission workspace
- Desktop stack: Tauri 2.x, React 18, TypeScript 5, Vite 5, Rust/Cargo

## Architecture Boundaries

- The WebView is an untrusted presentation layer and must not receive unrestricted filesystem or network access.
- Rust owns local storage, document processing, permissions, audit, and outbound network policy.
- Source manuscripts are immutable inputs. Generated targets and submissions are versioned snapshots.
- User-configured models handle general assistance. Protected professional review agents remain on the PWC service.
- Journal support uses composable, signed rule packs rather than journal-specific application branches.

## Historical Materials

The following paths contain early research or concept material. Treat them as read-only unless the user explicitly requests changes:

- `demo/`
- `output/`
- `tmp/`
- `学术知识体调研资料.md`
- `docs/browser-embedded-desktop-architecture.md`
- `docs/product-design-submission-lifecycle.md`

New product decisions belong in the current ManuscriptDock documents linked from `docs/README.md`.

## Repository Practices

- Preserve unrelated user changes.
- Do not commit secrets, real unpublished manuscripts, or identifiable review materials.
- Use synthetic fixtures for tests.
- Keep deterministic checks separate from AI suggestions.
- Make external transmission and destructive actions explicit and auditable.
- Use `apply_patch` for hand-authored file changes.
- Add executable build commands only when the corresponding toolchain files exist.

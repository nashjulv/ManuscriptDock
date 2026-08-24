# ManuscriptDock Repository Structure

## Active product paths

```text
apps/
└── desktop/                 Tauri + React desktop application

crates/
└── manuscript-core/        Reusable trusted Rust core and signed bundled rule packs

packages/
├── contracts/              Frontend-facing command and event contracts
└── ui/                     Shared React components and design tokens

schemas/                    Manuscript, rule-pack and exchange schemas
fixtures/                   Synthetic test data only
scripts/                    Repository-wide developer automation
tests/                      Cross-package and end-to-end tests
design-system/              ManuscriptDock visual and interaction rules
docs/                       Product, architecture and decision records
```

The workspace is a monorepo so the desktop shell, trusted core, schemas, UI, and tests can evolve together while preserving explicit boundaries.

The current executable graph is intentionally narrow: `apps/desktop` depends on
`crates/manuscript-core`; placeholder package, schema, fixture, script, and cross-package test
directories document future boundaries but are not yet build dependencies. Runtime workspaces,
analysis results, and readiness previews live in the operating system application-data directory
and must never be committed to this repository.

## Toolchain baseline

The executable workspace targets Node.js 24 LTS with npm 11 and Rust 1.93. The root
`package.json` and `Cargo.toml` coordinate the React/Vite frontend, Tauri shell, and reusable
Rust core. CI runs the same type, unit, build, formatting, and lint checks exposed by
`npm run check`.

The provisional desktop application identifier is `com.manuscriptdock.desktop`. Supported
operating-system versions, release signing identities, and update channels remain release
decisions and must be recorded before distributable packages are enabled.

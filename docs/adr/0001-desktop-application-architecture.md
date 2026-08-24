# ADR 0001: Desktop Application Architecture

- Status: Accepted
- Date: 2026-08-23

## Context

ManuscriptDock processes private manuscripts and long-running local tasks while using a minimal Web UI across desktop operating systems. The application requires a narrow permission boundary between presentation and privileged local capabilities.

## Decision

Use:

- Tauri 2.x as the desktop shell;
- Rust for the trusted local core;
- React 18 and TypeScript 5 for the WebView UI;
- Vite 5 for frontend builds;
- Cargo for Rust builds.

Keep reusable document, rule, storage, audit, and network-policy logic in a Rust crate independent from Tauri window code. Expose only narrow validated commands to the WebView.

## Consequences

- One web-based UI can serve macOS, Windows, and Linux;
- operating-system WebView differences require platform testing;
- frontend development remains fast while privileged operations stay in Rust;
- OCR and other heavy local services need explicit process, resource, and update management;
- product behavior cannot depend on unrestricted frontend filesystem or network access.

## Related Decisions

- [Product design overview](../product-design-overview.md)
- [Submission rule system](../submission-rule-system.md)

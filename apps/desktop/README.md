# Desktop Application

This directory contains the installable ManuscriptDock desktop application:

- React 18 + TypeScript 5 WebView UI;
- Vite 5 frontend build;
- Tauri 2.x application shell under `src-tauri/`;
- platform packaging for macOS, Windows, and Linux.

Do not add unrestricted filesystem or network access to frontend code. Native capabilities must be exposed through narrow, validated Tauri commands.

Run the full desktop app from the repository root with `make start-dev` (or the lower-level
`npm run dev`). The Make target performs dependency and port preflight checks first. Use
`npm run frontend:dev` only when working on browser-safe presentation states.

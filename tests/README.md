# Cross-Cutting Tests

This directory is reserved for tests that cross package or application boundaries, including:

- import-to-submission-package workflows;
- rule-pack compatibility and regression suites;
- Tauri command permission boundaries;
- offline and blocked-network behavior;
- snapshot reproducibility;
- cross-platform packaging smoke tests.

Unit tests should remain next to their owning Rust crate or TypeScript package.

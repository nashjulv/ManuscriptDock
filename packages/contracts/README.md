# Application Contracts

This package will contain versioned frontend-facing contracts for narrow Tauri commands, events, task progress, errors, and outbound confirmation data.

Contracts must make these states explicit:

- local-only;
- queued or processing;
- requires user confirmation;
- will transmit externally;
- completed, cancelled, or failed;
- associated manuscript and snapshot version.

The Rust implementation remains authoritative for permissions and validation.

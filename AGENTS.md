<!-- bmad:context -->
<!-- Verified 2026-10-23 against a2cb62e. Managed by bmad-project-context. -->

## Meetily

Meetily is a Tauri desktop application with a Rust core and Next.js UI.
For BMAD usage and maintenance, read docs/BMAD.md.
Planning and implementation artifacts belong in _bmad-output/.

## Policy

- Use a separate task branch; do not commit, push, or open a PR without an explicit request.
- Keep real meeting recordings, transcripts, credentials, and personal configuration out of Git; use synthetic examples.
- Preserve CLAUDE.md when maintaining this Codex integration.

## Where things are

- For supported setup and build changes, read docs/BUILDING.md.
- Treat backend/ as an unsupported historical archive; implement current behavior through frontend/src-tauri and frontend/src.

## Running and verifying

- The frontend dev server alone cannot validate native recording; verify native behavior in the Tauri desktop app.
- There is no universal pnpm test script; inspect the relevant test file and its runner before choosing a test command.
- For BMAD runtime requirements and pinned reinstall instructions, read docs/BMAD.md.

## Conventions that differ from defaults

- Route frontend/native interactions through Tauri commands and events; do not introduce the archived FastAPI service as a runtime dependency.
- Use Tauri path APIs for cross-platform application data paths.

<!-- /bmad:context -->

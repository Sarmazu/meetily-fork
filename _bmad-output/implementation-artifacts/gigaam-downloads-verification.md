# Story 1.2 — GigaAM downloads verification

Date: 2026-10-30. Baseline commit: `a2cb62e827da7ef59f65064c97233efb2313878e`.

Implemented a package-only GigaAM manager and settings section. The embedded Story 1.1 manifest supplies all four model IDs, file URLs, sizes, and SHA-256 values; no Python/source files are downloaded. Native commands return `supported=false` outside Windows x64 before resolving/creating application directories. This story adds no provider/model selection or inference.

## Environment and isolation

Editing host: WSL2 Linux `6.6.87.2-microsoft-standard-WSL2`. Linux PATH has no Cargo, Rust, Bun or pnpm. Verification uses the existing isolated Windows x64 Rust 1.88.0/MSVC toolchain under `D:\meetily-whisper-build-20261029`, and a separate task directory `D:\meetily-gigaam-downloads-20261030`. Existing source/build/evidence directories were preserved. Native application/installed Meetily and personal application configuration were not launched or modified.

The new task contains a synthetic Rust harness, a separate full application source copy/fresh native target, and a frontend copy with a junction to the existing dependency directory. Node 20.20.2 and pnpm 9.15.9 are task-local existing tools. Bun 1.3.14 was downloaded into the new task only; rustfmt was added to the isolated Rust 1.88.0 toolchain. No global tool settings were changed.

## Executed checks

- `cargo test --offline` in the isolated minimal Rust harness: **11 passed**, using current production catalog/download module files and their unchanged relative manifest include path. Tests use tiny loopback HTTP packages, never real model weights. Coverage: exact four-package manifest, successful verified publication, offline reinitialization, already-valid final with unreachable URL, same-size wrong hash, retry, interrupted response, cancellation, duplicate rejection, cleanup and corrupt-final quarantine, cancellation during staging/final hashing and immediately before publication, Windows junction refusal preserving target bytes, unexpected staging-data preservation after cancellation, publication-failure rollback, and throttled actual progress events. Event callbacks assert model IDs/byte counts and one terminal event after publication or cleanup. This harness excludes Tauri command registration and is not a full application build.
- `bun test tests/components/gigaam-model-manager.test.tsx`: **5 passed**, 24 assertions. Mocked Tauri verifies subscribe-before-list, list/progress/download/cancel/retry/error, unsupported platform hiding, listener cleanup, stale initial-list protection after terminal refresh, and visible/retryable listener/list initialization failures. Every invoked command is a GigaAM command; the component has no settings writer or provider selection. This is a component test, **not native UI acceptance**.
- `pnpm exec tsc --noEmit` in the isolated frontend copy: **exit 0**, no diagnostics.
- `git diff --check`: **exit 0**.
- `cargo test --locked -p meetily gigaam_engine` in the separate Windows application source copy: **exit 0, 11 passed, 247 filtered**, main target 0 tests. Full application and Tauri command registration compiled successfully; output contains existing warnings outside GigaAM. The final logged rerun used SHA256-matched current Rust sources and did not require a lockfile update.

Final raw logs are local under `D:\meetily-gigaam-downloads-20261030\logs\`: `rust-harness-final.log`, `bun-final.log`, `typescript-final.log`, and `app-final.log` for the full application command. SHA-256 comparisons matched the worktree for the four Rust module files in the harness/application copies, application `lib.rs`, Cargo manifest/lock, embedded model manifest, new frontend component/API and component test. Only verified source copies are used; no binaries are added to Git.

## Preservation and behavior

Parent-captured pre-task baseline: `/tmp/meetily-gigaam-story12-baseline/` (hashes, original tracked edits, source copies). `.gitignore`, root `Cargo.toml`, `Cargo.lock`, `docs/BUILDING.md` and `whisper_engine.rs` remain byte-identical to that baseline. Existing native Cargo edits are preserved; this story adds only runtime `sha2 = "0.10"`. `lib.rs` gains the new module/state/three commands; `TranscriptSettings.tsx` gains the independent download section. Whisper/Parakeet implementations and downloaders are unchanged.

Each model has one Rust-owned task and a cancellation token. Downloads stage under a UUID directory on the same filesystem, verify every expected file before publication, and emit progress below 100 until publication. The old valid final is reused without network access. Corrupt finals are renamed to unique quarantine directories and retained; failed publication attempts restore the previous final. Cleanup removes only known files in this task's staging directory; unexpected entries, symlink/reparse paths and quarantine data are preserved/refused. Terminal cancellation is emitted only after cleanup; a cleanup failure is an error. UI unmount only unsubscribes and does not own/cancel the Rust task. Download-path hashing observes cancellation; token checks guard publication and restore any quarantined final if cancellation arrives before staging rename. Intermediate IPC is emitted on whole-percentage changes or 1 MiB increments, plus the initial update; terminal IPC remains immediate. Error/cancelled events retain actual transferred byte counts.

## Limits

Crash-orphan staging directories are preserved. Automatic crash recovery/startup cleanup is not implemented; package readiness continues to use actual file hashes.

No real weights were re-downloaded. Full installed-package/network acceptance with all four real packages and an isolated Windows desktop UI was not performed. No native recording or inference was involved. Runtime Windows UI provider/model persistence acceptance remains unverified; preservation is supported here by unchanged settings handlers, the independent component/API and mocked command-boundary checks. Filesystem checks reject observed symlink/reparse entries; the task does not establish protection against a separate hostile process replacing filesystem entries between checks and operations.

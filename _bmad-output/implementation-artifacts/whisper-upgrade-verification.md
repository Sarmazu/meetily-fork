# Whisper upgrade verification — 2026-10-29

Implementation checkout: `fix/whisper-library-upgrade`, baseline HEAD
`a2cb62e827da7ef59f65064c97233efb2313878e`; changes are uncommitted.
**Partial verification only. CAP-1 and Windows release acceptance remain open.**
No installed application, user models/data, provider routing, `CLAUDE.md`, or
Windows CI portability hook was changed.

## Implemented

- Root/frontend Rust requirement 1.88.0; all three target declarations pinned to
  registry `whisper-rs =0.16.0`, preserving platform features.
- Both engine paths use `set_suppress_nst` and fallible `WhisperSegment` text
  iteration. Segment counts are plain integers, timestamps remain centiseconds,
  context creation receives the native path. Text errors still skip a segment;
  successful byte lengths, confidence, partial status, joining and cleaning retain
  their previous behavior.
- Numerical Vulkan regression uses registry sources discovered by locked Cargo
  metadata. It executes `ggml_soft_max_ext` with the two specified logit rows,
  both absent and specified masks, GPU buffers and CPU-reference comparisons.
  CTest has separate mask/no-mask tests and `SKIP_RETURN_CODE 77`; the runner
  returns 77 when cases skip. It rejects nonempty build directories and any
  missing/ambiguous/wrong-version/non-registry sys package.
- BUILDING documents prerequisites and verification commands. The runner uses
  CTest JUnit output (CTest >=3.21); the backend itself requires CMake >=3.19.

## Isolated tools

Initially cargo/rustc/cmake/clang were absent on PATH. Searches of usual Linux
and Windows user/tool directories and mounted C:/D: found no usable installed
build toolchain. No system packages or existing toolchain settings were changed.
Official portable tools were downloaded/extracted under `/tmp`:

- Rust 1.88.0: `/tmp/meetily-rust-1.88.0/bin`.
  Official archive SHA256 verified:
  `7b5437c1d18a174faae253a18eac22c32288dccfc09ff78d5ee99b7467e21bca`.
- CMake/CTest 3.31.8: `/tmp/cmake-3.31.8-linux-x86_64/bin`.
- Cargo cache: `/tmp/meetily-upgrade-cargo`.
- Fresh target directories: `/tmp/meetily-upgrade-target`,
  `/tmp/meetily-upgrade-vulkan-target`, `/tmp/meetily-upgrade-api-target`.

Command environment (only for the individual checks):

```bash
export PATH=/tmp/meetily-rust-1.88.0/bin:/tmp/cmake-3.31.8-linux-x86_64/bin:$PATH
export CARGO_HOME=/tmp/meetily-upgrade-cargo
export CARGO_TARGET_DIR=/tmp/meetily-upgrade-target
```

## Results

| Command / check | Observed result |
| --- | --- |
| `cargo update -p whisper-rs --precise 0.16.0` | Passed; official Cargo generated the lockfile. |
| `cargo tree --locked -p whisper-rs` | Passed: 0.16.0 → sys 0.15.0. |
| `cargo tree --locked --offline -p whisper-rs-sys` | Passed: sys 0.15.0, bindgen 0.72.1, semver 1.0.28. |
| `cargo metadata --locked --offline --format-version 1` | Passed; both Whisper packages are registry sources, no override. JSON retained only at `/tmp/meetily-upgrade-metadata.json`. All-platform default metadata includes macOS Metal/CoreML as expected. |
| `cargo check --locked -p meetily` | Blocked in glib-sys 0.18.1: missing `pkg-config` executable while probing GLib >=2.70; app Rust/API compile not reached. |
| `cargo test --locked --offline -p meetily whisper_engine` | Blocked at the same glib-sys prerequisite; tests did not execute. |
| `CARGO_TARGET_DIR=/tmp/meetily-upgrade-vulkan-target cargo check --locked --offline -p meetily --features vulkan` | Blocked at glib-sys/gobject-sys, missing pkg-config. |
| `python3 tools/whisper-vulkan-regression/run.py --build-dir /tmp/meetily-regression-locked` | Locked registry discovery passed. CMake configuration failed: missing `Vulkan_LIBRARY`, `Vulkan_INCLUDE_DIR`, `glslc`. No GPU test executed; this is a prerequisite failure, not a GPU skip/pass. |
| `g++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only -I /tmp/meetily-upgrade-cargo/registry/src/index.crates.io-1949cf8c6b5b557f/whisper-rs-sys-0.15.0/whisper.cpp/ggml/include tools/whisper-vulkan-regression/softmax.cpp` | Passed against exact locked sys headers. |
| Python AST parse / runner `--help` | Passed. |
| Isolated engine API harness | Passed in 52.44s with the exact two changed method bodies, official registry whisper-rs 0.16.0/sys 0.15.0 and a real native CPU backend build. Temporary hardware/profile/state scaffolding avoids GTK; context Path API checked too. Initial attempt failed because libclang is absent; the successful attempt used the upstream `WHISPER_DONT_GENERATE_BINDINGS=1` option with its shipped bindings, only in this temporary check. This is compile validation, not ASR/native acceptance. Command: `CARGO_TARGET_DIR=/tmp/meetily-upgrade-api-target WHISPER_DONT_GENERATE_BINDINGS=1 cargo check --offline --manifest-path /tmp/meetily-whisper-api-harness/Cargo.toml`. |
| Existing acceleration tests via temporary rustc wrapper | All 4 passed, including Vulkan/no runtime GPU, Vulkan vs CUDA flash attention, CUDA fast tiers, and CPU GPU-disabled policy. The wrapper includes the unmodified acceleration.rs and the actual hardware enum definitions, avoiding GTK. Command: `rustc --edition 2021 --test /tmp/meetily-acceleration-tests.rs -o /tmp/meetily-acceleration-tests`, then `/tmp/meetily-acceleration-tests`. |
| `rustfmt --check --edition 2021 frontend/src-tauri/src/whisper_engine/whisper_engine.rs` | Exit 1: pre-existing whole-file formatting differences. Same check against `git show HEAD:...` also exits 1. Unrelated file-wide reformatting was not applied. Used official archive rustfmt with `LD_LIBRARY_PATH=/tmp/meetily-rust-1.88.0/lib`; logs `/tmp/meetily-rustfmt-{baseline-,}check.log`. |
| `git diff --check` | Passed. |

Cargo changed the lock format from 3 to 4. Its targeted update removed unused
bindgen 0.69.5, itertools 0.12.1, lazycell 1.3.0, rustc-hash 1.1.0 and which 4.4.2;
new sys reuses existing bindgen 0.72.1 and semver. The wrapper's optional Vulkan
feature adds an existing libc edge. No unrelated package versions were upgraded.
Cargo also reports existing ignored member-level patches/profiles; these were
not changed as part of this task.

## Remaining native acceptance

Windows/MSVC release build and its actual AVX2 / no AVX-512 compiler commands;
Radeon 780M Vulkan numerical tests with ordinary FP16; synthetic import,
retranscription and final saved live text; provider switching; CPU/Parakeet ASR
smokes; macOS default Metal/CoreML and CUDA/HIP/OpenBLAS/OpenMP build checks.
These have not been run. No model-backed ASR result is claimed. The synthetic
fixture and historical evidence were preserved; they do not validate this build.

## Independent implementation audit

A second agent with a fresh context reviewed the task diff read-only against the
specification and exact upstream sources. No concrete implementation defects were
found. It independently passed the C++ syntax check and diff whitespace check.
The orchestrator reran the four acceleration tests and the locked Whisper graph;
both passed. This is step-3 verification, not completed step-4 BMAD review or native
acceptance. The spec remains `in-progress` because its matrix audit is incomplete.

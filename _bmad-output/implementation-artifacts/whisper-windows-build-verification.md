# Windows Whisper build verification — 2026-10-29

**Windows x64 Vulkan release build passed. Native GPU numerical regression passed (2/2).**
This is partial specification acceptance: CAP-1 import/retranscription/live, CPU/Parakeet ASR and provider switching were not run. BMAD step 3 remains incomplete.

## Artifact and scope

- Branch: `fix/whisper-library-upgrade`; baseline HEAD `a2cb62e827da7ef59f65064c97233efb2313878e`, with the current uncommitted task changes.
- Built executable: `D:\meetily-whisper-build-20261029\target\release\meetily.exe`.
- Prepared runtime directory: `D:\meetily-whisper-build-20261029\run`; executable `run\meetily.exe`.
- Executable size: **101370368 bytes**.
- SHA256, identical in both locations: `5abf0e74bbb7ceba64997c2130672d5f174e2b363a839355d51b102d050c48e9`.
- Tauri release command exited **0**, optimized release duration **8m 39s** on the successful attempt. No installer, signing, publication or replacement of the installed application was performed.
- The new application GUI was **not launched**. No models, meetings, settings or recordings from the installed application were copied or accessed for this build.
- Product sources, manifests and lockfiles were not edited during Windows preparation/build. The specification was not changed.

## Host and isolated tools

Windows reports `10.0.26300.0`; PowerShell `5.1.26100.9711`. GPU: AMD Radeon 780M Graphics, driver `32.0.31021.5001`. Initial free space: approximately 148 GB on C: and 562 GB on D:.

Task root below is `D:\meetily-whisper-build-20261029`.

| Tool | Verified version | Location relative to task root |
| --- | --- | --- |
| Visual Studio Build Tools 2022 | 17.14.43 / installation 17.14.37824.6 | `tools\VS2022` |
| MSVC x64 compiler | 19.44.35229; VC tools 14.44.35207 | `tools\VS2022\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64` |
| Windows SDK | 10.0.26100.0 | System Windows Kits installation |
| Rust / Cargo MSVC | 1.88.0 / 1.88.0 | `cargo\bin`; isolated `rustup` and `cargo` homes |
| Node | 20.20.2 | `tools\node-ready\node-v20.20.2-win-x64` |
| pnpm | 9.15.9 | `tools\pnpm\node_modules\.bin` |
| CMake / CTest | 3.31.8 | `tools\cmake-fast\cmake-3.31.8-windows-x86_64\bin` |
| Python | 3.11.9 embedded x64 | `tools\python` |
| LLVM / libclang | 20.1.8; DLL load passed | `tools\llvm\clang+llvm-20.1.8-x86_64-pc-windows-msvc\bin` |
| Vulkan SDK | 1.4.309.0; glslc/shaderc reported v2023.8/v2025.1 | `tools\vulkan` |

MSVC installation used the official VS2022 bootstrapper, `Microsoft.VisualStudio.Workload.VCTools`, `--includeRecommended --quiet --wait --norestart`, with Windows elevation. `vswhere` confirmed complete/launchable installation and **no reboot required**. Vulkan used LunarG's documented `copy_only=1` installation into the task directory. LLVM, Node, CMake and Python were extracted locally; Rust used isolated homes and `--no-modify-path`. No global PATH, driver or Windows long-path policy was changed by the task scripts; no reboot occurred.

Tool readiness included a real CMake/MSVC tiny C++ configure, Release build and executable run (exit 0), plus loading `libclang.dll` through Python `ctypes`.

Downloads came from official Microsoft, Rust, Node, Kitware, Python, LLVM and LunarG endpoints. Verification: Microsoft/LunarG installer and Python executable Authenticode signatures **Valid**; Node/CMake/Rustup official SHA256 checks passed; LLVM official GitHub release SHA256 digest passed. Relevant hashes and installer/download logs remain local under `downloads` and `logs`.

## Source copy and build commands

An NTFS source copy was created at `task-root\src` from 455 build/source files: tracked frontend, helper, workflow/hook files, root Cargo manifests/lockfile, and the task regression runner. `.git`, environment files, dependency/build directories, personal configuration and meeting recordings were excluded. SHA256 comparisons against WSL passed for all 455 files before build and again after build; both the Windows copies and WSL originals remained unchanged. Manifest: `task-root\source-sha256.json`.

Preparation installed pnpm locally, then ran in the copied frontend:

```powershell
pnpm install --frozen-lockfile --store-dir D:\meetily-whisper-build-20261029\pnpm-store
```

Result: exit 0, 643 packages, pnpm 9.15.9. No dependency update was performed.

Commands used with the task-local tool PATH, VS `VsDevCmd.bat -arch=x64 -host_arch=x64`, isolated Cargo/Rustup homes, and a fresh task target:

```powershell
cargo tree --locked -p whisper-rs
cargo build --locked --release -p llama-helper -vv
pnpm tauri build --features vulkan --no-bundle --verbose -- --locked
```

The locked graph passed: registry `whisper-rs 0.16.0 -> whisper-rs-sys 0.15.0`. CPU helper Release passed (3m 20s), then its executable was staged under the Tauri sidecar name. Frontend production compilation, lint/type checks and export passed. Build.rs downloaded/verified FFmpeg 8.0.1 and hash-verified ONNX Runtime 1.22.0.

The first full release attempt hit Windows **MAX_PATH**, not a source/API failure: MSBuild `MSB3491`, nested Vulkan shader-generator `VCTargetsPath.lastbuildstate` path approximately 262 characters. The failed native CMake directory is preserved as `...\whisper-rs-sys-e02f2c7b73d46bf1\out\build-maxpath-old`.

The successful retry used a temporary drive mapping to shorten native paths while retaining the Rust cache:

```powershell
subst W: D:\meetily-whisper-build-20261029\target
$env:CARGO_TARGET_DIR = 'W:\'
$env:CMAKE_PROJECT_INCLUDE = 'D:/meetily-whisper-build-20261029/src/.github/force-portable-ggml.cmake'
$env:RUSTFLAGS = '-C target-cpu=x86-64-v2'
pnpm tauri build --features vulkan --no-bundle --config D:\meetily-whisper-build-20261029\tauri-reuse-frontend.json --verbose -- --locked
node .github/verify-portable-ggml.cjs W:\release
```

The temporary Tauri config only sets `build.beforeBuildCommand` to an empty string to reuse the already successful frontend export. Product identity and data paths remain unchanged. The failed CMake cache was preserved and a fresh native CMake build was generated under the short path. Both retry and portability verification exited 0. W: was removed after verification (`subst W: /D`, exit 0); the physical build artifacts remain available.

Task-local reproducible environment/build scripts are in `task-root\scripts`. Recreate W: before reusing the successful native CMake cache; the saved build script does this and removes its mapping afterwards. Raw PowerShell logs are local UTF-16 files; they are not committed.

## Native compiler and numerical evidence

Main application's successful Whisper CMake cache and the independent numerical build both show:

```text
GGML_NATIVE=OFF
GGML_AVX2=ON
GGML_AVX512=OFF
GGML_AVX512_VBMI=OFF
GGML_AVX512_VNNI=OFF
GGML_AVX512_BF16=OFF
```

The actual MSBuild `ggml-cpu.tlog\CL.command.1.tlog` contains **`/arch:AVX2`**; no AVX-512 architecture option was found. The main application imported `vulkan-1.dll` according to `dumpbin /dependents`. Rust invocations used `-C target-cpu=x86-64-v2`. The existing portability hook was preserved. An upstream helper warning about MSVC ignoring `-march=x86-64-v2` was observed; the separately verified Whisper compiler commands use `/arch:AVX2`.

Numerical command:

```powershell
$env:PYTHONUTF8 = '1'
python tools/whisper-vulkan-regression/run.py --build-dir D:\meetily-whisper-build-20261029\numerical-utf8
```

**Result: exit 0; both `softmax-no-mask` and `softmax-mask` passed, no skips.** CTest total 1.64s. Runtime explicitly selected AMD Radeon 780M Graphics (AMD proprietary driver), `fp16: 1`. `GGML_VK_DISABLE_F16` was absent. Both cases reported finite probabilities, normalized row sums and CPU reference agreement within `1e-4`. This validates the numerical operations; it is not model-backed ASR acceptance.

Evidence remains at `numerical-utf8\ctest-results.xml`, `numerical-utf8\Testing\Temporary\LastTest.log`, compiler `.tlog` files and `logs\numerical-utf8.log`. An initial runner attempt failed decoding Cargo's UTF-8 metadata with the Windows locale codepage; enabling `PYTHONUTF8=1` resolved it without source changes. Initial premature tool invocations also failed while archive extraction was incomplete; the complete readiness probe and subsequent successful builds supersede those attempts. They were not counted as passing tests.

## Runtime directory and launch limits

`task-root\run` contains `meetily.exe`, CPU `llama-helper.exe`, `ffmpeg.exe`, `onnxruntime.dll`, `onnxruntime_providers_shared.dll`, the ONNX license, `DirectML.dll` from the build output, `vulkan-1.dll` copied from the existing Windows runtime, and shipped templates. ONNX DLL hashes in this directory match the committed build.rs verification constants.

PE imports were inspected without executing the GUI. This host has the required MSVC runtime DLLs (`MSVCP140`, `MSVCP140_1`, `VCRUNTIME140`, `VCRUNTIME140_1`, `VCOMP140`), Windows UCRT/system graphics DLLs and WebView2 156.0.4314.50. Vulkan requires the installed GPU driver; copying its loader DLL does not replace that driver.

For a later authorized native launch, the executable is:

```powershell
Set-Location D:\meetily-whisper-build-20261029\run
.\meetily.exe
```

**Do not treat this as an isolated data profile.** It retains `com.meetily.ai`, normal Tauri application data paths and the single-instance behavior. The GUI was not started, and sharing/collision with the working installation has not been experimentally evaluated. Assess data-path isolation before GUI acceptance; the build directory alone does not provide it. Production credentials were not supplied, and online/licensing behavior was not verified.

## Remaining acceptance

- Separate Windows `cargo check`/filtered `cargo test whisper_engine` commands were not run after the successful full Release compilation; additional costly builds were deferred at handoff. The full app Rust/native compilation and the independent Vulkan numerical tests above did run.
- CAP-1 synthetic import, retranscription and final saved live transcript are not run.
- CPU/Parakeet ASR and provider switching are not run.
- macOS Metal/CoreML, CUDA/HIP/OpenBLAS/OpenMP matrix checks are not run on this Windows host.
- No claim of complete specification acceptance or BMAD step-3 completion is made.

Official installation references: [Microsoft command-line parameters](https://learn.microsoft.com/en-us/visualstudio/install/use-command-line-parameters-to-install-visual-studio?view=vs-2022), [VS Build Tools components](https://learn.microsoft.com/en-us/visualstudio/install/workload-component-id-vs-build-tools?view=vs-2022), [LunarG Windows SDK / copy-only mode](https://vulkan.lunarg.com/doc/view/1.4.313.0/windows/getting_started.html), [Rustup isolated homes](https://github.com/rust-lang/rustup/blob/main/doc/user-guide/src/installation/index.md).

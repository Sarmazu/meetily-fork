# Native Whisper acceptance — 2026-10-29

Status: **partial acceptance; BMAD step 3 remains incomplete**. No CAP-1 result is inferred from the successful numerical tests or Release compilation.

## Isolation established before application launch

The original Release executable retains `com.meetily.ai` and is not a safe disposable profile. Merely changing `APPDATA`/`LOCALAPPDATA` environment variables does not redirect Windows known folders: the resolved `dirs-sys` uses `SHGetKnownFolderPath`.

Source inspection established:

- Tauri 2.11.1 `path/desktop.rs` app data/config paths append `config.identifier` to the Windows known folder. Database/legacy migration, stores and normal model directories use this path.
- `tauri-plugin-single-instance 2.3.7/src/platform_impl/windows.rs` derives its mutex and hidden window names from `app.config().identifier`.
- Tauri `manager/webview.rs` normally appends the identifier to LocalAppData; the final runtime storage path was independently checked against this identifier.
- Identifier alone is insufficient: `notifications/settings.rs` uses the shared `meetily/notifications.json` path and startup loads and saves consent/permission automatically. Custom templates and fallback model paths also use shared `Meetily`; the Windows recording default uses Music.
- Windows first-launch legacy detection only looks inside the identifier-specific app data path. The Homebrew detection branch is macOS-only.

The task-only NTFS source copy was temporarily given six **path-only** changes for the diagnostic builds: notification directory, custom templates directory, Whisper/Parakeet/summary fallback directories, and the Windows recording default. Neither repository product source nor inference/dependency logic was edited for isolation. A separate Tauri config changes the identifier to `com.meetily.whisperacceptance20261029`, gives the window an explicit acceptance title, and attempts a task-directory WebView `dataDirectory` override. Runtime verification established that this override is ignored by the resolved config-to-WebView conversion; the effective storage remains **LocalAppData/com.meetily.whisperacceptance20261029/EBWebView**, isolated by the unique identifier. No claim of D:-local WebView storage is made.

Task root: `D:\meetily-whisper-build-20261029\native-acceptance`. The diagnostic runtime is `run`; temporary files and recordings are task-local. WebView storage is in fresh identifier-specific LocalAppData; data/stores/models are fresh identifier-specific roaming AppData; shared-path replacements use the distinct suffix `MeetilyWhisperAcceptance20261029`. No global known folder, environment setting, Windows account, installed application or installed executable was changed.

Local isolation evidence: `evidence/path-only.patch`, `path-patch-sha256.json`, `expected-paths.json`, `tauri-acceptance.json`. The original runtime remains separately at `D:\meetily-whisper-build-20261029\run`.

Only the exact `ggml-large-v3-turbo.bin` weight was copied from the existing model directory into the fresh profile. Both copies have SHA256 `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69`, 1624555275 bytes. No existing settings, meetings, transcripts or recordings were copied. The exact synthetic fixture was copied with SHA256 `1eb645414fb61898b1b0c2df5b5eb077948a6ee95b1bbff214e70ef09bef75f4`.

Fresh test stores and application defaults keep analytics opted out, and the patched recording default keeps output in the task directory. Onboarding is seeded completed to avoid unrelated summary-model downloads; its model status is explicitly `not_downloaded`. Onboarding behavior is not under acceptance.

## Execution and matrix

Diagnostic Vulkan Release build passed (`pnpm tauri build --features vulkan --no-bundle --config <task>/tauri-acceptance.json --verbose -- --locked`, exit 0, 4m02s). SHA256: `4436e789ed9cba1e0892f00dc05e30caebfedfaef1c7b3f99667cd94035a08e1`. Original runtime hash was rechecked unchanged: `5abf0e74bbb7ceba64997c2130672d5f174e2b363a839355d51b102d050c48e9`. Native acceptance here applies to this path-isolated diagnostic build; the original Release GUI was not launched.

Native Vulkan PID 21344 started at 14:39:43 UTC. Runtime confirmed a fresh identifier-specific database, model paths and task recording directory. WebView CDP listened only on `127.0.0.1:19336`. Test setup used the normal `save_onboarding_status_cmd` after UTF-8 BOM in initial seeded JSON prevented store parsing; fresh application defaults were used until then. No existing meetings or personal settings were read; model-weight reads are listed separately below. The first launch request hit an automatic approval-review timeout before execution; one tool-authorized retry succeeded.

UI automation emitted the synthetic file path through the normal Tauri drag-drop event, then clicked the regular Import/Advanced/model/retranscription buttons in the native WebView. It did not replace inference with browser code or a CLI harness.

| Case | Status | Required evidence |
| --- | --- | --- |
| GPU numerical softmax | PASS (separate report) | Windows build verification, 2/2 on Radeon 780M, ordinary FP16 |
| Native Vulkan import | **PASS** | 14:41:10–14:41:21 UTC; large-v3-turbo, auto language; exact 202080 samples; saved UI text passes all three phrases |
| Native Vulkan retranscription | **PASS** | Same synthetic meeting; 14:41:43 UTC start; saved UI text passes all three phrases |
| Native live, saved final text | Not run | Synthetic-only input has not yet been established |
| Parakeet → Whisper → Parakeet | **Functional sequence PASS; strict semantic acceptance FAIL** | v3 → Whisper → v3, same PID, no restart; Whisper passes all phrases, Parakeet v3 misses required first phrase |
| CPU ASR smoke | **PASS** | Separate native CPU build; exact WAV saved as new synthetic meeting; all three required phrases pass |

The available connected automation is Playwright, with no desktop/computer connector. A process-scoped, loopback-only WebView2 CDP connection was used through Windows .NET `ClientWebSocket`; this targets the native Tauri WebView rather than a browser dev server. Launch guards refuse an unstaged original executable, an occupied debug port or a non-loopback CDP listener. The CPU runtime guard additionally verifies the browser process `--user-data-dir` against the unique LocalAppData identifier. An initial overstrict expectation of D:-local storage stopped the first CPU process before import; the source-backed expectation was corrected and the effective safe profile was confirmed. No microphone or general loopback recording will begin without a proven synthetic-only source.

## Observed native results

Whisper import/retranscription/switch-back saved UI text:

> This is a synthetic speech recognition test We are checking the local transcription engine The quick brown facts jump over the lazy dog

Case/punctuation/whitespace normalization passes `synthetic speech recognition test`, `local transcription engine`, and `quick brown`. `facts` is the explicitly allowed local fox/facts variation. Native logs show `compiled_backend=Vulkan`, `use_gpu=true`, `flash_attn=false`, Radeon 780M `fp16: 1`, 1623.92 MB on Vulkan0 and `using Vulkan0 backend`; the launched process had no `GGML_VK_DISABLE_F16`. The saved transcript and UI were checked after completion, not only progress events.

Parakeet v2 weights were downloaded into the isolated profile from the code-pinned Hugging Face revision `0bbb45a3365852604aef28b538a8f066f4ccaa85`. V3's four exact model artifacts were separately copied from the known installed model directory. Both sets passed the application's exact-byte specification; source/copy SHA256 equality was checked for v3. Hash manifests are local `parakeet-v2-sha256.json` and `parakeet-v3-sha256.json`. No hardlinks or user configuration copies were used.

V2 observed text:

> Tis a synthetic speech ritzagnetion test. We are rechecking the local transcription engine. The quick brown fox jumps over the lazy dog.

V3 observed text, repeated identically after switching back from Whisper:

> TCSA и Synthetics Peach Rets Ignition Test. We are checking the local transcription engine. The quick brown fax jumps over the lazy dog.

Both Parakeet models perform native inference and save text, but fail the required `synthetic speech recognition test` phrase. V3 also contains a Cyrillic token. This failure remains explicit; the criteria were not relaxed. No before/after baseline A/B was run, so neither a Parakeet regression nor preservation of baseline accuracy is established. Switching itself works through the retranscription model selector in the same native process; live provider switching and global Settings changes are not claimed.

Local semantic evidence: `native-retranscribe-result.json`, `native-parakeet-first-result.json`, `native-v3-first-result.json`, `native-switch-whisper-complete.json`, `native-switch-v3-complete.json`, and `native.stderr.log`. All transcript evidence is synthetic. Vulkan was exited through the ordinary Tauri process plugin at 14:45:57 UTC before the separate CPU build.

## Live boundary and minimum walkthrough

Read-only native device enumeration found one Realtek output (Динамики) and one Realtek input (Микрофон), with no virtual input; `is_recording=false`. Source `recording_commands.rs:158` explicitly states there is no "no microphone" picker option: null means the default microphone, and an invalid name also falls back to it. Therefore leaving the mic selector blank does not exclude ambient speech.

Before any live test, the user must establish a synthetic-only source: hardware-mute/disable the real input with explicit confirmation and eliminate other playback, or provide a dedicated virtual synthetic input. No global device settings are changed by this task. Do not start recording merely because synthetic playback is ready.

Once that condition is confirmed: launch the **diagnostic Vulkan** executable only; in Settings select Local Whisper / large-v3-turbo and automatic language, keep automatic summaries off; verify recording output remains the task directory. Start a new synthetic meeting, play only `<task>/synthetic-en.wav` once through the confirmed source, wait for processing, then Stop. Open the saved meeting and check all three required phrases in final saved text, plus native Vulkan/FP16 log evidence. A transient transcript before Stop does not complete acceptance. If the physical input cannot be excluded, leave live not run.

## Source and runtime preservation

After diagnostic Vulkan and CPU binaries were staged separately, the six task-only source patches were restored from their original snapshots with hash guards. All **455/455** copied build/source files again match `source-sha256.json`; evidence: `source-restoration.txt`. The six original source snapshots also match the repository files byte for byte. The diagnostic binaries retain only their documented path changes. Patch/hash evidence is retained; the WSL product source was never changed for isolation.

`target/release/meetily.exe` is a scratch build output and now contains the CPU diagnostic build. The preserved original Vulkan product executable is still `<build-root>/run/meetily.exe`; the path-isolated Vulkan executable is `<build-root>/native-acceptance/run/meetily.exe`. The CPU diagnostic runtime is `<build-root>/native-acceptance/cpu-run`.

CPU diagnostic compilation passed, exit 0, 4m41s, using the same task overlay without GPU features. CPU executable SHA256: `24fa52c658f7be96e63681588df295aa9ca9c5e9e92336a8637f8eef4436b069`. Its native cache has `GGML_VULKAN=OFF`, `GGML_NATIVE=OFF`, `GGML_AVX2=ON`, all AVX512 variants OFF; the actual CPU compiler command contains `/arch:AVX2` and no `/arch:AVX512`. The CUDA/HIP/Metal matrix remains outside this Windows runtime check.

CPU runtime confirmed the same isolated profile and actual `--user-data-dir=<LocalAppData>/com.meetily.whisperacceptance20261029/EBWebView`, with only a loopback CDP listener. Evidence: `runtime-webview.json`. Its exact-fixture import started at 14:52:42 UTC, logging `compiled_backend=Cpu`, `use_gpu=false`, `whisper ... use gpu = 0`, 1623.92 MB CPU model storage and `no GPU found`. Runtime GPU hardware detection alone therefore did not substitute for the separately compiled CPU backend.

The manual launcher is `<build-root>/native-acceptance/scripts/native-launch-manual.ps1` and intentionally enables no debug port. It verifies the diagnostic Vulkan executable hash, avoids an already running acceptance process, scopes temporary files to the task directory, and preserves stderr/stdout for the new session. For exact synthetic playback after recording has started and input isolation is confirmed, Windows PowerShell can use:

```powershell
(New-Object System.Media.SoundPlayer 'D:\meetily-whisper-build-20261029\native-acceptance\synthetic-en.wav').PlaySync()
```

To quit after manual testing, use the tray Exit action; closing the main window only hides it.

A CPU performance limitation was observed during this smoke: although Rust reports an optimized Release build, the sys crate's generated CPU `CMAKE_C_FLAGS_RELEASE` and `CMAKE_CXX_FLAGS_RELEASE` contain the baseline reproducibility/runtime flags without `/O2`, and the actual CPU CL command likewise contains no `/O2`. AVX2/FMA/F16C are enabled. No optimization flags or dependency code were changed to hide the slow inference, and no performance acceptance is claimed.

## Final CPU result and shutdown

CPU import completed at **15:02:46 UTC**, approximately **10m04s** after import start. It produced one saved transcript and a new synthetic meeting. `native-cpu-result.json` shows the final saved UI text:

> This is a synthetic speech recognition test We are checking the local transcription engine The quick brown facts jump over the lazy dog

All three required phrases pass normalization. Thus CPU semantic smoke **passes**, while its observed duration remains an explicit performance limitation. The result arrived before the bounded shutdown deadline; no timeout, cancellation or fallback was used to produce it.

The CPU app exited through the normal Tauri process plugin at 15:03:40 UTC. The finalizer verifies that its process is gone and TCP port 19336 has no listening socket; `final-process-state.txt` records this. The original product runtime, isolated Vulkan runtime and CPU runtime hashes were rechecked unchanged. No acceptance app or debug listener is left running. `semantic-checks.json` records the independent phrase checks; `cpu-native.stderr.log` records CPU model storage, inference completion and normal cleanup.

**Remaining:** live saved text requires the user's confirmed synthetic-only audio setup. Strict Parakeet semantic acceptance remains failed for the exact fixture; baseline A/B has not established whether this is pre-existing. GPU import/retranscription, functional same-process switching and CPU ASR are verified, but the full matrix is not complete and BMAD step 4 must not start.

After successful app cleanup, WebView emitted a shutdown-only `Chrome_WidgetWin_0` unregister error 1412. Process exit and CDP closure were independently verified afterward; this is recorded as a shutdown observation, not an ASR acceptance failure.

## Live attempt and user-requested stop — 2026-10-30

The user initially confirmed microphone muted and other playback stopped. Devices had changed since the earlier check: Windows selected Realme Buds by default. The agent incorrectly started recording before inspecting the device preflight result. Start was at 12:52:07 UTC; the normal Stop action was issued at 12:52:28 UTC. Synthetic playback was never started. This attempt is invalid for acceptance. Audio/transcript contents from this attempt must not be read, exported, quoted, or treated as synthetic evidence; they may remain locally in the isolated diagnostic profile and require metadata-only identification before cleanup. No assertion that these artifacts contain only silence is made.

The user explicitly requested stopping the live test. After daemon recovery, the orchestrator found the diagnostic app still open, verified `is_recording=false`, and requested a normal Tauri process exit. No repeat recording is authorized by the subsequent request to continue cleanup. Live acceptance remains not completed; the previously recorded Whisper import/retranscription/CPU results are unchanged.

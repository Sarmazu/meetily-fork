# GigaAM FP32 model contract

Standalone preparation and Windows CPU verification for story 1.1. No Meetily,
GPU, microphone, playback, real recordings, global dependency installation or
speed measurements are involved. Binary files stay outside Git. The manifest
pins actual downloaded bytes, model/source revisions and both MIT licenses.
No model export or quantization is performed.

Use Python 3.11 x64. Create a **new** task directory outside the repository; the
following PowerShell example assumes an existing Python with venv support:

```powershell
$task = 'D:\meetily-gigaam-contract-reproduction'
if (Test-Path $task) { throw 'Choose a fresh task directory' }
New-Item -ItemType Directory $task | Out-Null
py -3.11 -m venv "$task\venv"
$python = "$task\venv\Scripts\python.exe"
& $python -m pip install -r tools/gigaam-model-contract/requirements.txt
& $python tools/gigaam-model-contract/prepare.py --cache "$task\cache"
if ($LASTEXITCODE -ne 0) { throw 'Preparation failed' }
& $python tools/gigaam-model-contract/prepare.py --cache "$task\cache" --check-only
```

`prepare.py` downloads immutable revision URLs, verifies complete byte counts
and SHA-256, and refuses unknown, altered, extra files, symlinks and Windows reparse points. It validates
existing files before downloading. Keep cache and result/environment directories
separate. Verified files are published atomically with an exclusive hard link on the
same filesystem. An interrupted staging directory is preserved and fails the
next run; inspect it and choose a fresh cache instead of silently deleting it.
These checks do not reject every hardlink and do not make files immutable.
Integrity is checked at preflight; concurrent mutation of the cache during
preparation or inference is unsupported. Use a dedicated task cache without
other writers. `--check-only` never accesses the network. The source snapshot includes the
reference Python modules; only the GigaAM, decoding and NumPy preprocessing
modules are loaded by the harness. Its dependencies are exactly pinned in
`requirements.txt`; no Hugging Face client, PyTorch or source package build is
needed. `ml_dtypes` reproduces the reference bfloat16-rounded *constants*;
all exported models and inference remain FP32.

Create the synthetic Russian fixture directly to a mono PCM16 16 kHz file:

```powershell
& tools/gigaam-model-contract/synthesize.ps1 -Output "$task\synthetic-ru.wav"
$phrase = 'Сегодня мы проверяем распознавание русской речи. Все четыре модели работают локально на компьютере.'
$env:PYTHONUTF8 = '1'
& $python -s tools/gigaam-model-contract/verify.py --cache "$task\cache" `
  --wav "$task\synthetic-ru.wav" --expected-text $phrase --output "$task\results.json"
if ($LASTEXITCODE -ne 0) { throw 'Acceptance incomplete; inspect all four JSON results' }
```

The synthesis script enumerates voice names and requires Microsoft Irina
Desktop Russian. Missing voice or unavailable Windows/runtime means incomplete
acceptance. Do not substitute personal recordings. Different voice versions may
produce different WAV bytes: record SHA-256 from the actual JSON report.

The harness enforces every runtime dependency version in `requirements.txt`,
requires numpy/onnxruntime/ml_dtypes and distribution metadata inside the isolated
prefix, and rejects user/system site-packages and external search/.pth origins.
It records dependency versions and critical module origins in JSON.
The harness requires native Windows x64, isolated Python, CPU ORT **1.22.0**,
no installed GPU provider, exact cache integrity and a 320..400000-sample fixture.
AzureExecutionProvider may be present in the official CPU wheel but is never
selected. Every ONNX session must report **only CPUExecutionProvider** and has
fallback disabled before inference. JSON includes platform, versions, paths,
manifest and WAV SHA-256, provenance, per-session ABI, four original transcripts
and independent semantic assessment. JSON outputs are exclusive-created.

Semantic PASS requires exact Unicode lexical agreement with the known phrase,
ignoring case, punctuation and ё/е only in the *comparison*. Extra Latin words
and `<unk>` are retained by tokenization and prevent PASS. Raw E2E text is
preserved unchanged. A difference is an explicit failure for human examination,
never automatically approved. Inspect the saved original text as well as exit
status. The harness also records the full `[1,64,2]` reference feature tensor for a
fixed 480-sample sine waveform, its exact NumPy input formula and byte hashes;
see `reference-features.json`. It does not measure or log inference speed.

Actual 2026-10-30 execution uses a separate copy of the pre-existing Python
3.11.9 embedded distribution under `D:\meetily-gigaam-contract-20261030\python`,
with task-local `Lib\site-packages`, enabled `import site` in `python311._pth`,
and `python -s` to disable user site-packages explicitly.
The old Python and installed Meetily remain untouched. See
`_bmad-output/implementation-artifacts/gigaam-model-contract-verification.md`
for actual evidence and limitations, and
`_bmad-output/specs/spec-gigaam-cpu/model-contract.md` for decoding details.

Small offline tests (no dependency install or model download):

```bash
python3 -m unittest discover -s tools/gigaam-model-contract/tests
python3 tools/gigaam-model-contract/prepare.py --help
python3 tools/gigaam-model-contract/verify.py --help
```

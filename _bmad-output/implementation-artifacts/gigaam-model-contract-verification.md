# GigaAM model contract verification — 2026-10-30

**Standalone native Windows CPU matrix PASS: 4/4.** This completes the
model-package compatibility acceptance of story 1.1; it does not claim that
GigaAM is integrated into Meetily or that any dependent story was implemented.

## Scope and environment

- Branch retained: `fix/whisper-library-upgrade`; HEAD baseline
  `a2cb62e827da7ef59f65064c97233efb2313878e`. Existing product changes preserved;
  this task writes tools and BMAD artifacts only. No commit, push or publication.
- Native Windows `10.0.26300`, AMD64; Python 3.11.9 x64 embedded, copied into a
  new task root `D:\meetily-gigaam-contract-20261030\python` from the existing
  isolated 2026-10-29 tool distribution. Original Python remains unchanged.
- Task-local pip packages match every runtime pin in
  `tools/gigaam-model-contract/requirements.txt`, including `onnxruntime==1.22.0`,
  `numpy==2.2.6`, `ml_dtypes==0.5.1`. Pip bootstrap tooling lives only in this
  new environment (pip 26.2.2, setuptools 84.2.1, wheel 0.48.0). The final
  harness uses `python -s`, rejects user/system site-packages and external
  search paths/.pth path origins, checks every pinned runtime distribution
  version/location and numpy/onnxruntime/ml_dtypes module origins. All versions
  and critical origins are recorded in final JSON.
- Official CPU wheel offers AzureExecutionProvider and CPUExecutionProvider;
  every created inference session reports **only CPUExecutionProvider**, with
  fallback disabled. No GPU provider is installed or selected.
- Installed TTS enumeration: Microsoft Irina Desktop Russian, Microsoft Zira
  Desktop English US, Microsoft David Desktop English US. Irina used with SAPI
  file output, SAFT16kHz16BitMono (18). No playback, microphone or GUI.
- No installed Meetily launch, model/settings/meeting access, global installation,
  runtime upgrade, GPU, INT8, performance benchmark or inference-speed measurement.

## Provenance and actual file verification

Model source: `istupakov/gigaam-v3-onnx` revision
`322c3b29492673eb7d0b434bfa9dfb8653e34d02`.
Reference source: `istupakov/onnx-asr` revision
`a2fd7530c01a78727422e592bc67ab98d96a9697`.
Both MIT licenses are included in the exact manifest. Actual bodies of all 55
files were downloaded into `/tmp/meetily-gigaam-contract-20261030/cache`, hashed
locally, then staged and rehashed by the Windows harness at
`D:\meetily-gigaam-contract-20261030\cache`. ONNX LFS metadata SHA-256 and size
were also compared against the downloaded bytes and matched. No binaries are
stored under the Git worktree. No upstream model export was performed.

Manifest SHA-256: `ee1e632dd79720e6198ddb353d3273b30d6f8b5c4073f94e34310c9fcc3560f7`.

| FP32 graph | Actual bytes | SHA-256 |
| --- | --- | --- |
| `models/v3_ctc.onnx` | 885264128 | `1fb978d4f41e1334003fae9d29f2d9844d4132315a90553a4b986839051ba9d3` |
| `models/v3_e2e_ctc.onnx` | 885950079 | `377701bd33568f4733feec2db5b2dc12544fd09a5a5dfa69ccf55d161f84027a` |
| `models/v3_e2e_rnnt_decoder.onnx` | 4599910 | `7b0a16d67fd2cb37061decc93c69e364a9ab27afee3c57495d55b1c974cf7231` |
| `models/v3_e2e_rnnt_encoder.onnx` | 885084534 | `cd60b3764a832e8560ae6d3ad0b10adc1a42ffae412b9476f25620aae4f4a508` |
| `models/v3_e2e_rnnt_joint.onnx` | 2712896 | `602ff7017a93311aad34df1437c8d7f49911353c13d6eae7a6ee7b041339465c` |
| `models/v3_rnnt_decoder.onnx` | 3331429 | `443c3b7bd42b453611618135d6b1e7d9467e5dd97c8a68501da4aa355750c0da` |
| `models/v3_rnnt_encoder.onnx` | 885084534 | `7ae7509c3f1128369564df0b00e2ee4950adf539de2392ac5c800a5bc04c7132` |
| `models/v3_rnnt_joint.onnx` | 1440448 | `fd1d02f45c2ad3d6b67cc149811ad794ab4b020ed49a0a9e2790a8619d1cddd8` |

Reference implementation bodies used for preprocessing/decoding (also verified
inside the full cache). The harness imports pinned `GigaamPreprocessorNumpy`;
its initializer supplies exactly the pinned periodic-Hann and HTK filterbank
constants with the BF16-to-FP32 rounding prescribed by pinned
`preprocessors/gigaam.py`. It does not replace the reference FFT/decoding logic.

| Reference file | Bytes | SHA-256 |
| --- | --- | --- |
| `source/preprocessors/fbanks.py` | 1826 | `d3bda8a7b7813f2f279e6e901e1d9e832600badaa22e2b7fd9298104a07223af` |
| `source/preprocessors/gigaam.py` | 4153 | `de170cb2d2066f9ebe803ecb0cbf48a14ca6442b5e205a58bab0334861dce2c6` |
| `source/src/onnx_asr/asr.py` | 8272 | `90d79832a9f5073270e604dc19308a56927cecabcf29815de97825e19187a862` |
| `source/src/onnx_asr/models/gigaam.py` | 6637 | `15879d724aab4fc71bd65bd14a38a5070ddc5788160a7a628d0766d53f51ba33` |
| `source/src/onnx_asr/preprocessors/numpy_preprocessor.py` | 8985 | `215b10e9549843feb5961f9a5c03bfa79270b5c08f7bfdd34388bce3f6f0a83f` |

## Synthetic fixture and command

WAV `D:\meetily-gigaam-contract-20261030\synthetic-ru.wav`: mono, uncompressed
PCM16, 16000 Hz, 141120 samples. SHA-256
`034a71efec3f2eb19b44093ba8f029a5d9e90db64ef4a2d14a5d4f08f8731659`.
Known text: «Сегодня мы проверяем распознавание русской речи. Все четыре модели
работают локально на компьютере.» Source text is synthetic and contains no
meeting data. Regenerate with the committed BOM-encoded `synthesize.ps1`;
voice/version differences may produce different bytes and must be rehashed.

Executed final native command (PYTHONUTF8=1):

```powershell
$root = 'D:\meetily-gigaam-contract-20261030'
& "$root\python\python.exe" -s "$root\harness\verify.py" `
  --cache "$root\cache" --wav "$root\synthetic-ru.wav" `
  --expected-text 'Сегодня мы проверяем распознавание русской речи. Все четыре модели работают локально на компьютере.' `
  --output "$root\results-reviewfix2-isolated.json"
```

Final command exit 0; JSON status pass; exactly four distinct model results,
all nonempty and semantically equal to the source phrase. Initial and pre-review runs also passed. After review fixes, a stricter
environment check correctly rejected embedded Python with user-site enabled,
recording four not-run rows in `results-reviewfix.json`. The final isolated
command adds `-s` to disable user-site explicitly; it does not relax any check.
All original/rejected evidence files are preserved. No failed result was
converted into PASS.

| Model | Active providers | Original output | Assessment |
| --- | --- | --- | --- |
| `v3_ctc` | CPUExecutionProvider only | сегодня мы проверяем распознавание русской речи все четыре модели работают локально на компьютере | PASS; exact lexical match |
| `v3_rnnt` | CPUExecutionProvider only | сегодня мы проверяем распознавание русской речи все четыре модели работают локально на компьютере | PASS; exact lexical match |
| `v3_e2e_ctc` | CPUExecutionProvider only | Сегодня мы проверяем распознавание русской речи. Все четыре модели работают локально на компьютере. | PASS; exact lexical match |
| `v3_e2e_rnnt` | CPUExecutionProvider only | Сегодня мы проверяем распознавание русской речи. Все четыре модели работают локально на компьютере. | PASS; exact lexical match |

The saved texts were inspected directly against the known source phrase.
Ordinary CTC/RNNT omit capitals/punctuation as expected; both E2E outputs match
the complete original phrase exactly, with its punctuation untouched. Automated
semantic comparison uses Unicode word tokens, retaining Latin words and the
`unk` token from `<unk>`, and ignores case/punctuation and ё/е only in the comparison;
the original text is never rewritten. Empty text, alternate provider, missing
or duplicate model, failed status, or lexical mismatch prevents matrix PASS.

Verbatim JSON content is mirrored at
`_bmad-output/implementation-artifacts/gigaam-model-contract-results.json`.
It contains environment, manifest/fixture identity, source revisions, four
outputs and actual tensor names/shapes/dtypes. Canonical ABI digests below use
UTF-8 `json.dumps(result['abi'], sort_keys=True, separators=(',', ':'))`:

| Model | Canonical ABI SHA-256 |
| --- | --- |
| `v3_ctc` | `4b5b34e43ffc85de3e9c6e336d1c2c09a4bd8b7b295b9e6ae3ab323f4a6f029f` |
| `v3_rnnt` | `2973e7469d541a4e881b3e75f9dc3513f2daa415f4b494a2833b3678ae439bce` |
| `v3_e2e_ctc` | `585f57cebcb0c9aebea8870776639ec8eaf26dde3cbd52265933202af6b3b0df` |
| `v3_e2e_rnnt` | `c2ce600b3bb0ec850c7bb77b6728814289aea7ba4926fec46f27443c535422a0` |

JSON mirror SHA-256: `2c632f6c6015d09870c4f8e8b114866b2890aba0235fe578349f0c295fe43c65`.

Newest executed harness bodies were compared byte-for-byte with the worktree:

| File | SHA-256 |
| --- | --- |
| `verify.py` | `bda8ed050c8919744d610d478447cba8902a716d6a5b4f74da15942f741e76d7` |
| `contract.py` | `e66460524e39abb5ea2781b79c2eb766dd3c6810edafab16bd291835fa696cbc` |
| `prepare.py` | `52ba84c5a39f061ffc861788a4c99fc1e9fb8d1fab2ef7c3cec5a1eaf23fea24` |

## Other verification and limitations

- `python3 tools/gigaam-model-contract/prepare.py --cache /tmp/meetily-gigaam-contract-20261030/cache --check-only`: PASS, 55 full file bodies verified.
- `python3 -m unittest discover -s tools/gigaam-model-contract/tests`: 30 PASS;
  synthetic tiny files only, including missing/corrupt/same-size-substituted/
  symlink/unknown entries, an actual local-URL verified download, corrupt-download
  non-publication, unsafe per-model roles, Unicode/unknown token false-PASS,
  actual execute provider rejection using mocked sessions, dependency versions/
  origins/isolation and oversized/truncated WAV rejection before allocation.
- Both command `--help` interfaces pass. `git diff --check` passes.
- Reproduction instructions, exact package/source identities, tensor ABI,
  preprocessing, CTC/RNNT state/emission rules, vocab and detokenization, and
  reference inputs/outputs are recorded in the README and `model-contract.md`.
- This is one known synthetic Russian phrase on one Windows host, not a broad
  ASR quality evaluation, Rust numerical parity, model downloader/app behavior,
  live/import/retranscription, offline restart or full epic acceptance.
- Models and fixture remain outside Git at the explicit local paths. Different
  synthetic fixtures require an explicit known expected phrase and fresh output
  file; differences stay failed for human inspection. No downstream story was
  started. No speed metric was collected.

## Post-review nonzero preprocessing reference

`tools/gigaam-model-contract/reference-features.json` records a deterministic
480-sample, 440 Hz sine at amplitude 0.25. The exact NumPy float64 construction
then float32 cast is included as its input formula. It includes input/output
shapes/dtypes, little-endian float32 byte hashes, all 128 `[1,64,2]` feature
values, output lengths, pinned source revisions/files and dependency versions.
It was generated directly by the pinned NumPy preprocessing implementation in
the final Windows matrix. A separate isolated Windows check reloaded the saved
JSON, revalidated the full source/model cache and environment, recomputed the
pinned reference features and compared **every** saved input/features/lengths
field exactly: PASS. This is reference material, not Rust numerical parity.

Reference JSON SHA-256: `739ec80b3b656cf345625ab668a07fc026005d4d8776f6b3a99e7919fd0e665f`.
Feature little-endian float32 SHA-256:
`cd455f5432249be396c6a98b4341704e0ab4251f106b5df29aa9b1dafe184358`.

Post-review fixes additionally constrain each model role to that model's
verified declared files, validate WAV frame bounds before payload read and
exact payload bytes before NumPy allocation, and release the temporary session
loop reference after fallback disabling. Cache checks reject symlinks/reparse
points; they are preflight checks, not immutable snapshots or a guarantee to
reject all hardlinks. Concurrent cache mutation is unsupported.

Prior Windows evidence remains under `D:\meetily-gigaam-contract-20261030`:
`results-initial.json`, `results-final.json` (pre-review PASS), and
`results-reviewfix.json` (strict user-site rejection). Final post-fix evidence:
`results-reviewfix2-isolated.json` (4/4 PASS). Prior normalized JSON mirror hash
was `18d5836503fced0e5cf31a8e0d83892b672686ee93fa955c8720dd2ad6560f63`. Prior executed harness hashes, retained for audit:

| Pre-review file | SHA-256 |
| --- | --- |
| `verify.py` | `c86c28bd8a5ca227ad921a8dd743261caf02e7345ed92739a06f2d856c108622` |
| `contract.py` | `83d6deebe01466cd3543d5912c04bc40cfcfb7d5d4f1076dd743b440de73cd3b` |
| `prepare.py` | `52ba84c5a39f061ffc861788a4c99fc1e9fb8d1fab2ef7c3cec5a1eaf23fea24` |

## Python-version and failure-cleanup follow-up

The harness now explicitly requires Python major/minor **3.11**. Every model
iteration starts with model/sessions cleared and releases both references in
`finally`, including inference exceptions and semantic mismatch. Focused tests
reject Python 3.10/3.12/4.11 and use weak references to confirm failed models and
sessions are gone before the next model is constructed and after the matrix
returns. The standalone tool suite has 30 passing tests.

Newest native acceptance: `D:\meetily-gigaam-contract-20261030\results-reviewfix2-isolated.json`,
Python 3.11.9 with `-s`, CPU ORT 1.22.0, **4/4 PASS**. All final harness bodies
were compared byte-for-byte to the executed Windows copy again. Earlier
`results-reviewfix-isolated.json` remains preserved alongside the other prior
JSON files. Its executed `verify.py` SHA-256 was `47f195268a7db5829e40a27baa19ed6559fada1495a8ce422bad0ec78be0e945`.
Newest `verify.py` SHA-256 is `bda8ed050c8919744d610d478447cba8902a716d6a5b4f74da15942f741e76d7`.
No timing metrics were collected; no application behavior was changed.

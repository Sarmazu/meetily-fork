# Evidence review — GigaAM architecture

Verdict: PASS WITH TWO TARGETED CLARIFICATIONS. No demonstrated technology incompatibility; native Windows/model-binary compatibility remains unverified as explicitly deferred.

## Confirmed

- Cargo.lock pins both ort and ort-sys to 2.0.0-rc.10. The Windows bundle builder pins ONNX Runtime 1.22.0 with size and SHA-256 checks. Reusing these as a starting baseline is evidence-backed; claiming that GigaAM already runs on them would not be.
- Existing TranscriptionProvider accepts 16 kHz mono f32 audio and returns text, optional confidence and partial flag, matching the proposed adapter boundary. Parakeet already uses CPUExecutionProvider.
- The model owner's card lists all four CTC/RNNT/E2E models and CPU inference: https://huggingface.co/istupakov/gigaam-v3-onnx/raw/main/README.md . Its implementation confirms separate CTC and encoder/decoder/joint RNNT paths and per-model vocabularies: https://github.com/istupakov/onnx-asr/blob/main/src/onnx_asr/models/gigaam.py . This supports the design choice but does not establish ABI compatibility of any downloaded binary.
- Existing import/retranscription code uses binary Parakeet-versus-Whisper routing, including cleanup. AD-6 correctly requires handling GigaAM before those defaults.

## Findings

1. **Medium — existing silence splitter is not a strict 25-second limiter.** AD-5 requires each GigaAM input to remain within 25 seconds using the existing mechanism. In audio/common.rs, split_segment_at_silence searches +/-3 seconds around the target and may return a segment longer than the target. Clarify that GigaAM must enforce a strict final bound (adapt/supplement the helper without changing legacy behavior), and test each emitted segment <=400,000 samples. Otherwise a builder may mistakenly assume the helper already satisfies the invariant.
2. **Medium — live provider language is reread per segment.** audio/transcription/worker.rs currently calls get_language_preference_internal immediately before provider.transcribe. AD-4's immutable provider/model/language contract is valid but not inherited from this path. Explicitly require GigaAM's operation snapshot to override that mutable language argument, or pass a captured value through its path. Include a mid-recording language-setting change in the snapshot test. Do not change Whisper/Parakeet behavior under this feature.

## Limits

No model binaries downloaded, no opset/tensor ABI inspected from binaries, no Windows execution, numerical comparison or performance test performed. Those are not inferred from metadata. Current upstream default feature extractor differs from some published export configuration, supporting the existing rule to pair preprocessing with the exact export rather than copy current upstream defaults.

## Targeted recheck

Verdict: PASS. Both findings are resolved in the architecture text. AD-5 now places strict <=400,000-sample enforcement in the shared internal GigaAM segmentation adapter, after any existing splitting; AD-4 explicitly requires the captured language rather than the live worker's mutable setting. The verification contract includes both the per-part size limit and post-snapshot language changes. This confirms document closure only; implementation and native acceptance remain future work.

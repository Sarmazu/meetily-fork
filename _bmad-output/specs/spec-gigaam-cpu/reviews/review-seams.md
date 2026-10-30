# Adversarial seams review — GigaAM CPU

Verdict: PASS WITH FIXES. No critical/high finding established. Two medium seams should be tightened before independent implementation; no product-code changes made.

## Medium — splitting ownership and output aggregation are not fixed

AD-5 (line 67) requires segments <=25 seconds and reuse of the existing split mechanism but does not assign splitting to the caller or provider, nor define aggregation for a live chunk that splits into multiple segments. A provider implementer can internally split and concatenate into its single TranscriptResult while a recording implementer splits externally and emits one timed result per subsegment. Both satisfy the duration rule yet produce different segment granularity/timestamps and potentially different boundary handling. Existing file paths split upstream (`audio/import.rs` around 525 and `audio/retranscription.rs` around 320), while the recording worker computes one timestamp/duration from each original chunk (`audio/transcription/worker.rs` around 153).

Fix: assign splitting and original-offset preservation to a common caller-side GigaAM segmentation adapter reused by all three paths; a provider call consumes one bounded segment and returns one result. Assign existing operation code ownership of ordering and persistence. This does not require publishing every inner type now.

## Medium — the privacy rule requires edits in shared wrappers

AD-6 requires logs without transcript text. A new GigaAM module can obey this perfectly but its existing recording wrapper still logs full results (`worker.rs` around 169 and 174), and import/retranscription wrappers log text previews (`import.rs` around 598, `retranscription.rs` around 388). The current structural seed names providers/readiness but does not call out these shared wrappers, so independently developed native/UI work can leave the rule unenforced despite a clean provider.

Fix: explicitly bind shared operation logging to AD-6 and require GigaAM-path logs to omit transcript content end-to-end, including failure/debug messages. Keep existing Whisper/Parakeet behavior outside this scope as required, or separately authorize a general privacy fix. Add a synthetic marker assertion covering shared wrappers.

## Low — None confidence adaptation into legacy file paths

AD-5 correctly requires `confidence=None`, whereas existing file loops expect `(text, f32)` and sum confidence. Implementers can independently invent numeric substitutes. Do not invent a confidence score for GigaAM; omit its aggregation/display or represent it as unavailable while retaining existing persistence schema. Set the exact compatibility adapter in the technical task before coding.

## Other reviewed seams

Operation-bound handles, per-segment RNNT state, immutable package identity, publication-before-Available, active-file protection, provider routing before Whisper defaults, and revision/ABI gating are directionally sufficient. Detailed command/event shapes are explicitly gated to technical specification before coding and are not a separate blocker at feature altitude.

## Verification after revision

PASS. Revised AD-5 assigns a single internal GigaAM segmentation adapter, prohibits independent caller-side GigaAM splitters, enforces 400,000 samples before ONNX, and aggregates internal parts into the original input segment with its external timestamps. This closes the ownership seam; the chosen internal-owner solution is equivalent in consistency effect to the initially suggested caller-side owner. Revised AD-6 explicitly covers shared worker/import/retranscription logging and synthetic-marker verification, closing the privacy integration seam. Confidence remains unavailable through adapters; numeric legacy fallbacks cannot be presented as measured GigaAM confidence. Exact display/storage compatibility is appropriately gated to technical tasks. No remaining finding from this review blocks the feature-level spine. This verification is a document review, not a runtime test.

Conduct a review of CONTENT.
Look for what's missing, not only what's wrong.
Compute your finding floor N from the diff file's size: N = min(floor(sqrt(kB) + 1), 10), where kB is the file's size in kilobytes. State the arithmetic in one line, then find at least N issues to fix or improve.
Output a Markdown list of findings only — no severity, priority, or ranking.
If the content is empty, stop and say so.
If you have zero findings, re-check and keep thinking; do not stop with an empty list.

CONTENT: the unified diff below (22351 UTF-8 bytes).

Do not invoke any skill, and do not spawn subagents of your own — you are the reviewer. Return your findings as text in your final message; do not route them through any findings-reporting tool the host may offer.

```diff
diff --git a/frontend/src-tauri/src/gigaam_engine/preprocess.rs b/frontend/src-tauri/src/gigaam_engine/preprocess.rs
new file mode 100755
index 0000000..0f98c70
--- /dev/null
+++ b/frontend/src-tauri/src/gigaam_engine/preprocess.rs
@@ -0,0 +1,100 @@
+//! Pinned GigaAM v3 unpadded log-mel frontend (mono PCM f32 / 16 kHz).
+use realfft::RealFftPlanner;
+
+pub(crate) const WINDOW: usize = 320;
+pub(crate) const LIMIT: usize = 400_000;
+
+fn bf16(value: f64) -> f32 {
+    let bits = (value as f32).to_bits();
+    f32::from_bits(bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) & 0xffff0000)
+}
+
+pub(crate) fn features(audio: &[f32]) -> Result<(Vec<f32>, usize), String> {
+    if !(WINDOW..=LIMIT).contains(&audio.len()) || audio.iter().any(|x| !x.is_finite()) {
+        return Err("GigaAM requires 320..400000 finite PCM samples per inference".into());
+    }
+    let frames = (audio.len() - WINDOW) / 160 + 1;
+    let window: Vec<f32> = (0..WINDOW)
+        .map(|i| bf16(0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / WINDOW as f64).cos()))
+        .collect();
+    let max_mel = 2595.0 * (1.0_f64 + 8000.0 / 700.0).log10();
+    let points: Vec<f64> = (0..66)
+        .map(|i| 700.0 * (10.0_f64.powf(max_mel * i as f64 / 65.0 / 2595.0) - 1.0))
+        .collect();
+    let filters: Vec<Vec<f32>> = (0..64)
+        .map(|m| {
+            (0..161)
+                .map(|k| {
+                    let hz = k as f64 * 50.0;
+                    bf16(
+                        ((hz - points[m]) / (points[m + 1] - points[m]))
+                            .min((points[m + 2] - hz) / (points[m + 2] - points[m + 1]))
+                            .max(0.0),
+                    )
+                })
+                .collect()
+        })
+        .collect();
+    let fft = RealFftPlanner::<f64>::new().plan_fft_forward(WINDOW);
+    let mut input = fft.make_input_vec();
+    let mut spectrum = fft.make_output_vec();
+    let mut scratch = fft.make_scratch_vec();
+    let mut output = vec![0.0; 64 * frames];
+    for t in 0..frames {
+        for i in 0..WINDOW {
+            input[i] = (audio[t * 160 + i] * window[i]) as f64;
+        }
+        fft.process_with_scratch(&mut input, &mut spectrum, &mut scratch)
+            .map_err(|e| e.to_string())?;
+        let power: Vec<f32> = spectrum
+            .iter()
+            .map(|z| {
+                let magnitude = z.norm() as f32;
+                magnitude * magnitude
+            })
+            .collect();
+        for m in 0..64 {
+            let mel: f32 = power.iter().zip(&filters[m]).map(|(p, w)| p * w).sum();
+            output[m * frames + t] = mel.clamp(1e-9, 1e9).ln();
+        }
+    }
+    Ok((output, frames))
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    #[test]
+    fn reference_features_and_unpadded_lengths() {
+        for (n, t) in [(320, 1), (479, 1), (480, 2)] {
+            let (f, frames) = features(&vec![0.; n]).unwrap();
+            assert_eq!(frames, t);
+            assert!(f.iter().all(|x| *x == 1e-9_f32.ln()));
+        }
+        let input: Vec<f32> = (0..480)
+            .map(|i| {
+                (0.25 * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 16000.0).sin()) as f32
+            })
+            .collect();
+        let reference: serde_json::Value = serde_json::from_str(include_str!(
+            "../../../../tools/gigaam-model-contract/reference-features.json"
+        ))
+        .unwrap();
+        let (actual, frames) = features(&input).unwrap();
+        assert_eq!(frames, 2);
+        for m in 0..64 {
+            for t in 0..2 {
+                let expected = reference["features"]["values"][0][m][t].as_f64().unwrap() as f32;
+                assert!(
+                    (actual[m * 2 + t] - expected).abs() <= 1e-4 + 1e-4 * expected.abs(),
+                    "mel {m} frame {t}: {} != {expected}",
+                    actual[m * 2 + t]
+                );
+            }
+        }
+        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
+            assert!(features(&vec![value; 320]).is_err());
+        }
+        assert!(features(&[0.; 319]).is_err());
+    }
+}
diff --git a/tmp/meetily-gigaam-story13-baseline/frontend/src-tauri/src/gigaam_engine/mod.rs b/frontend/src-tauri/src/gigaam_engine/mod.rs
index d0f3914..51994ab 100644
--- a/tmp/meetily-gigaam-story13-baseline/frontend/src-tauri/src/gigaam_engine/mod.rs
+++ b/frontend/src-tauri/src/gigaam_engine/mod.rs
@@ -1,3 +1,6 @@
 pub mod catalog;
 pub mod commands;
 pub mod downloads;
+pub mod ctc;
+mod preprocess;
+mod segments;
diff --git a/tmp/meetily-gigaam-story13-baseline/frontend/src-tauri/src/gigaam_engine/downloads.rs b/frontend/src-tauri/src/gigaam_engine/downloads.rs
index fdea31d..bb89027 100644
--- a/tmp/meetily-gigaam-story13-baseline/frontend/src-tauri/src/gigaam_engine/downloads.rs
+++ b/frontend/src-tauri/src/gigaam_engine/downloads.rs
@@ -66,7 +66,7 @@ fn cancelled(cancel: Option<&CancellationToken>) -> Result<(), String> {
         Ok(())
     }
 }
-async fn verify(dir: &Path, package: &Package) -> Result<(), String> {
+pub(crate) async fn verify(dir: &Path, package: &Package) -> Result<(), String> {
     verify_inner(dir, package, None, || {}).await
 }
 async fn verify_inner(
diff --git a/frontend/src-tauri/src/gigaam_engine/segments.rs b/frontend/src-tauri/src/gigaam_engine/segments.rs
new file mode 100755
index 0000000..85c69a6
--- /dev/null
+++ b/frontend/src-tauri/src/gigaam_engine/segments.rs
@@ -0,0 +1,70 @@
+//! The only GigaAM-specific splitter. Ranges partition the input exactly.
+use super::preprocess::{LIMIT, WINDOW};
+use std::ops::Range;
+
+pub(crate) fn split(audio: &[f32]) -> Vec<Range<usize>> {
+    let mut ranges = Vec::new();
+    let mut start = 0;
+    while audio.len() - start > LIMIT {
+        let latest = start + LIMIT;
+        let earliest = latest - 48_000;
+        // Compare 20 ms local energy windows, taking the latest equal minimum.
+        let mut boundary = latest;
+        let mut minimum = f64::INFINITY;
+        let mut energy: f64 = audio[earliest - WINDOW..earliest]
+            .iter()
+            .map(|x| (*x as f64).powi(2))
+            .sum();
+        for end in earliest..=latest {
+            if energy <= minimum {
+                minimum = energy;
+                boundary = end;
+            }
+            if end < latest {
+                energy += (audio[end] as f64).powi(2) - (audio[end - WINDOW] as f64).powi(2);
+            }
+        }
+        if audio.len() - boundary < WINDOW {
+            boundary = audio.len() - WINDOW;
+        }
+        ranges.push(start..boundary);
+        start = boundary;
+    }
+    if start < audio.len() {
+        ranges.push(start..audio.len());
+    }
+    ranges
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    #[test]
+    fn bounded_partition_without_short_tail() {
+        for n in [0, 319, 320, 400000, 400001, 800001, 1200001] {
+            let ranges = split(&vec![0.; n]);
+            let mut end = 0;
+            for r in &ranges {
+                assert_eq!(r.start, end);
+                assert!(r.len() <= LIMIT);
+                if n > LIMIT {
+                    assert!(r.len() >= WINDOW);
+                }
+                end = r.end;
+            }
+            assert_eq!(end, n);
+            if n == 400000 {
+                assert_eq!(ranges.len(), 1);
+            }
+            if n == 400001 {
+                assert_eq!(ranges, vec![0..399681, 399681..400001]);
+            }
+        }
+    }
+    #[test]
+    fn chooses_low_energy_in_last_three_seconds() {
+        let mut audio = vec![1.; 500000];
+        audio[370000 - 320..370000].fill(0.);
+        assert_eq!(split(&audio)[0].end, 370000);
+    }
+}
diff --git a/frontend/src-tauri/src/gigaam_engine/ctc.rs b/frontend/src-tauri/src/gigaam_engine/ctc.rs
new file mode 100755
index 0000000..9c4c6c7
--- /dev/null
+++ b/frontend/src-tauri/src/gigaam_engine/ctc.rs
@@ -0,0 +1,359 @@
+//! Operation-owned FP32 CPU CTC engine; no application routing or global session.
+use super::{catalog, downloads, preprocess, segments};
+use crate::audio::transcription::provider::TranscriptResult;
+use ort::{
+    execution_providers::CPUExecutionProvider,
+    session::Session,
+    tensor::TensorElementType,
+    value::{Tensor, ValueType},
+};
+use std::path::Path;
+
+pub struct CtcEngine {
+    session: Session,
+    vocabulary: Vec<String>,
+    blank: usize,
+}
+
+fn vocabulary(source: &str, size: usize, blank: usize) -> Result<Vec<String>, String> {
+    let mut tokens = vec![None; size];
+    for line in source.lines() {
+        let (token, id) = line
+            .rsplit_once(' ')
+            .ok_or("Invalid GigaAM vocabulary line")?;
+        let id: usize = id.parse().map_err(|_| "Invalid GigaAM vocabulary ID")?;
+        if id >= size || tokens[id].is_some() || token.is_empty() {
+            return Err("Invalid or duplicate GigaAM vocabulary entry".into());
+        }
+        tokens[id] = Some(token.replace('\u{2581}', " "));
+    }
+    let tokens: Vec<String> = tokens
+        .into_iter()
+        .collect::<Option<_>>()
+        .ok_or("Incomplete GigaAM vocabulary")?;
+    if tokens[blank] != "<blk>" || tokens.iter().filter(|t| *t == "<blk>").count() != 1 {
+        return Err("Invalid GigaAM blank token".into());
+    }
+    if size == 257 && tokens[0] != "<unk>" {
+        return Err("Invalid E2E unknown token".into());
+    }
+    Ok(tokens)
+}
+
+fn tensor_abi(value: &ValueType, ty: TensorElementType, dimensions: &[i64]) -> bool {
+    match value {
+        ValueType::Tensor {
+            ty: actual, shape, ..
+        } => {
+            *actual == ty
+                && shape.len() == dimensions.len()
+                && shape.iter().zip(dimensions).all(|(actual, expected)| {
+                    if *expected < 0 {
+                        *actual < 0 || *actual == 1
+                    } else {
+                        *actual == *expected
+                    }
+                })
+        }
+        _ => false,
+    }
+}
+
+fn validate_abi(session: &Session, size: usize) -> Result<(), String> {
+    if session.inputs.len() != 2 || session.outputs.len() != 1 {
+        return Err("Invalid GigaAM CTC tensor count".into());
+    }
+    for (name, ty, shape) in [
+        ("features", TensorElementType::Float32, vec![-1, 64, -1]),
+        ("feature_lengths", TensorElementType::Int64, vec![-1]),
+    ] {
+        let input = session
+            .inputs
+            .iter()
+            .find(|i| i.name == name)
+            .ok_or("Missing GigaAM input")?;
+        if !tensor_abi(&input.input_type, ty, &shape) {
+            return Err(format!("Invalid GigaAM input ABI: {name}"));
+        }
+    }
+    let output = &session.outputs[0];
+    if output.name != "log_probs"
+        || !tensor_abi(
+            &output.output_type,
+            TensorElementType::Float32,
+            &[-1, -1, size as i64],
+        )
+    {
+        return Err("Invalid GigaAM CTC output ABI".into());
+    }
+    Ok(())
+}
+
+fn detokenize(ids: impl IntoIterator<Item = usize>, vocab: &[String], blank: usize) -> String {
+    let mut previous = None;
+    let mut text = String::new();
+    for id in ids {
+        if previous != Some(id) && id != blank {
+            text.push_str(&vocab[id]);
+        }
+        previous = Some(id);
+    }
+    // Exact pinned Python expression and replacement; default Unicode boundaries.
+    let cleanup = regex::Regex::new(r"\A\s|\s\B|(\s)\b").expect("constant regex");
+    cleanup
+        .replace_all(
+            &text,
+            |captures: &regex::Captures<'_>| if captures.get(1).is_some() { " " } else { "" },
+        )
+        .into_owned()
+}
+
+fn decode_logits(
+    logits: ndarray::ArrayViewD<'_, f32>,
+    frames: usize,
+    vocabulary: &[String],
+    blank: usize,
+) -> Result<String, String> {
+    let shape = logits.shape();
+    let valid = frames.div_ceil(4);
+    if frames == 0
+        || shape.len() != 3
+        || shape[0] != 1
+        || shape[1] < valid
+        || shape[2] != vocabulary.len()
+        || logits.iter().any(|x| !x.is_finite())
+    {
+        return Err("Invalid GigaAM runtime logits shape/length/values".into());
+    }
+    let ids = (0..valid).map(|t| {
+        let mut best = 0;
+        for id in 1..vocabulary.len() {
+            if logits[[0, t, id]] > logits[[0, t, best]] {
+                best = id;
+            }
+        }
+        best
+    });
+    Ok(detokenize(ids, vocabulary, blank))
+}
+
+impl CtcEngine {
+    pub async fn load(model_id: &str, package_dir: impl AsRef<Path>) -> Result<Self, String> {
+        let (size, blank, vocab) = match model_id {
+            "v3_ctc" => (34, 33, "v3_vocab.txt"),
+            "v3_e2e_ctc" => (257, 256, "v3_e2e_ctc_vocab.txt"),
+            _ => return Err("Unsupported GigaAM CTC model ID".into()),
+        };
+        let package = catalog::catalog()?
+            .into_iter()
+            .find(|p| p.name == model_id)
+            .ok_or("Missing GigaAM package contract")?;
+        let dir = package_dir.as_ref();
+        downloads::verify(dir, &package).await?;
+        let vocabulary = vocabulary(
+            &tokio::fs::read_to_string(dir.join(vocab))
+                .await
+                .map_err(|e| e.to_string())?,
+            size,
+            blank,
+        )?;
+        crate::ensure_onnx_runtime_available().map_err(|e| e.to_string())?;
+        let session = Session::builder()
+            .map_err(|e| e.to_string())?
+            .with_execution_providers([CPUExecutionProvider::default().build().error_on_failure()])
+            .map_err(|e| e.to_string())?
+            .commit_from_file(dir.join(format!("{model_id}.onnx")))
+            .map_err(|e| e.to_string())?;
+        validate_abi(&session, size)?;
+        Ok(Self {
+            session,
+            vocabulary,
+            blank,
+        })
+    }
+
+    pub fn transcribe(&mut self, audio: &[f32]) -> Result<TranscriptResult, String> {
+        if audio.iter().any(|x| !x.is_finite()) {
+            return Err("Non-finite GigaAM PCM input".into());
+        }
+        let mut texts = Vec::new();
+        for range in segments::split(audio) {
+            let part = &audio[range];
+            if part.len() < preprocess::WINDOW || part.iter().all(|x| *x == 0.) {
+                continue;
+            }
+            let (features, frames) = preprocess::features(part)?;
+            let input =
+                Tensor::from_array(([1usize, 64, frames], features)).map_err(|e| e.to_string())?;
+            let length =
+                Tensor::from_array(([1usize], vec![frames as i64])).map_err(|e| e.to_string())?;
+            let outputs = self
+                .session
+                .run(ort::inputs!["features"=>input,"feature_lengths"=>length])
+                .map_err(|e| e.to_string())?;
+            let logits = outputs
+                .get("log_probs")
+                .ok_or("Missing GigaAM logits")?
+                .try_extract_array::<f32>()
+                .map_err(|e| e.to_string())?;
+            let text = decode_logits(logits, frames, &self.vocabulary, self.blank)?;
+            if !text.is_empty() {
+                texts.push(text);
+            }
+        }
+        Ok(TranscriptResult {
+            text: texts.join(" "),
+            confidence: None,
+            is_partial: false,
+        })
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    #[test]
+    fn collapse_original_frames_and_preserve_e2e_spacing() {
+        let vocab = vec![" ".into(), "а".into(), "б".into(), "<blk>".into()];
+        assert_eq!(detokenize([3, 1, 1, 3, 1, 0, 2, 2, 3], &vocab, 3), "аа б");
+        assert_eq!(detokenize([3, 3], &vocab, 3), "");
+        let vocab = vec![
+            " Сегодня".into(),
+            " мы".into(),
+            " проверяем".into(),
+            " .".into(),
+            "<blk>".into(),
+        ];
+        assert_eq!(detokenize([0, 1, 2, 3], &vocab, 4), "Сегодня мы проверяем.");
+    }
+    #[test]
+    fn tensor_metadata_rejects_invalid_abi() {
+        use ort::tensor::{Shape, SymbolicDimensions};
+        let tensor = |ty, shape: &[i64]| ValueType::Tensor {
+            ty,
+            shape: Shape::new(shape.iter().copied()),
+            dimension_symbols: SymbolicDimensions::new(vec![String::new(); shape.len()]),
+        };
+        assert!(tensor_abi(
+            &tensor(TensorElementType::Float32, &[-1, 64, -1]),
+            TensorElementType::Float32,
+            &[-1, 64, -1]
+        ));
+        for (ty, shape) in [
+            (TensorElementType::Int64, vec![-1, 64, -1]),
+            (TensorElementType::Float32, vec![-1, 63, -1]),
+            (TensorElementType::Float32, vec![-1, 64]),
+            (TensorElementType::Float32, vec![2, 64, -1]),
+        ] {
+            assert!(!tensor_abi(
+                &tensor(ty, &shape),
+                TensorElementType::Float32,
+                &[-1, 64, -1]
+            ));
+        }
+        assert!(!tensor_abi(
+            &tensor(TensorElementType::Float32, &[-1, -1, 34]),
+            TensorElementType::Float32,
+            &[-1, -1, 257]
+        ));
+    }
+    #[test]
+    fn rejects_invalid_vocab() {
+        assert!(vocabulary("а 0\n<blk> 1\n", 2, 1).is_ok());
+        for bad in [
+            "а 0\nб 0\n<blk> 1",
+            "а 0",
+            "а 0\nб 1",
+            "а 2\n<blk> 1",
+            "а nope",
+        ] {
+            assert!(vocabulary(bad, 2, 1).is_err());
+        }
+    }
+    #[test]
+    fn runtime_logits_validate_shape_length_finiteness_and_mask_padding() {
+        let vocab = vec!["а".into(), "<blk>".into()];
+        let values = ndarray::Array::from_shape_vec((1, 2, 2), vec![0., 1., 10., 0.])
+            .unwrap()
+            .into_dyn();
+        assert_eq!(decode_logits(values.view(), 1, &vocab, 1).unwrap(), "");
+        assert_eq!(decode_logits(values.view(), 5, &vocab, 1).unwrap(), "а");
+        assert!(decode_logits(values.view(), 9, &vocab, 1).is_err());
+        for shape in [vec![1, 2], vec![2, 2, 2], vec![1, 2, 3]] {
+            let values = ndarray::ArrayD::zeros(shape);
+            assert!(decode_logits(values.view(), 1, &vocab, 1).is_err());
+        }
+        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
+            let values = ndarray::Array::from_shape_vec((1, 1, 2), vec![value, 0.])
+                .unwrap()
+                .into_dyn();
+            assert!(decode_logits(values.view(), 1, &vocab, 1).is_err());
+        }
+    }
+    #[tokio::test]
+    async fn invalid_package_and_id_fail_before_runtime() {
+        assert!(CtcEngine::load("v3_rnnt", Path::new("missing"))
+            .await
+            .is_err());
+        let dir = std::env::temp_dir().join(format!("gigaam-ctc-invalid-{}", uuid::Uuid::new_v4()));
+        std::fs::create_dir(&dir).unwrap();
+        std::fs::write(dir.join("v3_ctc.onnx"), b"invalid").unwrap();
+        assert!(CtcEngine::load("v3_ctc", &dir).await.is_err());
+        std::fs::remove_dir_all(dir).unwrap();
+    }
+    #[tokio::test]
+    #[ignore = "requires pinned Windows ORT 1.22.0, verified packages and synthetic WAV"]
+    async fn model_backed_cpu_smoke() {
+        let runtime = std::env::var("GIGAAM_TEST_ORT_DLL").expect("GIGAAM_TEST_ORT_DLL");
+        ort::init_from(runtime)
+            .with_execution_providers([CPUExecutionProvider::default().build().error_on_failure()])
+            .commit()
+            .unwrap();
+        let bytes =
+            std::fs::read(std::env::var("GIGAAM_TEST_WAV").expect("GIGAAM_TEST_WAV")).unwrap();
+        use sha2::Digest;
+        assert_eq!(
+            format!("{:x}", sha2::Sha256::digest(&bytes)),
+            "034a71efec3f2eb19b44093ba8f029a5d9e90db64ef4a2d14a5d4f08f8731659"
+        );
+        let mut offset = 12;
+        let mut data = None;
+        let mut format = None;
+        assert_eq!(&bytes[..4], b"RIFF");
+        assert_eq!(&bytes[8..12], b"WAVE");
+        while offset + 8 <= bytes.len() {
+            let size =
+                u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
+            let chunk = &bytes[offset + 8..offset + 8 + size];
+            if &bytes[offset..offset + 4] == b"fmt " {
+                format = Some(chunk);
+            }
+            if &bytes[offset..offset + 4] == b"data" {
+                data = Some(chunk);
+            }
+            offset += 8 + size + (size % 2);
+        }
+        let format = format.unwrap();
+        assert_eq!(u16::from_le_bytes(format[0..2].try_into().unwrap()), 1);
+        assert_eq!(u16::from_le_bytes(format[2..4].try_into().unwrap()), 1);
+        assert_eq!(u32::from_le_bytes(format[4..8].try_into().unwrap()), 16000);
+        assert_eq!(u16::from_le_bytes(format[14..16].try_into().unwrap()), 16);
+        let audio: Vec<f32> = data
+            .unwrap()
+            .chunks_exact(2)
+            .map(|x| i16::from_le_bytes([x[0], x[1]]) as f32 / 32768.)
+            .collect();
+        assert_eq!(audio.len(), 141120);
+        for (id,var,expected) in [
+            ("v3_ctc","GIGAAM_TEST_CTC_DIR","сегодня мы проверяем распознавание русской речи все четыре модели работают локально на компьютере"),
+            ("v3_e2e_ctc","GIGAAM_TEST_E2E_CTC_DIR","Сегодня мы проверяем распознавание русской речи. Все четыре модели работают локально на компьютере.")
+        ] {
+            let mut engine=CtcEngine::load(id,std::env::var(var).expect(var)).await.unwrap();
+            let result=engine.transcribe(&audio).unwrap();
+            println!("{id} CPU text: {}",result.text);
+            assert_eq!(result.text,expected); assert_eq!(result.confidence,None); assert!(!result.is_partial);
+            for n in [0,319,320,479,480,400000,400001] {assert!(engine.transcribe(&vec![0.;n]).unwrap().text.is_empty());}
+            for value in [f32::NAN,f32::INFINITY,f32::NEG_INFINITY] {assert!(engine.transcribe(&[value;320]).is_err());}
+        }
+    }
+}

```

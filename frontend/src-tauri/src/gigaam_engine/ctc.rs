//! Operation-owned FP32 CPU CTC engine; no application routing or global session.
use super::common::{aggregate, cleanup_text, tensor_abi, vocabulary};
#[cfg(test)]
use super::segments;
use super::{catalog, downloads, preprocess};
use crate::audio::transcription::provider::TranscriptResult;
use ort::{
    execution_providers::CPUExecutionProvider, session::Session, tensor::TensorElementType,
    value::Tensor,
};
use std::path::Path;

pub struct CtcEngine {
    session: Session,
    vocabulary: Vec<String>,
    blank: usize,
}

fn validate_abi(session: &Session, size: usize) -> Result<(), String> {
    if session.inputs.len() != 2 || session.outputs.len() != 1 {
        return Err("Invalid GigaAM CTC tensor count".into());
    }
    for (name, ty, shape) in [
        ("features", TensorElementType::Float32, vec![-1, 64, -1]),
        ("feature_lengths", TensorElementType::Int64, vec![-1]),
    ] {
        let input = session
            .inputs
            .iter()
            .find(|i| i.name == name)
            .ok_or("Missing GigaAM input")?;
        if !tensor_abi(&input.input_type, ty, &shape) {
            return Err(format!("Invalid GigaAM input ABI: {name}"));
        }
    }
    let output = &session.outputs[0];
    if output.name != "log_probs"
        || !tensor_abi(
            &output.output_type,
            TensorElementType::Float32,
            &[-1, -1, size as i64],
        )
    {
        return Err("Invalid GigaAM CTC output ABI".into());
    }
    Ok(())
}

fn detokenize(ids: impl IntoIterator<Item = usize>, vocab: &[String], blank: usize) -> String {
    let mut previous = None;
    let mut text = String::new();
    for id in ids {
        if previous != Some(id) && id != blank {
            text.push_str(&vocab[id]);
        }
        previous = Some(id);
    }
    cleanup_text(&text)
}

fn decode_logits(
    logits: ndarray::ArrayViewD<'_, f32>,
    frames: usize,
    vocabulary: &[String],
    blank: usize,
) -> Result<String, String> {
    let shape = logits.shape();
    let valid = frames.div_ceil(4);
    if frames == 0
        || shape.len() != 3
        || shape[0] != 1
        || shape[1] < valid
        || shape[2] != vocabulary.len()
        || logits.iter().any(|x| !x.is_finite())
    {
        return Err("Invalid GigaAM runtime logits shape/length/values".into());
    }
    let ids = (0..valid).map(|t| {
        let mut best = 0;
        for id in 1..vocabulary.len() {
            if logits[[0, t, id]] > logits[[0, t, best]] {
                best = id;
            }
        }
        best
    });
    Ok(detokenize(ids, vocabulary, blank))
}

impl CtcEngine {
    pub async fn load(model_id: &str, package_dir: impl AsRef<Path>) -> Result<Self, String> {
        let (size, blank, vocab) = match model_id {
            "v3_ctc" => (34, 33, "v3_vocab.txt"),
            "v3_e2e_ctc" => (257, 256, "v3_e2e_ctc_vocab.txt"),
            _ => return Err("Unsupported GigaAM CTC model ID".into()),
        };
        let package = catalog::catalog()?
            .into_iter()
            .find(|p| p.name == model_id)
            .ok_or("Missing GigaAM package contract")?;
        let dir = package_dir.as_ref();
        downloads::verify(dir, &package).await?;
        let vocabulary = vocabulary(
            &tokio::fs::read_to_string(dir.join(vocab))
                .await
                .map_err(|e| e.to_string())?,
            size,
            blank,
        )?;
        crate::ensure_onnx_runtime_available().map_err(|e| e.to_string())?;
        let session = Session::builder()
            .map_err(|e| e.to_string())?
            .with_execution_providers([CPUExecutionProvider::default().build().error_on_failure()])
            .map_err(|e| e.to_string())?
            .commit_from_file(dir.join(format!("{model_id}.onnx")))
            .map_err(|e| e.to_string())?;
        validate_abi(&session, size)?;
        Ok(Self {
            session,
            vocabulary,
            blank,
        })
    }

    pub fn transcribe(&mut self, audio: &[f32]) -> Result<TranscriptResult, String> {
        aggregate(audio, |part| self.transcribe_part(part))
    }

    fn transcribe_part(&mut self, part: &[f32]) -> Result<String, String> {
        let (features, frames) = preprocess::features(part)?;
        let input =
            Tensor::from_array(([1usize, 64, frames], features)).map_err(|e| e.to_string())?;
        let length =
            Tensor::from_array(([1usize], vec![frames as i64])).map_err(|e| e.to_string())?;
        let outputs = self
            .session
            .run(ort::inputs!["features"=>input,"feature_lengths"=>length])
            .map_err(|e| e.to_string())?;
        let logits = outputs
            .get("log_probs")
            .ok_or("Missing GigaAM logits")?
            .try_extract_array::<f32>()
            .map_err(|e| e.to_string())?;
        decode_logits(logits, frames, &self.vocabulary, self.blank)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ort::value::ValueType;
    #[test]
    fn collapse_original_frames_and_preserve_e2e_spacing() {
        let vocab = vec![" ".into(), "а".into(), "б".into(), "<blk>".into()];
        assert_eq!(detokenize([3, 1, 1, 3, 1, 0, 2, 2, 3], &vocab, 3), "аа б");
        assert_eq!(detokenize([3, 3], &vocab, 3), "");
        let vocab = vec![
            " Сегодня".into(),
            " мы".into(),
            " проверяем".into(),
            " .".into(),
            "<blk>".into(),
        ];
        assert_eq!(detokenize([0, 1, 2, 3], &vocab, 4), "Сегодня мы проверяем.");
    }
    #[test]
    fn allowed_vocabulary_spacing_parity() {
        // Expected strings from the pinned Python regex for the allowed alphabet.
        for (source, expected) in [
            ("  Привет  мир !  123 . ", " Привет мир! 123."),
            (" A 1 , B 2 ? ", "A 1, B 2?"),
            (" 42 : 7 ; ёж ", "42: 7; ёж"),
        ] {
            let vocab = vec![source.to_owned(), "<blk>".into()];
            assert_eq!(detokenize([0], &vocab, 1), expected);
        }
    }
    #[test]
    fn aggregation_preserves_order_skips_empty_and_propagates_errors() {
        let audio: Vec<f32> = (0..1_000_000)
            .map(|i| 1.0 + i as f32 / 1_000_000.)
            .collect();
        let ranges = segments::split(&audio);
        assert!(ranges.len() >= 3);
        let mut calls = 0;
        let result = aggregate(&audio, |part| {
            assert_eq!(part, &audio[ranges[calls].clone()]);
            let text = match calls {
                0 => "first",
                1 => "",
                _ => "last",
            };
            calls += 1;
            Ok(text.into())
        })
        .unwrap();
        assert_eq!(calls, ranges.len());
        assert_eq!(result.text, "first last");
        assert_eq!(result.confidence, None);
        assert!(!result.is_partial);
        let mut calls = 0;
        let failure = aggregate(&audio, |_| {
            calls += 1;
            if calls == 2 {
                Err("inference failed".into())
            } else {
                Ok("first".into())
            }
        });
        assert_eq!(failure.unwrap_err(), "inference failed");
        assert_eq!(calls, 2);
        for input in [
            vec![0.; 400001],
            vec![0.1; 319],
            vec![f32::NAN; 320],
            vec![f32::INFINITY; 320],
        ] {
            let _ = aggregate(&input, |_| panic!("must reject or skip before inference"));
        }
    }
    #[test]
    fn tensor_metadata_rejects_invalid_abi() {
        use ort::tensor::{Shape, SymbolicDimensions};
        let tensor = |ty, shape: &[i64]| ValueType::Tensor {
            ty,
            shape: Shape::new(shape.iter().copied()),
            dimension_symbols: SymbolicDimensions::new(vec![String::new(); shape.len()]),
        };
        assert!(tensor_abi(
            &tensor(TensorElementType::Float32, &[-1, 64, -1]),
            TensorElementType::Float32,
            &[-1, 64, -1]
        ));
        for (ty, shape) in [
            (TensorElementType::Int64, vec![-1, 64, -1]),
            (TensorElementType::Float32, vec![-1, 63, -1]),
            (TensorElementType::Float32, vec![-1, 64]),
            (TensorElementType::Float32, vec![2, 64, -1]),
            (TensorElementType::Float32, vec![-1, 64, 1]),
            (TensorElementType::Float32, vec![1, 64, 1]),
        ] {
            assert!(!tensor_abi(
                &tensor(ty, &shape),
                TensorElementType::Float32,
                &[-1, 64, -1]
            ));
        }
        assert!(tensor_abi(
            &tensor(TensorElementType::Float32, &[1, 64, -1]),
            TensorElementType::Float32,
            &[-1, 64, -1]
        ));
        for shape in [[-1, 1, 257], [1, 1, 257]] {
            assert!(!tensor_abi(
                &tensor(TensorElementType::Float32, &shape),
                TensorElementType::Float32,
                &[-1, -1, 257]
            ));
        }
        assert!(!tensor_abi(
            &tensor(TensorElementType::Float32, &[-1, -1, 34]),
            TensorElementType::Float32,
            &[-1, -1, 257]
        ));
    }
    #[test]
    fn rejects_invalid_vocab() {
        assert!(vocabulary("а 0\n<blk> 1\n", 2, 1).is_ok());
        for bad in [
            "а 0\nб 0\n<blk> 1",
            "а 0",
            "а 0\nб 1",
            "а 2\n<blk> 1",
            "а nope",
        ] {
            assert!(vocabulary(bad, 2, 1).is_err());
        }
    }
    #[test]
    fn runtime_logits_validate_shape_length_finiteness_and_mask_padding() {
        let vocab = vec!["а".into(), "<blk>".into()];
        let values = ndarray::Array::from_shape_vec((1, 2, 2), vec![0., 1., 10., 0.])
            .unwrap()
            .into_dyn();
        assert_eq!(decode_logits(values.view(), 1, &vocab, 1).unwrap(), "");
        assert_eq!(decode_logits(values.view(), 5, &vocab, 1).unwrap(), "а");
        assert!(decode_logits(values.view(), 9, &vocab, 1).is_err());
        for shape in [vec![1, 2], vec![2, 2, 2], vec![1, 2, 3]] {
            let values = ndarray::ArrayD::zeros(shape);
            assert!(decode_logits(values.view(), 1, &vocab, 1).is_err());
        }
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let values = ndarray::Array::from_shape_vec((1, 1, 2), vec![value, 0.])
                .unwrap()
                .into_dyn();
            assert!(decode_logits(values.view(), 1, &vocab, 1).is_err());
        }
    }
    #[tokio::test]
    async fn invalid_package_and_id_fail_before_runtime() {
        assert!(CtcEngine::load("v3_rnnt", Path::new("missing"))
            .await
            .is_err());
        let dir = std::env::temp_dir().join(format!("gigaam-ctc-invalid-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("v3_ctc.onnx"), b"invalid").unwrap();
        assert!(CtcEngine::load("v3_ctc", &dir).await.is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[tokio::test]
    #[ignore = "requires pinned Windows ORT 1.22.0, verified packages and synthetic WAV"]
    async fn model_backed_cpu_smoke() {
        let runtime = std::env::var("GIGAAM_TEST_ORT_DLL").expect("GIGAAM_TEST_ORT_DLL");
        super::super::common::verify_test_runtime(&runtime).unwrap();
        ort::init_from(runtime)
            .with_execution_providers([CPUExecutionProvider::default().build().error_on_failure()])
            .commit()
            .unwrap();
        let bytes =
            std::fs::read(std::env::var("GIGAAM_TEST_WAV").expect("GIGAAM_TEST_WAV")).unwrap();
        use sha2::Digest;
        assert_eq!(
            format!("{:x}", sha2::Sha256::digest(&bytes)),
            "034a71efec3f2eb19b44093ba8f029a5d9e90db64ef4a2d14a5d4f08f8731659"
        );
        let mut offset = 12;
        let mut data = None;
        let mut format = None;
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        while offset + 8 <= bytes.len() {
            let size =
                u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
            let chunk = &bytes[offset + 8..offset + 8 + size];
            if &bytes[offset..offset + 4] == b"fmt " {
                format = Some(chunk);
            }
            if &bytes[offset..offset + 4] == b"data" {
                data = Some(chunk);
            }
            offset += 8 + size + (size % 2);
        }
        let format = format.unwrap();
        assert_eq!(u16::from_le_bytes(format[0..2].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(format[2..4].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(format[4..8].try_into().unwrap()), 16000);
        assert_eq!(u16::from_le_bytes(format[14..16].try_into().unwrap()), 16);
        let audio: Vec<f32> = data
            .unwrap()
            .chunks_exact(2)
            .map(|x| i16::from_le_bytes([x[0], x[1]]) as f32 / 32768.)
            .collect();
        assert_eq!(audio.len(), 141120);
        for (id,var,expected) in [
            ("v3_ctc","GIGAAM_TEST_CTC_DIR","сегодня мы проверяем распознавание русской речи все четыре модели работают локально на компьютере"),
            ("v3_e2e_ctc","GIGAAM_TEST_E2E_CTC_DIR","Сегодня мы проверяем распознавание русской речи. Все четыре модели работают локально на компьютере.")
        ] {
            let mut engine=CtcEngine::load(id,std::env::var(var).expect(var)).await.unwrap();
            let result=engine.transcribe(&audio).unwrap();
            println!("{id} CPU text: {}",result.text);
            assert_eq!(result.text,expected); assert_eq!(result.confidence,None); assert!(!result.is_partial);
            let mut long=audio.clone(); long.resize(400000,0.); long.extend_from_slice(&audio);
            assert_eq!(segments::split(&long),vec![0..400000,400000..long.len()]);
            let repeated=engine.transcribe(&long).unwrap();
            assert_eq!(repeated.text,format!("{expected} {expected}"));
            assert_eq!(repeated.confidence,None); assert!(!repeated.is_partial);
            println!("{id} CPU long repeated phrase verified");
            for n in [320,479,480] {
                let tone:Vec<f32>=(0..n).map(|i| (0.1*(2.0*std::f64::consts::PI*440.0*i as f64/16000.).sin()) as f32).collect();
                let result=engine.transcribe(&tone).unwrap();
                assert_eq!(result.confidence,None); assert!(!result.is_partial);
            }
            for n in [0,319,320,479,480,400000,400001] {assert!(engine.transcribe(&vec![0.;n]).unwrap().text.is_empty());}
            for value in [f32::NAN,f32::INFINITY,f32::NEG_INFINITY] {assert!(engine.transcribe(&[value;320]).is_err());}
        }
    }
}

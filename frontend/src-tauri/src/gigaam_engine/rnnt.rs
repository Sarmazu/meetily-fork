//! Operation-owned FP32 CPU RNNT. Recurrent state belongs to one segment only.
use super::{
    catalog,
    common::{aggregate, cleanup_text, tensor_abi, vocabulary},
    downloads, preprocess,
};
use crate::audio::transcription::provider::TranscriptResult;
use ort::{
    execution_providers::CPUExecutionProvider, session::Session, tensor::TensorElementType,
    value::Tensor,
};
use std::path::Path;

pub struct RnntEngine {
    encoder: Session,
    decoder: Session,
    joint: Session,
    vocabulary: Vec<String>,
    blank: usize,
}

fn validate_abi(
    session: &Session,
    inputs: &[(&str, TensorElementType, &[i64])],
    outputs: &[(&str, TensorElementType, &[i64])],
) -> Result<(), String> {
    if session.inputs.len() != inputs.len() || session.outputs.len() != outputs.len() {
        return Err("Invalid GigaAM RNNT tensor count".into());
    }
    for (name, ty, shape) in inputs {
        let input = session
            .inputs
            .iter()
            .find(|i| i.name == *name)
            .ok_or("Missing GigaAM RNNT input")?;
        if !tensor_abi(&input.input_type, *ty, shape) {
            return Err(format!("Invalid GigaAM RNNT input ABI: {name}"));
        }
    }
    for (name, ty, shape) in outputs {
        let output = session
            .outputs
            .iter()
            .find(|i| i.name == *name)
            .ok_or("Missing GigaAM RNNT output")?;
        if !tensor_abi(&output.output_type, *ty, shape) {
            return Err(format!("Invalid GigaAM RNNT output ABI: {name}"));
        }
    }
    Ok(())
}

fn checked_values(
    array: ndarray::ArrayViewD<'_, f32>,
    expected: &[usize],
) -> Result<Vec<f32>, String> {
    if array.shape() != expected || array.iter().any(|v| !v.is_finite()) {
        return Err("Invalid GigaAM RNNT runtime tensor shape/values".into());
    }
    Ok(array.iter().copied().collect())
}

fn encoded_frames(
    encoded: ndarray::ArrayViewD<'_, f32>,
    length: ndarray::ArrayViewD<'_, i32>,
) -> Result<usize, String> {
    if encoded.ndim() != 3
        || encoded.shape()[0] != 1
        || encoded.shape()[1] != 768
        || encoded.iter().any(|v| !v.is_finite())
        || length.shape() != [1]
    {
        return Err("Invalid GigaAM RNNT encoder output".into());
    }
    let n = length[[0]];
    if n < 0 || n as usize > encoded.shape()[2] {
        return Err("Invalid GigaAM RNNT encoded length".into());
    }
    Ok(n as usize)
}

struct Prediction {
    dec: Vec<f32>,
    h: Vec<f32>,
    c: Vec<f32>,
}

// Both fake tests and the real sessions use this exact state/cache loop.
fn greedy(
    frames: usize,
    size: usize,
    blank: usize,
    mut predict: impl FnMut(usize, &[f32], &[f32]) -> Result<Prediction, String>,
    mut joint: impl FnMut(usize, &[f32]) -> Result<Vec<f32>, String>,
) -> Result<Vec<usize>, String> {
    let (mut h, mut c) = (vec![0.; 320], vec![0.; 320]);
    let mut label = blank;
    let mut cache: Option<Prediction> = None;
    let mut ids = Vec::new();
    for t in 0..frames {
        for _ in 0..3 {
            if cache.is_none() {
                let p = predict(label, &h, &c)?;
                if [&p.dec, &p.h, &p.c]
                    .iter()
                    .any(|v| v.len() != 320 || v.iter().any(|x| !x.is_finite()))
                {
                    return Err("Invalid GigaAM RNNT prediction".into());
                }
                cache = Some(p);
            }
            let logits = joint(t, &cache.as_ref().unwrap().dec)?;
            if logits.len() != size || logits.iter().any(|x| !x.is_finite()) {
                return Err("Invalid GigaAM RNNT joint logits".into());
            }
            let mut best = 0;
            for i in 1..size {
                if logits[i] > logits[best] {
                    best = i;
                }
            }
            if best == blank {
                break;
            }
            ids.push(best);
            label = best;
            let p = cache.take().unwrap();
            h = p.h;
            c = p.c;
        }
    }
    Ok(ids)
}

impl RnntEngine {
    pub async fn load(model_id: &str, package_dir: impl AsRef<Path>) -> Result<Self, String> {
        let (size, blank, vocab) = match model_id {
            "v3_rnnt" => (34, 33, "v3_vocab.txt"),
            "v3_e2e_rnnt" => (1025, 1024, "v3_e2e_rnnt_vocab.txt"),
            _ => return Err("Unsupported GigaAM RNNT model ID".into()),
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
        let load = |role| {
            Session::builder()
                .map_err(|e| e.to_string())?
                .with_execution_providers([CPUExecutionProvider::default()
                    .build()
                    .error_on_failure()])
                .map_err(|e| e.to_string())?
                .commit_from_file(dir.join(format!("{model_id}_{role}.onnx")))
                .map_err(|e| e.to_string())
        };
        let encoder = load("encoder")?;
        let decoder = load("decoder")?;
        let joint = load("joint")?;
        use TensorElementType::{Float32 as F, Int32 as L, Int64 as I};
        validate_abi(
            &encoder,
            &[("audio_signal", F, &[-1, 64, -1]), ("length", I, &[-1])],
            &[("encoded", F, &[-1, 768, -1]), ("encoded_len", L, &[-1])],
        )?;
        validate_abi(
            &decoder,
            &[
                ("x", I, &[1, 1]),
                ("h.1", F, &[1, 1, 320]),
                ("c.1", F, &[1, 1, 320]),
            ],
            &[
                ("dec", F, &[1, 1, 320]),
                ("h", F, &[1, 1, 320]),
                ("c", F, &[1, 1, 320]),
            ],
        )?;
        validate_abi(
            &joint,
            &[("enc", F, &[1, 768, 1]), ("dec", F, &[1, 320, 1])],
            &[("joint", F, &[1, 1, 1, size as i64])],
        )?;
        Ok(Self {
            encoder,
            decoder,
            joint,
            vocabulary,
            blank,
        })
    }
    pub fn transcribe(&mut self, audio: &[f32]) -> Result<TranscriptResult, String> {
        aggregate(audio, |part| self.infer_segment(part))
    }
    fn infer_segment(&mut self, part: &[f32]) -> Result<String, String> {
        let (features, frames) = preprocess::features(part)?;
        let input =
            Tensor::from_array(([1usize, 64, frames], features)).map_err(|e| e.to_string())?;
        let length =
            Tensor::from_array(([1usize], vec![frames as i64])).map_err(|e| e.to_string())?;
        let outputs = self
            .encoder
            .run(ort::inputs!["audio_signal"=>input,"length"=>length])
            .map_err(|e| e.to_string())?;
        let encoded = outputs
            .get("encoded")
            .ok_or("Missing encoded")?
            .try_extract_array::<f32>()
            .map_err(|e| e.to_string())?;
        let length = outputs
            .get("encoded_len")
            .ok_or("Missing encoded_len")?
            .try_extract_array::<i32>()
            .map_err(|e| e.to_string())?;
        let n = encoded_frames(encoded.view(), length)?;
        let decoder = &mut self.decoder;
        let joint = &mut self.joint;
        let ids = greedy(
            n,
            self.vocabulary.len(),
            self.blank,
            |label, h, c| {
                let x = Tensor::from_array(([1usize, 1], vec![label as i64]))
                    .map_err(|e| e.to_string())?;
                let h = Tensor::from_array(([1usize, 1, 320], h.to_vec()))
                    .map_err(|e| e.to_string())?;
                let c = Tensor::from_array(([1usize, 1, 320], c.to_vec()))
                    .map_err(|e| e.to_string())?;
                let output = decoder
                    .run(ort::inputs!["x"=>x,"h.1"=>h,"c.1"=>c])
                    .map_err(|e| e.to_string())?;
                let read = |name| {
                    checked_values(
                        output
                            .get(name)
                            .ok_or("Missing decoder output")?
                            .try_extract_array::<f32>()
                            .map_err(|e| e.to_string())?,
                        &[1, 1, 320],
                    )
                };
                Ok(Prediction {
                    dec: read("dec")?,
                    h: read("h")?,
                    c: read("c")?,
                })
            },
            |t, dec| {
                let enc: Vec<f32> = (0..768).map(|i| encoded[[0, i, t]]).collect();
                let enc = Tensor::from_array(([1usize, 768, 1], enc)).map_err(|e| e.to_string())?;
                let dec = Tensor::from_array(([1usize, 320, 1], dec.to_vec()))
                    .map_err(|e| e.to_string())?;
                let output = joint
                    .run(ort::inputs!["enc"=>enc,"dec"=>dec])
                    .map_err(|e| e.to_string())?;
                checked_values(
                    output
                        .get("joint")
                        .ok_or("Missing joint")?
                        .try_extract_array::<f32>()
                        .map_err(|e| e.to_string())?,
                    &[1, 1, 1, self.vocabulary.len()],
                )
            },
        )?;
        let text: String = ids
            .into_iter()
            .map(|i| self.vocabulary[i].as_str())
            .collect();
        Ok(cleanup_text(&text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn prediction(h: &[f32], c: &[f32]) -> Prediction {
        Prediction {
            dec: h.to_vec(),
            h: h.iter().map(|v| v + 1.).collect(),
            c: c.iter().map(|v| v + 2.).collect(),
        }
    }
    #[test]
    fn blank_reuses_cache_and_only_nonblank_commits() {
        let mut calls = 0;
        let mut step = 0;
        let ids = greedy(
            3,
            2,
            1,
            |label, h, c| {
                if calls == 0 {
                    assert_eq!(label, 1);
                    assert_eq!(h, [0.; 320]);
                    assert_eq!(c, [0.; 320]);
                } else {
                    assert_eq!(label, 0);
                    assert_eq!(h, [1.; 320]);
                    assert_eq!(c, [2.; 320]);
                }
                calls += 1;
                Ok(prediction(h, c))
            },
            |t, dec| {
                step += 1;
                if step <= 3 {
                    assert_eq!(dec, [0.; 320]);
                } else {
                    assert_eq!(dec, [1.; 320]);
                }
                Ok(if t == 2 && step == 3 {
                    vec![2., 0.]
                } else {
                    vec![0., 2.]
                })
            },
        )
        .unwrap();
        assert_eq!(ids, vec![0]);
        assert_eq!(calls, 2);
    }
    #[test]
    fn cap_repeats_ties_and_reset_per_segment() {
        for _ in 0..2 {
            let mut calls = 0;
            let mut joint_frames = Vec::new();
            let ids = greedy(
                2,
                2,
                1,
                |_, h, c| {
                    assert_eq!(h[0], calls as f32);
                    calls += 1;
                    Ok(prediction(h, c))
                },
                |frame, _| {
                    joint_frames.push(frame);
                    Ok(vec![1., 1.])
                },
            )
            .unwrap();
            assert_eq!(ids, vec![0; 6]);
            assert_eq!(calls, 6);
            assert_eq!(joint_frames, vec![0, 0, 0, 1, 1, 1]);
            let vocab = vec!["а".to_owned(), "<blk>".into()];
            let text: String = ids.iter().map(|i| vocab[*i].as_str()).collect();
            assert_eq!(cleanup_text(&text), "аааааа");
        }
    }
    #[test]
    fn rejects_bad_prediction_joint_and_encoder_tensors() {
        for field in 0..3 {
            for invalid in [None, Some(f32::NAN), Some(f32::INFINITY)] {
                assert!(greedy(
                    1,
                    2,
                    1,
                    |_, h, c| {
                        let mut p = prediction(h, c);
                        let target = match field {
                            0 => &mut p.dec,
                            1 => &mut p.h,
                            _ => &mut p.c,
                        };
                        if let Some(value) = invalid {
                            target[0] = value;
                        } else {
                            target.pop();
                        }
                        Ok(p)
                    },
                    |_, _| panic!("joint must not run after invalid prediction"),
                )
                .is_err());
            }
        }
        for logits in [vec![], vec![f32::NAN, 0.], vec![0., f32::INFINITY]] {
            assert!(greedy(
                1,
                2,
                1,
                |_, h, c| Ok(prediction(h, c)),
                |_, _| Ok(logits.clone())
            )
            .is_err());
        }
        let enc = ndarray::ArrayD::<f32>::zeros(vec![1, 768, 2]);
        for len in [-1, 3] {
            let l = ndarray::arr1(&[len]).into_dyn();
            assert!(encoded_frames(enc.view(), l.view()).is_err());
        }
        let l = ndarray::arr1(&[2]).into_dyn();
        assert_eq!(encoded_frames(enc.view(), l.view()).unwrap(), 2);
        for shape in [vec![1, 767, 2], vec![2, 768, 2], vec![1, 768]] {
            let enc = ndarray::ArrayD::<f32>::zeros(shape);
            assert!(encoded_frames(enc.view(), l.view()).is_err());
        }
        let bad = ndarray::ArrayD::from_elem(vec![1, 768, 2], f32::NAN);
        assert!(encoded_frames(bad.view(), l.view()).is_err());
        assert!(checked_values(ndarray::arr1(&[0.]).into_dyn().view(), &[1, 1, 1]).is_err());
    }
    #[test]
    fn independent_segments_and_later_failure() {
        let audio = vec![0.1; 800000];
        let mut calls = 0;
        let result = aggregate(&audio, |_| {
            let mut predictions = 0;
            let ids = greedy(
                1,
                2,
                1,
                |label, h, c| {
                    assert_eq!(label, if predictions == 0 { 1 } else { 0 });
                    assert_eq!(h, [predictions as f32; 320]);
                    assert_eq!(c, [2. * predictions as f32; 320]);
                    predictions += 1;
                    Ok(prediction(h, c))
                },
                |_, _| Ok(vec![1., 0.]),
            )?;
            assert_eq!(predictions, 3);
            calls += 1;
            Ok(format!("{}", ids.len()))
        })
        .unwrap();
        assert!(calls >= 2);
        assert_eq!(result.text, vec!["3"; calls].join(" "));
        let mut calls = 0;
        assert!(aggregate(&audio, |_| {
            calls += 1;
            if calls == 2 {
                Err("later".into())
            } else {
                Ok("first".into())
            }
        })
        .is_err());
        assert_eq!(calls, 2);
    }
    #[tokio::test]
    async fn invalid_packages_fail_before_runtime() {
        let dir =
            std::env::temp_dir().join(format!("gigaam-rnnt-invalid-{}", uuid::Uuid::new_v4()));
        assert_eq!(
            RnntEngine::load("v3_ctc", &dir).await.err().unwrap(),
            "Unsupported GigaAM RNNT model ID"
        );
        assert_eq!(
            RnntEngine::load("v3_rnnt", &dir).await.err().unwrap(),
            "Package directory missing"
        );
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("config.json"), b"{}").unwrap();
        let error = RnntEngine::load("v3_rnnt", &dir).await.err().unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(error, "Invalid size: models/config.json");
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
            ("v3_rnnt","GIGAAM_TEST_RNNT_DIR","сегодня мы проверяем распознавание русской речи все четыре модели работают локально на компьютере"),
            ("v3_e2e_rnnt","GIGAAM_TEST_E2E_RNNT_DIR","Сегодня мы проверяем распознавание русской речи. Все четыре модели работают локально на компьютере.")
        ] {
            let mut engine=RnntEngine::load(id,std::env::var(var).expect(var)).await.unwrap();
            let result=engine.transcribe(&audio).unwrap();
            println!("{id} CPU text: {}",result.text);
            assert_eq!(result.text,expected); assert_eq!(result.confidence,None); assert!(!result.is_partial);
            let mut long=audio.clone(); long.resize(400000,0.); long.extend_from_slice(&audio);
            assert_eq!(super::super::segments::split(&long),vec![0..400000,400000..long.len()]);
            let repeated=engine.transcribe(&long).unwrap();
            assert_eq!(repeated.text,format!("{expected} {expected}"));
            assert_eq!(repeated.confidence,None); assert!(!repeated.is_partial);
            println!("{id} CPU long repeated phrase verified");
            for n in [320,479,480] {
                let tone:Vec<f32>=(0..n).map(|i| (0.1*(2.0*std::f64::consts::PI*440.0*i as f64/16000.).sin()) as f32).collect();
                let result=engine.transcribe(&tone).unwrap();
                assert_eq!(result.confidence,None); assert!(!result.is_partial);
            }
            assert_eq!(engine.transcribe(&audio).unwrap().text, expected);
            for n in [0,319,320,479,480,400000,400001] {assert!(engine.transcribe(&vec![0.;n]).unwrap().text.is_empty());}
            for value in [f32::NAN,f32::INFINITY,f32::NEG_INFINITY] {assert!(engine.transcribe(&[value;320]).is_err());}
        }
    }
}

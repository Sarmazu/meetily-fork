//! Shared package vocabulary, ABI and segment aggregation; decoding stays model-specific.
use super::{preprocess, segments};
use crate::audio::transcription::provider::TranscriptResult;
use ort::{tensor::TensorElementType, value::ValueType};
pub(super) fn vocabulary(source: &str, size: usize, blank: usize) -> Result<Vec<String>, String> {
    if size == 0 || blank >= size {
        return Err("Invalid GigaAM vocabulary size/blank".into());
    }
    let mut tokens = vec![None; size];
    for line in source.lines() {
        let (token, id) = line
            .rsplit_once(' ')
            .ok_or("Invalid GigaAM vocabulary line")?;
        let id: usize = id.parse().map_err(|_| "Invalid GigaAM vocabulary ID")?;
        if id >= size || tokens[id].is_some() || token.is_empty() {
            return Err("Invalid or duplicate GigaAM vocabulary entry".into());
        }
        tokens[id] = Some(token.replace('\u{2581}', " "));
    }
    let tokens: Vec<String> = tokens
        .into_iter()
        .collect::<Option<_>>()
        .ok_or("Incomplete GigaAM vocabulary")?;
    if tokens[blank] != "<blk>" || tokens.iter().filter(|t| *t == "<blk>").count() != 1 {
        return Err("Invalid GigaAM blank token".into());
    }
    if matches!(size, 257 | 1025) && tokens[0] != "<unk>" {
        return Err("Invalid E2E unknown token".into());
    }
    Ok(tokens)
}

pub(super) fn tensor_abi(value: &ValueType, ty: TensorElementType, dimensions: &[i64]) -> bool {
    match value {
        ValueType::Tensor {
            ty: actual, shape, ..
        } => {
            *actual == ty
                && shape.len() == dimensions.len()
                && shape
                    .iter()
                    .zip(dimensions)
                    .enumerate()
                    .all(|(axis, (actual, expected))| {
                        if *expected < 0 {
                            *actual < 0 || (axis == 0 && *actual == 1)
                        } else {
                            *actual == *expected
                        }
                    })
        }
        _ => false,
    }
}

pub(super) fn cleanup_text(text: &str) -> String {
    crate::model_support::cleanup_decode_text(text)
}

pub(super) fn aggregate(
    audio: &[f32],
    mut infer: impl FnMut(&[f32]) -> Result<String, String>,
) -> Result<TranscriptResult, String> {
    if audio.iter().any(|x| !x.is_finite()) {
        return Err("Non-finite GigaAM PCM input".into());
    }
    let mut texts = Vec::new();
    for range in segments::split(audio) {
        let part = &audio[range];
        if part.len() < preprocess::WINDOW || part.iter().all(|x| *x == 0.) {
            continue;
        }
        let text = infer(part)?;
        if !text.is_empty() {
            texts.push(text);
        }
    }
    Ok(TranscriptResult {
        text: texts.join(" "),
        confidence: None,
        is_partial: false,
    })
}

#[cfg(test)]
pub(super) fn verify_test_runtime(path: impl AsRef<std::path::Path>) -> Result<(), String> {
    use sha2::Digest;
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if format!("{:x}", sha2::Sha256::digest(bytes))
        != "579b636403983254346a5c1d80bd28f1519cd1e284cd204f8d4ff41f8d711559"
    {
        return Err("GigaAM smoke requires pinned CPU ONNX Runtime 1.22.0 DLL SHA-256".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_preserves_punctuation_case_and_repeated_words() {
        assert_eq!(cleanup_text(" Привет , мир!"), "Привет, мир!");
        assert_eq!(cleanup_text(" да да."), "да да.");
    }
    #[test]
    fn rejects_unpinned_test_runtime() {
        let path = std::env::temp_dir().join(format!(
            "gigaam-invalid-runtime-{}.dll",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&path, b"not the pinned DLL").unwrap();
        let error = verify_test_runtime(&path).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            error,
            "GigaAM smoke requires pinned CPU ONNX Runtime 1.22.0 DLL SHA-256"
        );
    }
    #[test]
    fn e2e_unknown_required_for_both_vocabularies() {
        for size in [257, 1025] {
            let source: String = (0..size)
                .map(|i| {
                    format!(
                        "{} {i}\n",
                        if i == size - 1 {
                            "<blk>".into()
                        } else if i == 0 {
                            "<unk>".into()
                        } else {
                            format!("т{i}")
                        }
                    )
                })
                .collect();
            assert!(vocabulary(&source, size, size - 1).is_ok());
            assert!(vocabulary(&source.replace("<unk>", "wrong"), size, size - 1).is_err());
        }
        assert!(vocabulary("", 0, 0).is_err());
        assert!(vocabulary("а 0", 1, 1).is_err());
    }
    #[test]
    fn fixed_one_only_on_contracted_axes() {
        use ort::tensor::{Shape, SymbolicDimensions};
        let tensor = |shape: &[i64]| ValueType::Tensor {
            ty: TensorElementType::Float32,
            shape: Shape::new(shape.iter().copied()),
            dimension_symbols: SymbolicDimensions::new(vec![String::new(); shape.len()]),
        };
        assert!(tensor_abi(
            &tensor(&[1, 768, -1]),
            TensorElementType::Float32,
            &[-1, 768, -1]
        ));
        assert!(!tensor_abi(
            &tensor(&[1, 768, 1]),
            TensorElementType::Float32,
            &[-1, 768, -1]
        ));
        assert!(tensor_abi(
            &tensor(&[1, 768, 1]),
            TensorElementType::Float32,
            &[1, 768, 1]
        ));
        assert!(!tensor_abi(
            &tensor(&[-1, 1, 320]),
            TensorElementType::Float32,
            &[1, 1, 320]
        ));
    }
}

//! Pinned GigaAM v3 unpadded log-mel frontend (mono PCM f32 / 16 kHz).
use realfft::RealFftPlanner;

pub(crate) const WINDOW: usize = 320;
pub(crate) const LIMIT: usize = 400_000;

fn bf16(value: f64) -> f32 {
    let bits = (value as f32).to_bits();
    f32::from_bits(bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) & 0xffff0000)
}

pub(crate) fn features(audio: &[f32]) -> Result<(Vec<f32>, usize), String> {
    if !(WINDOW..=LIMIT).contains(&audio.len()) || audio.iter().any(|x| !x.is_finite()) {
        return Err("GigaAM requires 320..400000 finite PCM samples per inference".into());
    }
    let frames = (audio.len() - WINDOW) / 160 + 1;
    let window: Vec<f32> = (0..WINDOW)
        .map(|i| bf16(0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / WINDOW as f64).cos()))
        .collect();
    let max_mel = 2595.0 * (1.0_f64 + 8000.0 / 700.0).log10();
    let points: Vec<f64> = (0..66)
        .map(|i| 700.0 * (10.0_f64.powf(max_mel * i as f64 / 65.0 / 2595.0) - 1.0))
        .collect();
    let filters: Vec<Vec<f32>> = (0..64)
        .map(|m| {
            (0..161)
                .map(|k| {
                    let hz = k as f64 * 50.0;
                    bf16(
                        ((hz - points[m]) / (points[m + 1] - points[m]))
                            .min((points[m + 2] - hz) / (points[m + 2] - points[m + 1]))
                            .max(0.0),
                    )
                })
                .collect()
        })
        .collect();
    let fft = RealFftPlanner::<f64>::new().plan_fft_forward(WINDOW);
    let mut input = fft.make_input_vec();
    let mut spectrum = fft.make_output_vec();
    let mut scratch = fft.make_scratch_vec();
    let mut output = vec![0.0; 64 * frames];
    for t in 0..frames {
        for i in 0..WINDOW {
            input[i] = (audio[t * 160 + i] * window[i]) as f64;
        }
        fft.process_with_scratch(&mut input, &mut spectrum, &mut scratch)
            .map_err(|e| e.to_string())?;
        let power: Vec<f32> = spectrum
            .iter()
            .map(|z| {
                let magnitude = z.norm() as f32;
                magnitude * magnitude
            })
            .collect();
        for m in 0..64 {
            let mel: f32 = power.iter().zip(&filters[m]).map(|(p, w)| p * w).sum();
            output[m * frames + t] = mel.clamp(1e-9, 1e9).ln();
        }
    }
    if output.iter().any(|value| !value.is_finite()) {
        return Err("Non-finite GigaAM features".into());
    }
    Ok((output, frames))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_pcm_overflow_is_rejected() {
        assert!(features(&[f32::MAX; 320]).is_err());
    }
    #[test]
    fn reference_features_and_unpadded_lengths() {
        for (n, t) in [(320, 1), (479, 1), (480, 2)] {
            let (f, frames) = features(&vec![0.; n]).unwrap();
            assert_eq!(frames, t);
            assert!(f.iter().all(|x| *x == 1e-9_f32.ln()));
        }
        let input: Vec<f32> = (0..480)
            .map(|i| {
                (0.25 * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 16000.0).sin()) as f32
            })
            .collect();
        let reference: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tools/gigaam-model-contract/reference-features.json"
        ))
        .unwrap();
        let (actual, frames) = features(&input).unwrap();
        assert_eq!(frames, 2);
        for m in 0..64 {
            for t in 0..2 {
                let expected = reference["features"]["values"][0][m][t].as_f64().unwrap() as f32;
                assert!(
                    (actual[m * 2 + t] - expected).abs() <= 1e-4 + 1e-4 * expected.abs(),
                    "mel {m} frame {t}: {} != {expected}",
                    actual[m * 2 + t]
                );
            }
        }
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(features(&vec![value; 320]).is_err());
        }
        assert!(features(&[0.; 319]).is_err());
    }
}

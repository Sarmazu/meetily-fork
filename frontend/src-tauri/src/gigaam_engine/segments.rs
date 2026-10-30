//! The only GigaAM-specific splitter. Ranges partition the input exactly.
use super::preprocess::{LIMIT, WINDOW};
use std::ops::Range;

pub(crate) fn split(audio: &[f32]) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    while audio.len() - start > LIMIT {
        let latest = start + LIMIT;
        let earliest = latest - 48_000;
        // Compare 20 ms local energy windows, taking the latest equal minimum.
        let mut boundary = latest;
        let mut minimum = f64::INFINITY;
        let mut energy: f64 = audio[earliest - WINDOW..earliest]
            .iter()
            .map(|x| (*x as f64).powi(2))
            .sum();
        for end in earliest..=latest {
            if energy <= minimum {
                minimum = energy;
                boundary = end;
            }
            if end < latest {
                energy += (audio[end] as f64).powi(2) - (audio[end - WINDOW] as f64).powi(2);
            }
        }
        if audio.len() - boundary < WINDOW {
            boundary = audio.len() - WINDOW;
        }
        ranges.push(start..boundary);
        start = boundary;
    }
    if start < audio.len() {
        ranges.push(start..audio.len());
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_partition_without_short_tail() {
        for n in [0, 319, 320, 400000, 400001, 800001, 1200001] {
            let ranges = split(&vec![0.; n]);
            let mut end = 0;
            for r in &ranges {
                assert_eq!(r.start, end);
                assert!(r.len() <= LIMIT);
                if n > LIMIT {
                    assert!(r.len() >= WINDOW);
                }
                end = r.end;
            }
            assert_eq!(end, n);
            if n == 400000 {
                assert_eq!(ranges.len(), 1);
            }
            if n == 400001 {
                assert_eq!(ranges, vec![0..399681, 399681..400001]);
            }
        }
    }
    #[test]
    fn chooses_low_energy_in_last_three_seconds() {
        let mut audio = vec![1.; 500000];
        audio[370000 - 320..370000].fill(0.);
        assert_eq!(split(&audio)[0].end, 370000);
    }
}

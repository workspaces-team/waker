//! Mel filterbank construction and projection.
//!
//! Builds triangular mel-scale filters and projects power spectra onto them.

/// Convert frequency in Hz to mel scale.
#[inline]
fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

/// Convert mel scale value back to Hz.
#[inline]
fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0f32.powf(mel / 2595.0) - 1.0)
}

/// A precomputed mel filterbank.
///
/// Stores triangular filters as a flat Vec<f32> with shape `[n_mels, n_bins]`.
pub struct MelFilterbank {
    /// Flat storage: filters[mel_idx * n_bins + bin_idx].
    pub filters: Vec<f32>,
    pub n_mels: usize,
    pub n_bins: usize,
}

impl MelFilterbank {
    /// Build a mel filterbank with the given parameters.
    ///
    /// Matches the JS `buildMelFilterbank` implementation.
    pub fn new(
        sample_rate: u32,
        frame_length: usize,
        n_mels: usize,
        min_hz: f32,
        max_hz: f32,
    ) -> Self {
        let n_bins = frame_length / 2 + 1;
        let mel_min = hz_to_mel(min_hz);
        let mel_max = hz_to_mel(max_hz);

        // Compute n_mels + 2 mel-spaced points
        let n_points = n_mels + 2;
        let mel_points: Vec<f32> = (0..n_points)
            .map(|i| mel_min + (mel_max - mel_min) * i as f32 / (n_points - 1) as f32)
            .collect();

        let bin_points: Vec<usize> = mel_points
            .iter()
            .map(|&mel| {
                let hz = mel_to_hz(mel);
                let bin = (hz / (sample_rate as f32 / frame_length as f32)).floor() as usize;
                bin.min(n_bins - 1)
            })
            .collect();

        let mut filters = vec![0.0f32; n_mels * n_bins];

        for mel_idx in 0..n_mels {
            let left = bin_points[mel_idx];
            let center = bin_points[mel_idx + 1];
            let right = bin_points[mel_idx + 2];
            let up_span = (center as f32 - left as f32).max(1.0);
            let down_span = (right as f32 - center as f32).max(1.0);

            let row_offset = mel_idx * n_bins;

            // Rising slope: left → center
            for bin in left..center.min(n_bins) {
                filters[row_offset + bin] = (bin as f32 - left as f32) / up_span;
            }
            // Falling slope: center → right
            for bin in center..right.min(n_bins) {
                filters[row_offset + bin] = (right as f32 - bin as f32) / down_span;
            }
        }

        Self {
            filters,
            n_mels,
            n_bins,
        }
    }

    /// Project a power spectrum onto the mel filterbank, producing log-mel energies.
    ///
    /// `power` must have length >= `n_bins`.
    /// `mel_out` must have length >= `n_mels`.
    pub fn apply(&self, power: &[f32], mel_out: &mut [f32]) {
        debug_assert!(power.len() >= self.n_bins);
        debug_assert!(mel_out.len() >= self.n_mels);

        for mel_idx in 0..self.n_mels {
            let row_offset = mel_idx * self.n_bins;
            let mut energy: f32 = 0.0;
            for bin in 0..self.n_bins {
                energy += power[bin] * self.filters[row_offset + bin];
            }
            // log-mel with floor + normalization matching JS:
            // (10 * log10(max(energy, 1e-10)) / 10) + 2
            mel_out[mel_idx] = energy.max(1e-10).log10() + 2.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filterbank_has_correct_shape() {
        let fb = MelFilterbank::new(16000, 400, 32, 60.0, 3800.0);
        assert_eq!(fb.n_mels, 32);
        assert_eq!(fb.n_bins, 201);
        assert_eq!(fb.filters.len(), 32 * 201);
    }

    #[test]
    fn filters_are_non_negative() {
        let fb = MelFilterbank::new(16000, 400, 32, 60.0, 3800.0);
        for &v in &fb.filters {
            assert!(v >= 0.0, "filter value is negative: {v}");
        }
    }

    #[test]
    fn apply_produces_finite_values() {
        let fb = MelFilterbank::new(16000, 400, 32, 60.0, 3800.0);
        let power = vec![1.0f32; fb.n_bins];
        let mut mel_out = vec![0.0f32; fb.n_mels];
        fb.apply(&power, &mut mel_out);
        for &v in &mel_out {
            assert!(v.is_finite(), "mel output is not finite: {v}");
        }
    }
}

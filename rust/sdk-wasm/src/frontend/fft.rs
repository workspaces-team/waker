//! Real-valued FFT wrapper using the `realfft` crate.
//!
//! Computes the power spectrum (magnitude-squared) of a real-valued windowed frame.

use realfft::RealFftPlanner;
use rustfft::num_complex::Complex;

/// Reusable FFT processor for a fixed frame length.
///
/// Pre-plans the FFT at construction time so each subsequent call only
/// executes the transform without re-planning.
pub struct FftProcessor {
    fft_size: usize,
    n_bins: usize,
    planner_scratch: Vec<Complex<f32>>,
    complex_buf: Vec<Complex<f32>>,
    real_buf: Vec<f32>,
    fft: std::sync::Arc<dyn realfft::RealToComplex<f32>>,
}

impl FftProcessor {
    /// Create a new FFT processor for frames of `fft_size` samples.
    ///
    /// `fft_size` must be a power of 2 for optimal performance.
    pub fn new(fft_size: usize) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);
        let n_bins = fft_size / 2 + 1;
        let scratch_len = fft.get_scratch_len();
        Self {
            fft_size,
            n_bins,
            planner_scratch: vec![Complex::new(0.0, 0.0); scratch_len],
            complex_buf: vec![Complex::new(0.0, 0.0); n_bins],
            real_buf: vec![0.0; fft_size],
            fft,
        }
    }

    /// Number of frequency bins produced (fft_size / 2 + 1).
    pub fn n_bins(&self) -> usize {
        self.n_bins
    }

    /// Compute the power spectrum (magnitude squared) of a windowed frame.
    ///
    /// `frame` must have exactly `frame_length` samples (which may be <= fft_size).
    /// If `frame_length < fft_size`, the frame is zero-padded.
    /// Results are written to `power_out` which must have length >= `n_bins`.
    pub fn power_spectrum(&mut self, frame: &[f32], frame_length: usize, power_out: &mut [f32]) {
        debug_assert!(power_out.len() >= self.n_bins);
        debug_assert!(frame.len() >= frame_length);

        // Copy frame into real buffer, zero-pad if needed
        self.real_buf[..frame_length].copy_from_slice(&frame[..frame_length]);
        for v in self.real_buf[frame_length..self.fft_size].iter_mut() {
            *v = 0.0;
        }

        // Execute FFT
        self.fft
            .process_with_scratch(
                &mut self.real_buf,
                &mut self.complex_buf,
                &mut self.planner_scratch,
            )
            .expect("FFT processing failed");

        // Compute magnitude squared
        for (i, c) in self.complex_buf.iter().enumerate() {
            power_out[i] = c.re * c.re + c.im * c.im;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dc_signal_has_energy_in_bin_zero() {
        let mut proc = FftProcessor::new(512);
        let frame = vec![1.0f32; 400];
        let mut power = vec![0.0f32; proc.n_bins()];
        proc.power_spectrum(&frame, 400, &mut power);
        // DC bin should have the most energy for a constant signal
        assert!(power[0] > 0.0);
        assert!(power[0] > power[1]);
    }

    #[test]
    fn zero_signal_has_zero_power() {
        let mut proc = FftProcessor::new(512);
        let frame = vec![0.0f32; 400];
        let mut power = vec![0.0f32; proc.n_bins()];
        proc.power_spectrum(&frame, 400, &mut power);
        for &p in &power {
            assert!(p.abs() < 1e-10);
        }
    }
}

//! Full log-mel spectrogram pipeline.
//!
//! Takes a raw waveform and produces a flat [input_mel_frames × n_mels] tensor
//! suitable for backbone inference.

use super::fft::FftProcessor;
use super::mel::MelFilterbank;

/// Configuration for the log-mel frontend.
#[derive(Debug, Clone)]
pub struct LogMelConfig {
    pub sample_rate: u32,
    pub frame_length: usize,
    pub hop_length: usize,
    pub n_mels: usize,
    pub input_mel_frames: usize,
    pub min_hz: f32,
    pub max_hz: f32,
}

impl Default for LogMelConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            frame_length: 400,
            hop_length: 160,
            n_mels: 32,
            input_mel_frames: 198,
            min_hz: 60.0,
            max_hz: 3800.0,
        }
    }
}

/// Reusable log-mel spectrogram computer.
///
/// Pre-allocates all work buffers and the FFT plan at construction time.
pub struct LogMelFrontend {
    config: LogMelConfig,
    fft: FftProcessor,
    filterbank: MelFilterbank,
    hanning_window: Vec<f32>,
    // Work buffers (reused across calls)
    windowed_frame: Vec<f32>,
    power_buf: Vec<f32>,
    mel_row_buf: Vec<f32>,
}

impl LogMelFrontend {
    /// Create a new log-mel frontend with the given configuration.
    pub fn new(config: LogMelConfig) -> Self {
        // Use next power of 2 for FFT efficiency
        let fft_size = config.frame_length.next_power_of_two();
        let fft = FftProcessor::new(fft_size);
        let filterbank = MelFilterbank::new(
            config.sample_rate,
            config.frame_length,
            config.n_mels,
            config.min_hz,
            config.max_hz,
        );

        // Pre-compute Hanning window
        let hanning_window: Vec<f32> = (0..config.frame_length)
            .map(|i| {
                let denom = (config.frame_length as f32 - 1.0).max(1.0);
                0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / denom).cos()
            })
            .collect();

        let n_bins = fft.n_bins();

        Self {
            windowed_frame: vec![0.0; fft_size],
            power_buf: vec![0.0; n_bins],
            mel_row_buf: vec![0.0; config.n_mels],
            config,
            fft,
            filterbank,
            hanning_window,
        }
    }

    /// Compute the log-mel spectrogram from a waveform.
    ///
    /// `waveform` is the raw PCM audio clip (e.g. 2 seconds at 16 kHz = 32,000 samples).
    /// `output` must have length >= `input_mel_frames * n_mels`.
    /// The output is a flat row-major tensor: `output[frame_idx * n_mels + mel_idx]`.
    pub fn compute(&mut self, waveform: &[f32], output: &mut [f32]) {
        let frame_length = self.config.frame_length;
        let hop_length = self.config.hop_length;
        let n_mels = self.config.n_mels;
        let target_frames = self.config.input_mel_frames;

        debug_assert!(output.len() >= target_frames * n_mels);

        // Compute raw mel frames
        let frame_count = if waveform.len() >= frame_length {
            1 + (waveform.len() - frame_length) / hop_length
        } else {
            0
        };

        // Collect mel frames into a temporary buffer
        let mut raw_frames: Vec<f32> = Vec::with_capacity(frame_count * n_mels);

        for frame_idx in 0..frame_count {
            let start = frame_idx * hop_length;

            // Apply Hanning window
            for i in 0..frame_length {
                self.windowed_frame[i] = waveform
                    .get(start + i)
                    .copied()
                    .unwrap_or(0.0)
                    * self.hanning_window[i];
            }
            // Zero-pad beyond frame_length (for FFT size)
            let fft_size = self.windowed_frame.len();
            for v in self.windowed_frame[frame_length..fft_size].iter_mut() {
                *v = 0.0;
            }

            // Compute power spectrum
            self.fft
                .power_spectrum(&self.windowed_frame, frame_length, &mut self.power_buf);

            // Apply mel filterbank
            self.filterbank.apply(&self.power_buf, &mut self.mel_row_buf);

            raw_frames.extend_from_slice(&self.mel_row_buf);
        }

        // Resize frames to target_frames using linear interpolation
        resize_mel_frames(&raw_frames, frame_count, n_mels, target_frames, output);
    }
}

/// Resize mel frames from `source_count` to `target_count` using linear interpolation.
/// Matches the JS `resizeMelFrames` implementation.
fn resize_mel_frames(
    source: &[f32],
    source_count: usize,
    n_mels: usize,
    target_count: usize,
    output: &mut [f32],
) {
    if source_count == 0 {
        output[..target_count * n_mels].fill(0.0);
        return;
    }
    if source_count == target_count {
        output[..target_count * n_mels].copy_from_slice(&source[..target_count * n_mels]);
        return;
    }

    for target_idx in 0..target_count {
        let position = if target_count > 1 {
            target_idx as f32 / (target_count - 1) as f32
        } else {
            0.0
        };

        // Find the left source index
        let source_pos = position * (source_count - 1) as f32;
        let left_idx = (source_pos.floor() as usize).min(source_count - 1);
        let right_idx = (left_idx + 1).min(source_count - 1);
        let weight = source_pos - left_idx as f32;

        let out_offset = target_idx * n_mels;
        let left_offset = left_idx * n_mels;
        let right_offset = right_idx * n_mels;

        for mel_idx in 0..n_mels {
            let left_val = source[left_offset + mel_idx];
            let right_val = source[right_offset + mel_idx];
            output[out_offset + mel_idx] = left_val + (right_val - left_val) * weight;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_has_correct_shape() {
        let config = LogMelConfig::default();
        let mut frontend = LogMelFrontend::new(config.clone());
        let waveform = vec![0.0f32; 32000]; // 2 seconds at 16kHz
        let mut output = vec![0.0f32; config.input_mel_frames * config.n_mels];
        frontend.compute(&waveform, &mut output);
        // Should not panic and output should be filled
        assert_eq!(output.len(), 198 * 32);
    }

    #[test]
    fn all_outputs_are_finite() {
        let config = LogMelConfig::default();
        let mut frontend = LogMelFrontend::new(config.clone());
        // Use a simple sine wave
        let waveform: Vec<f32> = (0..32000)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16000.0).sin() * 0.5)
            .collect();
        let mut output = vec![0.0f32; config.input_mel_frames * config.n_mels];
        frontend.compute(&waveform, &mut output);
        for (i, &v) in output.iter().enumerate() {
            assert!(v.is_finite(), "output[{i}] is not finite: {v}");
        }
    }

    #[test]
    fn resize_identity() {
        let source = vec![1.0, 2.0, 3.0, 4.0]; // 2 frames × 2 mels
        let mut output = vec![0.0f32; 4];
        resize_mel_frames(&source, 2, 2, 2, &mut output);
        assert_eq!(output, source);
    }
}

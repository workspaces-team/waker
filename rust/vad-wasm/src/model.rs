//! Silero VAD v5 forward pass — bespoke implementation.
//!
//! Architecture (16 kHz path):
//!   1. STFT:    Conv1d(1, 258, k=256, s=128, no bias) → magnitude → [129, T]
//!   2. Encoder: Conv1d(129,128,3,s=1,p=1) → ReLU
//!              → Conv1d(128,64,3,s=2,p=1) → ReLU
//!              → Conv1d(64,64,3,s=2,p=1) → ReLU
//!              → Conv1d(64,128,3,s=1,p=1) → ReLU → [128, T']
//!   3. LSTM:    (128→128) single-layer, one step per temporal position
//!   4. Decoder: ReLU → Conv1d(128,1,1) → Sigmoid → speech probability

use crate::nn;
use crate::weights::SileroWeights;

/// Internal state for the LSTM (persisted across chunks).
pub struct VadState {
    pub h: Vec<f32>,  // [128]
    pub c: Vec<f32>,  // [128]
    pub context: Vec<f32>, // internal context buffer for cross-chunk continuity
}

const HIDDEN_SIZE: usize = 128;

/// Number of context samples to prepend from the previous chunk.
/// Silero v5 internally manages context via the STFT overlap.
const CONTEXT_SIZE_16K: usize = 64;

impl VadState {
    pub fn new() -> Self {
        Self {
            h: vec![0.0; HIDDEN_SIZE],
            c: vec![0.0; HIDDEN_SIZE],
            context: vec![0.0; CONTEXT_SIZE_16K],
        }
    }

    pub fn reset(&mut self) {
        self.h.fill(0.0);
        self.c.fill(0.0);
        self.context.fill(0.0);
    }
}

impl Default for VadState {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a single forward pass on a 512-sample chunk at 16 kHz.
///
/// Returns the speech probability in [0.0, 1.0].
pub fn forward(
    audio_chunk: &[f32],
    weights: &SileroWeights,
    state: &mut VadState,
) -> f32 {
    debug_assert_eq!(audio_chunk.len(), 512, "Silero v5 requires exactly 512 samples at 16kHz");

    // Prepend context from previous chunk
    let mut full_input = Vec::with_capacity(CONTEXT_SIZE_16K + 512);
    full_input.extend_from_slice(&state.context);
    full_input.extend_from_slice(audio_chunk);

    // Save context for next chunk (last CONTEXT_SIZE_16K samples)
    state.context.copy_from_slice(&audio_chunk[512 - CONTEXT_SIZE_16K..]);

    let input_len = full_input.len();

    // ─── Step 1: STFT ────────────────────────────────────────────────────
    // Conv1d(1, 258, k=256, s=128, no padding, no bias)
    let stft_out_len = (input_len - 256) / 128 + 1;
    let stft_bias = vec![0.0f32; 258];
    let stft_out = nn::conv1d(
        &full_input,
        1,
        input_len,
        &weights.stft_basis,
        &stft_bias,
        258,
        256,
        128,
        0,
    );

    // Magnitude: [258, T] → [129, T]
    let mag = nn::stft_magnitude(&stft_out, 258, stft_out_len);
    let enc_in_channels = 129;
    let mut enc_len = stft_out_len;

    // ─── Step 2: Encoder ─────────────────────────────────────────────────
    // Conv0: [129, T] → [128, T] (k=3, s=1, p=1) + ReLU
    let mut x = nn::conv1d(
        &mag,
        enc_in_channels,
        enc_len,
        &weights.enc0_w,
        &weights.enc0_b,
        128,
        3,
        1,
        1,
    );
    nn::relu_inplace(&mut x);
    // enc_len stays the same (stride=1, p=1)

    // Conv1: [128, T] → [64, T/2] (k=3, s=2, p=1) + ReLU
    let new_len = (enc_len + 2 * 1 - 3) / 2 + 1;
    x = nn::conv1d(&x, 128, enc_len, &weights.enc1_w, &weights.enc1_b, 64, 3, 2, 1);
    nn::relu_inplace(&mut x);
    enc_len = new_len;

    // Conv2: [64, T/2] → [64, T/4] (k=3, s=2, p=1) + ReLU
    let new_len = (enc_len + 2 * 1 - 3) / 2 + 1;
    x = nn::conv1d(&x, 64, enc_len, &weights.enc2_w, &weights.enc2_b, 64, 3, 2, 1);
    nn::relu_inplace(&mut x);
    enc_len = new_len;

    // Conv3: [128, T/4] → [128, T/4] (k=3, s=1, p=1) + ReLU
    x = nn::conv1d(&x, 64, enc_len, &weights.enc3_w, &weights.enc3_b, 128, 3, 1, 1);
    nn::relu_inplace(&mut x);
    // enc_len stays the same (stride=1, p=1)

    // ─── Step 3: LSTM ────────────────────────────────────────────────────
    // Process each temporal position through the LSTM
    let mut lstm_out = vec![0.0f32; HIDDEN_SIZE * enc_len];
    for t in 0..enc_len {
        // Extract input vector for this timestep: [128]
        let mut timestep_input = vec![0.0f32; HIDDEN_SIZE];
        for ch in 0..HIDDEN_SIZE {
            timestep_input[ch] = x[ch * enc_len + t];
        }

        nn::lstm_step(
            &timestep_input,
            &mut state.h,
            &mut state.c,
            &weights.lstm_w_ih,
            &weights.lstm_w_hh,
            &weights.lstm_b_ih,
            &weights.lstm_b_hh,
            HIDDEN_SIZE,
        );

        // Store LSTM output for this timestep
        for ch in 0..HIDDEN_SIZE {
            lstm_out[ch * enc_len + t] = state.h[ch];
        }
    }

    // ─── Step 4: Decoder ─────────────────────────────────────────────────
    // ReLU on LSTM output
    nn::relu_inplace(&mut lstm_out);

    // Conv(128, 1, k=1, s=1, p=0) → [1, enc_len]
    let dec_out = nn::conv1d(
        &lstm_out,
        128,
        enc_len,
        &weights.dec_w,
        &weights.dec_b,
        1,
        1,
        1,
        0,
    );

    // Sigmoid on the last temporal position
    let last_val = dec_out[enc_len - 1];
    nn::sigmoid(last_val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weights::SileroWeights;

    fn dummy_weights() -> SileroWeights {
        SileroWeights {
            stft_basis: vec![0.01; 258 * 1 * 256],
            enc0_w: vec![0.01; 128 * 129 * 3],
            enc0_b: vec![0.0; 128],
            enc1_w: vec![0.01; 64 * 128 * 3],
            enc1_b: vec![0.0; 64],
            enc2_w: vec![0.01; 64 * 64 * 3],
            enc2_b: vec![0.0; 64],
            enc3_w: vec![0.01; 128 * 64 * 3],
            enc3_b: vec![0.0; 128],
            lstm_w_ih: vec![0.01; 512 * 128],
            lstm_w_hh: vec![0.01; 512 * 128],
            lstm_b_ih: vec![0.0; 512],
            lstm_b_hh: vec![0.0; 512],
            dec_w: vec![0.01; 1 * 128 * 1],
            dec_b: vec![0.0; 1],
        }
    }

    #[test]
    fn forward_returns_valid_probability() {
        let weights = dummy_weights();
        let mut state = VadState::new();
        let chunk = vec![0.0f32; 512];
        let prob = forward(&chunk, &weights, &mut state);
        assert!(prob >= 0.0 && prob <= 1.0, "prob = {prob}");
        assert!(prob.is_finite());
    }

    #[test]
    fn state_persists_across_chunks() {
        let weights = dummy_weights();
        let mut state = VadState::new();
        let chunk1 = vec![0.1f32; 512];
        let chunk2 = vec![0.2f32; 512];

        let p1 = forward(&chunk1, &weights, &mut state);
        let h_after_1 = state.h.clone();

        let p2 = forward(&chunk2, &weights, &mut state);

        // State should have changed between chunks
        assert_ne!(state.h, h_after_1);
        assert!(p1.is_finite());
        assert!(p2.is_finite());
    }

    #[test]
    fn reset_clears_state() {
        let weights = dummy_weights();
        let mut state = VadState::new();
        forward(&[0.5f32; 512], &weights, &mut state);
        assert!(state.h.iter().any(|&v| v != 0.0));
        state.reset();
        assert!(state.h.iter().all(|&v| v == 0.0));
    }
}

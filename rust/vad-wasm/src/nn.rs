//! Core neural network operations for the Silero VAD forward pass.
//!
//! All operations are hand-coded in pure Rust — no external ML framework.

/// 1D convolution: input [in_ch, in_len] × kernel [out_ch, in_ch, k] + bias [out_ch]
///
/// Returns output [out_ch, out_len] where out_len depends on stride and padding.
pub fn conv1d(
    input: &[f32],
    in_channels: usize,
    in_len: usize,
    weight: &[f32],
    bias: &[f32],
    out_channels: usize,
    kernel_size: usize,
    stride: usize,
    padding: usize,
) -> Vec<f32> {
    let out_len = (in_len + 2 * padding - kernel_size) / stride + 1;
    let mut output = vec![0.0f32; out_channels * out_len];

    for oc in 0..out_channels {
        for out_pos in 0..out_len {
            let in_start = out_pos * stride;
            let mut sum = bias[oc];

            for ic in 0..in_channels {
                for k in 0..kernel_size {
                    let in_idx = in_start + k;
                    let in_val = if in_idx >= padding && (in_idx - padding) < in_len {
                        input[ic * in_len + (in_idx - padding)]
                    } else {
                        0.0
                    };
                    let w_idx = oc * in_channels * kernel_size + ic * kernel_size + k;
                    sum += in_val * weight[w_idx];
                }
            }
            output[oc * out_len + out_pos] = sum;
        }
    }

    output
}

/// Apply ReLU activation in-place.
pub fn relu_inplace(data: &mut [f32]) {
    for v in data.iter_mut() {
        *v = v.max(0.0);
    }
}

/// Sigmoid activation.
#[inline]
pub fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x.clamp(-40.0, 40.0)).exp())
}

/// Compute magnitude from STFT output.
///
/// Input is [258, time_len] (real+imag interleaved per frequency bin).
/// Output is [129, time_len] (magnitude at each frequency bin).
pub fn stft_magnitude(stft_out: &[f32], n_filters: usize, time_len: usize) -> Vec<f32> {
    // STFT basis has 258 filters = 129 complex pairs (real, imag)
    let n_freq = n_filters / 2;
    let mut mag = vec![0.0f32; n_freq * time_len];

    for freq in 0..n_freq {
        let real_row = freq * 2;
        let imag_row = freq * 2 + 1;
        for t in 0..time_len {
            let re = stft_out[real_row * time_len + t];
            let im = stft_out[imag_row * time_len + t];
            mag[freq * time_len + t] = (re * re + im * im).sqrt();
        }
    }

    mag
}

/// Single-layer LSTM forward pass.
///
/// - `input`: [input_size] — single timestep
/// - `h`: [hidden_size] — hidden state (mutated in place)
/// - `c`: [hidden_size] — cell state (mutated in place)
/// - `w_ih`: [4*hidden_size, input_size]
/// - `w_hh`: [4*hidden_size, hidden_size]
/// - `b_ih`: [4*hidden_size]
/// - `b_hh`: [4*hidden_size]
///
/// Gate order (PyTorch convention): input, forget, cell, output
pub fn lstm_step(
    input: &[f32],
    h: &mut [f32],
    c: &mut [f32],
    w_ih: &[f32],
    w_hh: &[f32],
    b_ih: &[f32],
    b_hh: &[f32],
    hidden_size: usize,
) {
    let input_size = input.len();
    let gate_size = hidden_size * 4;
    debug_assert_eq!(w_ih.len(), gate_size * input_size);
    debug_assert_eq!(w_hh.len(), gate_size * hidden_size);
    debug_assert_eq!(b_ih.len(), gate_size);
    debug_assert_eq!(b_hh.len(), gate_size);

    // Compute all gates: gates = W_ih @ input + b_ih + W_hh @ h + b_hh
    let mut gates = vec![0.0f32; gate_size];

    for g in 0..gate_size {
        let mut val = b_ih[g] + b_hh[g];
        // W_ih @ input
        let w_ih_offset = g * input_size;
        for i in 0..input_size {
            val += w_ih[w_ih_offset + i] * input[i];
        }
        // W_hh @ h
        let w_hh_offset = g * hidden_size;
        for j in 0..hidden_size {
            val += w_hh[w_hh_offset + j] * h[j];
        }
        gates[g] = val;
    }

    // Split into 4 gates and apply activations
    for j in 0..hidden_size {
        let i_gate = sigmoid(gates[j]); // input gate
        let f_gate = sigmoid(gates[hidden_size + j]); // forget gate
        let g_gate = gates[hidden_size * 2 + j].tanh(); // cell gate
        let o_gate = sigmoid(gates[hidden_size * 3 + j]); // output gate

        c[j] = f_gate * c[j] + i_gate * g_gate;
        h[j] = o_gate * c[j].tanh();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conv1d_identity_kernel() {
        // 1×1 convolution with identity mapping
        let input = vec![1.0, 2.0, 3.0, 4.0]; // [1, 4]
        let weight = vec![1.0]; // [1, 1, 1]
        let bias = vec![0.0]; // [1]
        let output = conv1d(&input, 1, 4, &weight, &bias, 1, 1, 1, 0);
        assert_eq!(output, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn conv1d_with_bias() {
        let input = vec![1.0, 2.0, 3.0]; // [1, 3]
        let weight = vec![1.0]; // [1, 1, 1]
        let bias = vec![0.5]; // [1]
        let output = conv1d(&input, 1, 3, &weight, &bias, 1, 1, 1, 0);
        assert_eq!(output, vec![1.5, 2.5, 3.5]);
    }

    #[test]
    fn conv1d_stride_2() {
        let input = vec![1.0, 2.0, 3.0, 4.0]; // [1, 4]
        let weight = vec![1.0]; // [1, 1, 1]
        let bias = vec![0.0]; // [1]
        let output = conv1d(&input, 1, 4, &weight, &bias, 1, 1, 2, 0);
        assert_eq!(output, vec![1.0, 3.0]);
    }

    #[test]
    fn relu_zeroes_negatives() {
        let mut data = vec![-1.0, 0.0, 1.0, -0.5, 2.0];
        relu_inplace(&mut data);
        assert_eq!(data, vec![0.0, 0.0, 1.0, 0.0, 2.0]);
    }

    #[test]
    fn sigmoid_boundary() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-6);
        assert!(sigmoid(40.0) > 0.999);
        assert!(sigmoid(-40.0) < 0.001);
    }

    #[test]
    fn stft_magnitude_basic() {
        // 4 filters (2 complex pairs), 2 time steps
        let stft_out = vec![
            3.0, 0.0, // real[0]: [3, 0]
            4.0, 0.0, // imag[0]: [4, 0]
            0.0, 1.0, // real[1]: [0, 1]
            0.0, 0.0, // imag[1]: [0, 0]
        ];
        let mag = stft_magnitude(&stft_out, 4, 2);
        // freq=0: sqrt(3²+4²)=5.0, sqrt(0²+0²)=0.0
        // freq=1: sqrt(0²+0²)=0.0, sqrt(1²+0²)=1.0
        assert!((mag[0] - 5.0).abs() < 1e-5);
        assert!((mag[1] - 0.0).abs() < 1e-5);
        assert!((mag[2] - 0.0).abs() < 1e-5);
        assert!((mag[3] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn lstm_step_runs() {
        let hidden = 4;
        let input_size = 4;
        let input = vec![1.0; input_size];
        let mut h = vec![0.0; hidden];
        let mut c = vec![0.0; hidden];
        let w_ih = vec![0.1; hidden * 4 * input_size];
        let w_hh = vec![0.1; hidden * 4 * hidden];
        let b_ih = vec![0.0; hidden * 4];
        let b_hh = vec![0.0; hidden * 4];

        lstm_step(&input, &mut h, &mut c, &w_ih, &w_hh, &b_ih, &b_hh, hidden);

        // h should be non-zero after one step
        for &val in &h {
            assert!(val.abs() > 0.0);
            assert!(val.is_finite());
        }
    }
}

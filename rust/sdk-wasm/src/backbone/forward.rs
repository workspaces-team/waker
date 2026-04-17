//! Bespoke backbone forward pass — eliminates onnxruntime-web entirely.
//!
//! Architecture:
//!   mel_features [1, 198, 32]
//!   → Linear(32, 128) + bias → transpose → [1, 128, 198]
//!   → GroupNorm(1 group, 128 channels)
//!   → 4× { GroupNorm → DepthwiseConv1d(128,k=5,dil=[1,2,4,8]) → PointwiseConv(128→256)
//!           → GELU → PointwiseConv(256→128) → residual add }
//!   → GroupNorm
//!   → Temporal pool: slice 198 → 49 windows of ~4 frames → mean each → [1, 49, 128]
//!   → Linear(128, 96) + bias
//!   → LayerNorm (mean-zero, unit-var, no affine)
//!   → adapter_embeddings [1, 49, 96]

use crate::backbone::weights::BackboneWeights;

const CHANNELS: usize = 128;
const EXPAND_CHANNELS: usize = 256;
const TIME_LEN: usize = 198;
const INPUT_DIM: usize = 32;
const OUTPUT_DIM: usize = 96;
const SEQ_LEN: usize = 49;
const KERNEL_SIZE: usize = 5;
const DILATIONS: [usize; 4] = [1, 2, 4, 8];

// ─── Helper ops ──────────────────────────────────────────────────────────────

/// MatMul: [T, K] × [K, N] → [T, N]
fn matmul_add(input: &[f32], weight: &[f32], bias: &[f32], t: usize, k: usize, n: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; t * n];
    for i in 0..t {
        for j in 0..n {
            let mut sum = bias[j];
            for kk in 0..k {
                sum += input[i * k + kk] * weight[kk * n + j];
            }
            out[i * n + j] = sum;
        }
    }
    out
}

/// Transpose [T, C] → [C, T] (channels-last to channels-first)
fn transpose_2d(data: &[f32], rows: usize, cols: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            out[c * rows + r] = data[r * cols + c];
        }
    }
    out
}

/// InstanceNorm per-channel: for each channel, normalize over time dimension.
/// Then apply scale and shift: out = scale * normalized + shift
fn group_norm(data: &mut [f32], channels: usize, time_len: usize, scale: &[f32], shift: &[f32]) {
    let eps = 1e-5f32;
    for ch in 0..channels {
        let offset = ch * time_len;
        // mean
        let mut mean = 0.0f32;
        for t in 0..time_len {
            mean += data[offset + t];
        }
        mean /= time_len as f32;
        // variance
        let mut var = 0.0f32;
        for t in 0..time_len {
            let d = data[offset + t] - mean;
            var += d * d;
        }
        var /= time_len as f32;
        let inv_std = 1.0 / (var + eps).sqrt();
        // normalize + scale + shift
        let s = scale[ch];
        let sh = shift[ch];
        for t in 0..time_len {
            data[offset + t] = (data[offset + t] - mean) * inv_std * s + sh;
        }
    }
}

/// Depthwise Conv1d: each channel convolved independently.
/// input [C, T], weight [C, 1, K], bias [C], dilation d, padding = d*(K-1)/2
fn depthwise_conv1d(
    input: &[f32],
    weight: &[f32],
    bias: &[f32],
    channels: usize,
    time_len: usize,
    kernel_size: usize,
    dilation: usize,
) -> Vec<f32> {
    let padding = dilation * (kernel_size - 1) / 2;
    let out_len = time_len; // same padding
    let mut output = vec![0.0f32; channels * out_len];

    for ch in 0..channels {
        for t in 0..out_len {
            let mut sum = bias[ch];
            for k in 0..kernel_size {
                let in_pos = (t + k * dilation) as isize - padding as isize;
                if in_pos >= 0 && (in_pos as usize) < time_len {
                    sum += input[ch * time_len + in_pos as usize]
                        * weight[ch * kernel_size + k];
                }
            }
            output[ch * out_len + t] = sum;
        }
    }
    output
}

/// Pointwise Conv1d: Conv1d with kernel=1 = channel mixing.
/// input [in_ch, T], weight [out_ch, in_ch, 1], bias [out_ch]
fn pointwise_conv1d(
    input: &[f32],
    weight: &[f32],
    bias: &[f32],
    in_ch: usize,
    out_ch: usize,
    time_len: usize,
) -> Vec<f32> {
    let mut output = vec![0.0f32; out_ch * time_len];

    for oc in 0..out_ch {
        for t in 0..time_len {
            let mut sum = bias[oc];
            for ic in 0..in_ch {
                sum += input[ic * time_len + t] * weight[oc * in_ch + ic];
            }
            output[oc * time_len + t] = sum;
        }
    }
    output
}

/// GELU activation: x * 0.5 * (1 + erf(x / sqrt(2)))
fn gelu_inplace(data: &mut [f32]) {
    let inv_sqrt2 = 1.0 / std::f32::consts::SQRT_2;
    for v in data.iter_mut() {
        let erf_val = libm::erff(*v * inv_sqrt2);
        *v = *v * 0.5 * (1.0 + erf_val);
    }
}

/// LayerNorm (no affine): normalize each [96]-dim vector to zero mean, unit variance.
fn layer_norm(data: &mut [f32], dim: usize, count: usize) {
    let eps = 1e-5f32;
    for i in 0..count {
        let offset = i * dim;
        let mut mean = 0.0f32;
        for j in 0..dim {
            mean += data[offset + j];
        }
        mean /= dim as f32;

        let mut var = 0.0f32;
        for j in 0..dim {
            let d = data[offset + j] - mean;
            var += d * d;
        }
        var /= dim as f32;
        let inv_std = 1.0 / (var + eps).sqrt().max(eps);

        for j in 0..dim {
            data[offset + j] = (data[offset + j] - mean) * inv_std;
        }
    }
}

// ─── Forward pass ────────────────────────────────────────────────────────────

/// Run the backbone forward pass.
///
/// Input:  `mel` — flat [198 × 32] mel spectrogram (row-major)
/// Output: flat [49 × 96] adapter embeddings (row-major)
pub fn forward(mel: &[f32], weights: &BackboneWeights) -> Vec<f32> {
    debug_assert_eq!(mel.len(), TIME_LEN * INPUT_DIM);

    // ── 1. Input projection: [198, 32] × [32, 128] + bias → [198, 128] ──
    let projected = matmul_add(
        mel,
        &weights.input_proj_w,
        &weights.input_proj_b,
        TIME_LEN,
        INPUT_DIM,
        CHANNELS,
    );

    // ── 2. Transpose to channels-first: [198, 128] → [128, 198] ──
    let mut x = transpose_2d(&projected, TIME_LEN, CHANNELS);

    // ── 3. GroupNorm 0 ──
    group_norm(&mut x, CHANNELS, TIME_LEN, &weights.gnorms[0].scale, &weights.gnorms[0].shift);

    // ── 4. Four depthwise-separable blocks ──
    //
    // Post-norm style: gnorms[1..=4] are applied after each block's residual add,
    // one per block. ONNX node ordering (val_52, val_81, val_110, val_139) confirms
    // each gnorm fires exactly once at the block boundary.
    for block_idx in 0..4 {
        let blk = &weights.blocks[block_idx];

        // Save residual before the conv path
        let residual = x.clone();

        // Depthwise conv
        let dw = depthwise_conv1d(
            &x,
            &blk.dw_w,
            &blk.dw_b,
            CHANNELS,
            TIME_LEN,
            KERNEL_SIZE,
            DILATIONS[block_idx],
        );

        // Pointwise expand: [128, T] → [256, T]
        let mut pw_in = pointwise_conv1d(
            &dw,
            &blk.pw_in_w,
            &blk.pw_in_b,
            CHANNELS,
            EXPAND_CHANNELS,
            TIME_LEN,
        );

        // GELU
        gelu_inplace(&mut pw_in);

        // Pointwise project: [256, T] → [128, T]
        let pw_out = pointwise_conv1d(
            &pw_in,
            &blk.pw_out_w,
            &blk.pw_out_b,
            EXPAND_CHANNELS,
            CHANNELS,
            TIME_LEN,
        );

        // Residual add
        x = residual;
        for i in 0..x.len() {
            x[i] += pw_out[i];
        }

        // Post-block GroupNorm (gnorms[1] after block 0, gnorms[2] after block 1, …, gnorms[4] after block 3)
        let gnorm = &weights.gnorms[block_idx + 1];
        group_norm(&mut x, CHANNELS, TIME_LEN, &gnorm.scale, &gnorm.shift);
    }

    // ── 6. Temporal pooling: [128, 198] → [49, 128] ──
    // Slice into 49 windows and take mean of each
    // Window boundaries come from the ONNX Slice nodes
    let mut pooled = vec![0.0f32; SEQ_LEN * CHANNELS];
    let frames_per_window = TIME_LEN / SEQ_LEN; // 198/49 ≈ 4
    let remainder = TIME_LEN % SEQ_LEN; // 198 - 49*4 = 2

    for win in 0..SEQ_LEN {
        // Distribute remainder frames to first `remainder` windows
        let start = if win < remainder {
            win * (frames_per_window + 1)
        } else {
            remainder * (frames_per_window + 1) + (win - remainder) * frames_per_window
        };
        let win_size = if win < remainder {
            frames_per_window + 1
        } else {
            frames_per_window
        };

        for ch in 0..CHANNELS {
            let mut sum = 0.0f32;
            for t in start..start + win_size {
                sum += x[ch * TIME_LEN + t];
            }
            pooled[win * CHANNELS + ch] = sum / win_size as f32;
        }
    }

    // ── 7. Output projection: [49, 128] × [128, 96] + bias → [49, 96] ──
    let mut embeddings = matmul_add(
        &pooled,
        &weights.output_proj_w,
        &weights.output_proj_b,
        SEQ_LEN,
        CHANNELS,
        OUTPUT_DIM,
    );

    // ── 8. LayerNorm (no affine) ──
    layer_norm(&mut embeddings, OUTPUT_DIM, SEQ_LEN);

    embeddings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backbone::weights::*;

    fn dummy_weights() -> BackboneWeights {
        let gnorm = || GroupNormWeights {
            scale: vec![1.0; CHANNELS],
            shift: vec![0.0; CHANNELS],
        };
        let block = || BlockWeights {
            dw_w: vec![0.01; 128 * 1 * 5],
            dw_b: vec![0.0; 128],
            pw_in_w: vec![0.01; 256 * 128 * 1],
            pw_in_b: vec![0.0; 256],
            pw_out_w: vec![0.01; 128 * 256 * 1],
            pw_out_b: vec![0.0; 128],
        };
        BackboneWeights {
            input_proj_w: vec![0.01; 32 * 128],
            input_proj_b: vec![0.0; 128],
            gnorms: [gnorm(), gnorm(), gnorm(), gnorm(), gnorm()],
            blocks: [block(), block(), block(), block()],
            output_proj_w: vec![0.01; 128 * 96],
            output_proj_b: vec![0.0; 96],
        }
    }

    #[test]
    fn forward_output_shape() {
        let weights = dummy_weights();
        let mel = vec![0.0f32; TIME_LEN * INPUT_DIM];
        let out = forward(&mel, &weights);
        assert_eq!(out.len(), SEQ_LEN * OUTPUT_DIM);
    }

    #[test]
    fn forward_outputs_are_finite() {
        let weights = dummy_weights();
        let mel = vec![0.1f32; TIME_LEN * INPUT_DIM];
        let out = forward(&mel, &weights);
        assert!(out.iter().all(|v| v.is_finite()), "non-finite output");
    }

    #[test]
    fn layer_norm_produces_zero_mean() {
        let mut data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 2 vectors of dim 3
        layer_norm(&mut data, 3, 2);
        // Each group of 3 should have ~zero mean
        let mean1: f32 = data[0..3].iter().sum::<f32>() / 3.0;
        let mean2: f32 = data[3..6].iter().sum::<f32>() / 3.0;
        assert!(mean1.abs() < 1e-5, "mean1={mean1}");
        assert!(mean2.abs() < 1e-5, "mean2={mean2}");
    }

    #[test]
    fn gelu_basic() {
        let mut data = vec![0.0, 1.0, -1.0];
        gelu_inplace(&mut data);
        assert!((data[0] - 0.0).abs() < 1e-4); // gelu(0) = 0
        assert!(data[1] > 0.8); // gelu(1) ≈ 0.841
        assert!(data[2] > -0.2 && data[2] < 0.0); // gelu(-1) ≈ -0.159
    }
}

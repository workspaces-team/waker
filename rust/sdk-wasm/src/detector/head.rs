//! Temporal convolution feature extraction for the detector head.
//!
//! Implements the full temporal conv pipeline:
//! 1. Cosine-basis projection to hidden_width
//! 2. Multi-dilation smooth/edge/accel convolution blocks with residual + normalization
//! 3. Mean + max pooling over time to produce the final feature vector

use super::projection::build_projection_matrix;

/// Configuration for the temporal conv head, loaded from detector.json.
#[derive(Debug, Clone)]
pub struct HeadConfig {
    pub hidden_width: usize,
    pub dilations: Vec<usize>,
    pub smooth_scale: f32,
    pub edge_scale: f32,
    pub accel_scale: f32,
    pub classifier_weight: Vec<f32>,
    pub classifier_bias: f32,
}

/// 1D convolution with "same" padding using a 3-tap kernel and dilation.
///
/// Operates on a single channel (1D signal of length `time_len`).
/// Matches the JS `conv1dSame` per-channel behavior.
#[inline]
fn conv1d_same_channel(
    input: &[f32],
    time_len: usize,
    kernel: &[f32; 3],
    dilation: usize,
    output: &mut [f32],
) {
    for t in 0..time_len {
        let left_idx = if t >= dilation { t - dilation } else { 0 };
        let right_idx = (t + dilation).min(time_len - 1);
        output[t] = input[left_idx] * kernel[0] + input[t] * kernel[1] + input[right_idx] * kernel[2];
    }
}

/// Sequence normalization: zero-mean, unit-variance per feature dimension.
///
/// `data` is [time_len × feature_dim] row-major, normalized in-place.
fn sequence_normalize(data: &mut [f32], time_len: usize, feature_dim: usize) {
    if time_len == 0 {
        return;
    }
    let n = time_len as f32;

    for f in 0..feature_dim {
        // Compute mean
        let mut mean = 0.0f32;
        for t in 0..time_len {
            mean += data[t * feature_dim + f];
        }
        mean /= n;

        // Compute std
        let mut var = 0.0f32;
        for t in 0..time_len {
            let delta = data[t * feature_dim + f] - mean;
            var += delta * delta;
        }
        let std = (var / n).sqrt() + 1e-5;

        // Normalize
        for t in 0..time_len {
            data[t * feature_dim + f] = (data[t * feature_dim + f] - mean) / std;
        }
    }
}

/// Compute temporal convolution features from a backbone embedding sequence.
///
/// `sequence`: flat [seq_len × embedding_dim] row-major (post-wEffective)
/// Returns: feature vector of length `hidden_width * 2` (mean + max pooling)
pub fn temporal_conv_features(
    sequence: &[f32],
    seq_len: usize,
    embedding_dim: usize,
    config: &HeadConfig,
) -> Vec<f32> {
    let hidden_width = config.hidden_width;
    let time_len = seq_len;

    // Step 1: Project input to hidden_width using cosine basis + ReLU
    let projection = build_projection_matrix(embedding_dim, hidden_width);

    // x: channel-first layout [hidden_width][time_len]
    let mut x = vec![0.0f32; hidden_width * time_len];
    for ch in 0..hidden_width {
        let proj_offset = ch * embedding_dim;
        for t in 0..time_len {
            let seq_offset = t * embedding_dim;
            let mut val = 0.0f32;
            for f in 0..embedding_dim {
                val += projection[proj_offset + f] * sequence[seq_offset + f];
            }
            x[ch * time_len + t] = val.max(0.0); // ReLU
        }
    }

    let smooth_kernel: [f32; 3] = [0.25, 0.5, 0.25];
    let edge_kernel: [f32; 3] = [-0.5, 0.0, 0.5];
    let accel_kernel: [f32; 3] = [1.0, -2.0, 1.0];

    // Work buffers for convolution (reused across channels/dilations)
    let mut smooth_buf = vec![0.0f32; time_len];
    let mut edge_buf = vec![0.0f32; time_len];
    let mut accel_buf = vec![0.0f32; time_len];

    // mixed_by_time: [time_len × hidden_width] row-major for sequence_normalize
    let mut mixed_by_time = vec![0.0f32; time_len * hidden_width];

    for &dilation in &config.dilations {
        let d = dilation.max(1);

        for ch in 0..hidden_width {
            let ch_offset = ch * time_len;
            let ch_slice = &x[ch_offset..ch_offset + time_len];

            conv1d_same_channel(ch_slice, time_len, &smooth_kernel, d, &mut smooth_buf);
            conv1d_same_channel(ch_slice, time_len, &edge_kernel, d, &mut edge_buf);
            conv1d_same_channel(ch_slice, time_len, &accel_kernel, d, &mut accel_buf);

            for t in 0..time_len {
                let residual = x[ch_offset + t];
                let mixed = residual
                    + config.smooth_scale * smooth_buf[t]
                    + config.edge_scale * edge_buf[t].abs()
                    + config.accel_scale * accel_buf[t].abs();
                mixed_by_time[t * hidden_width + ch] = mixed.max(0.0); // ReLU
            }
        }

        // Sequence normalize in time-major layout
        sequence_normalize(&mut mixed_by_time, time_len, hidden_width);

        // Transpose back to channel-first for next dilation
        for ch in 0..hidden_width {
            for t in 0..time_len {
                x[ch * time_len + t] = mixed_by_time[t * hidden_width + ch];
            }
        }
    }

    // Step 3: Mean + max pooling over time
    let mut features = vec![0.0f32; hidden_width * 2];
    for ch in 0..hidden_width {
        let ch_offset = ch * time_len;
        let mut sum = 0.0f32;
        let mut max_val = f32::NEG_INFINITY;
        for t in 0..time_len {
            let val = x[ch_offset + t];
            sum += val;
            if val > max_val {
                max_val = val;
            }
        }
        features[ch] = sum / time_len.max(1) as f32;
        features[ch + hidden_width] = if max_val.is_finite() { max_val } else { 0.0 };
    }

    features
}

/// Score a feature vector using the linear classifier.
///
/// Returns the raw sigmoid probability.
pub fn classify(features: &[f32], config: &HeadConfig) -> f32 {
    let weights = &config.classifier_weight;
    let mut logit = config.classifier_bias;
    let len = weights.len().min(features.len());
    for i in 0..len {
        logit += weights[i] * features[i];
    }
    sigmoid(logit)
}

#[inline]
fn sigmoid(x: f32) -> f32 {
    let clamped = x.clamp(-40.0, 40.0);
    1.0 / (1.0 + (-clamped).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigmoid_boundary_values() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-6);
        assert!(sigmoid(40.0) > 0.99);
        assert!(sigmoid(-40.0) < 0.01);
    }

    #[test]
    fn temporal_features_have_correct_length() {
        let config = HeadConfig {
            hidden_width: 128,
            dilations: vec![1, 2, 4],
            smooth_scale: 0.6,
            edge_scale: 0.25,
            accel_scale: 0.1,
            classifier_weight: vec![0.0; 256],
            classifier_bias: 0.0,
        };
        let seq = vec![0.0f32; 49 * 96];
        let features = temporal_conv_features(&seq, 49, 96, &config);
        assert_eq!(features.len(), 256); // hidden_width * 2
    }

    #[test]
    fn classify_zero_features_returns_sigmoid_of_bias() {
        let config = HeadConfig {
            hidden_width: 4,
            dilations: vec![1],
            smooth_scale: 0.6,
            edge_scale: 0.25,
            accel_scale: 0.1,
            classifier_weight: vec![1.0; 8],
            classifier_bias: -2.0,
        };
        let features = vec![0.0f32; 8];
        let score = classify(&features, &config);
        let expected = sigmoid(-2.0);
        assert!((score - expected).abs() < 1e-6);
    }
}

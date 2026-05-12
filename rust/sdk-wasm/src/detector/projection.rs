//! Cosine-basis projection and wEffective matrix application.

use std::f32::consts::PI;

/// Build a cosine-basis projection matrix of shape [output_dim, input_dim].
///
/// Returns a flat row-major Vec<f32>.
/// Matches the JS `projectionMatrix` implementation.
pub fn build_projection_matrix(input_dim: usize, output_dim: usize) -> Vec<f32> {
    let mut matrix = vec![0.0f32; output_dim * input_dim];
    for out_idx in 0..output_dim {
        let row_offset = out_idx * input_dim;
        let mut norm_sq = 0.0f32;
        for in_idx in 0..input_dim {
            let phase = (PI / input_dim as f32) * (out_idx as f32 + 0.5) * (in_idx as f32 + 0.5);
            let val = phase.cos();
            matrix[row_offset + in_idx] = val;
            norm_sq += val * val;
        }
        let norm = norm_sq.sqrt() + 1e-6;
        for in_idx in 0..input_dim {
            matrix[row_offset + in_idx] /= norm;
        }
    }
    matrix
}

/// Apply the wEffective matrix to a sequence of embeddings.
///
/// `sequence`: flat [seq_len × input_dim] row-major
/// `w_effective`: flat [output_dim × input_dim] row-major
/// `output`: flat [seq_len × output_dim] row-major
pub fn apply_w_effective(
    sequence: &[f32],
    seq_len: usize,
    input_dim: usize,
    w_effective: &[f32],
    output_dim: usize,
    output: &mut [f32],
) {
    debug_assert_eq!(sequence.len(), seq_len * input_dim);
    debug_assert_eq!(w_effective.len(), output_dim * input_dim);
    debug_assert!(output.len() >= seq_len * output_dim);

    for t in 0..seq_len {
        let seq_offset = t * input_dim;
        let out_offset = t * output_dim;
        for o in 0..output_dim {
            let w_offset = o * input_dim;
            let mut val = 0.0f32;
            for i in 0..input_dim {
                val += w_effective[w_offset + i] * sequence[seq_offset + i];
            }
            output[out_offset + o] = val;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_matrix_has_correct_shape() {
        let mat = build_projection_matrix(96, 128);
        assert_eq!(mat.len(), 128 * 96);
    }

    #[test]
    fn projection_rows_are_approximately_unit_norm() {
        let mat = build_projection_matrix(96, 128);
        for row_idx in 0..128 {
            let offset = row_idx * 96;
            let norm_sq: f32 = (0..96).map(|i| mat[offset + i] * mat[offset + i]).sum();
            let norm = norm_sq.sqrt();
            assert!((norm - 1.0).abs() < 0.01, "row {row_idx} norm = {norm}");
        }
    }

    #[test]
    fn w_effective_identity_passes_through() {
        let dim = 4;
        // Identity matrix
        let mut w = vec![0.0f32; dim * dim];
        for i in 0..dim {
            w[i * dim + i] = 1.0;
        }
        let seq = vec![1.0, 2.0, 3.0, 4.0]; // 1 timestep × 4 dim
        let mut out = vec![0.0f32; dim];
        apply_w_effective(&seq, 1, dim, &w, dim, &mut out);
        assert_eq!(out, seq);
    }
}

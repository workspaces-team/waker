//! Linear resampling from capture sample rate to model sample rate.

/// Resample a chunk of f32 PCM from `source_rate` to `target_rate` using
/// simple box-average interpolation (matching the JS implementation).
pub fn resample(input: &[f32], source_rate: u32, target_rate: u32, output: &mut Vec<f32>) {
    output.clear();
    if input.is_empty() || source_rate == 0 || target_rate == 0 {
        return;
    }
    if source_rate == target_rate {
        output.extend_from_slice(input);
        return;
    }
    let ratio = source_rate as f64 / target_rate as f64;
    let output_len = (input.len() as f64 / ratio).floor().max(1.0) as usize;
    output.reserve(output_len);

    for out_idx in 0..output_len {
        let start = (out_idx as f64 * ratio).floor() as usize;
        let end = ((out_idx + 1) as f64 * ratio).floor().min(input.len() as f64) as usize;
        if end <= start {
            output.push(0.0);
            continue;
        }
        let sum: f32 = input[start..end].iter().sum();
        output.push(sum / (end - start) as f32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_resampling() {
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let mut output = Vec::new();
        resample(&input, 16000, 16000, &mut output);
        assert_eq!(output, input);
    }

    #[test]
    fn downsample_halves_length() {
        let input: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let mut output = Vec::new();
        resample(&input, 24000, 16000, &mut output);
        // 24000/16000 = 1.5x ratio, so 100 samples → ~66 samples
        assert!(!output.is_empty());
        assert!(output.len() < input.len());
    }

    #[test]
    fn empty_input_produces_empty_output() {
        let mut output = Vec::new();
        resample(&[], 24000, 16000, &mut output);
        assert!(output.is_empty());
    }
}

//! Mu-Law audio decoding.
//!
//! Converts 8-bit Mu-Law encoded bytes to normalized f32 PCM samples in [-1, 1].

const MULAW_BIAS: i32 = 0x84;
const INT16_NORMALIZATION: f32 = 32_768.0;

/// Decode a single Mu-Law byte to a normalized f32 sample.
#[inline]
fn decode_sample(value: u8) -> f32 {
    let mu_law_byte = (!value) as i32 & 0xFF;
    let sign: f32 = if (mu_law_byte & 0x80) != 0 { -1.0 } else { 1.0 };
    let exponent = (mu_law_byte >> 4) & 0x07;
    let mantissa = mu_law_byte & 0x0F;
    let magnitude = ((mantissa << 3) + MULAW_BIAS) << exponent;
    let sample = ((magnitude - MULAW_BIAS) as f32 * sign) / INT16_NORMALIZATION;
    sample.clamp(-1.0, 1.0)
}

/// Decode a slice of Mu-Law bytes into f32 PCM samples.
pub fn decode_chunk(input: &[u8], output: &mut [f32]) {
    debug_assert!(output.len() >= input.len());
    for (i, &byte) in input.iter().enumerate() {
        output[i] = decode_sample(byte);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_decodes_to_near_zero() {
        // Mu-Law silence is 0xFF (positive) or 0x7F (negative)
        let pos_silence = decode_sample(0xFF);
        let neg_silence = decode_sample(0x7F);
        assert!(pos_silence.abs() < 0.01, "positive silence: {pos_silence}");
        assert!(neg_silence.abs() < 0.01, "negative silence: {neg_silence}");
    }

    #[test]
    fn output_is_bounded() {
        for byte in 0u8..=255 {
            let sample = decode_sample(byte);
            assert!(
                (-1.0..=1.0).contains(&sample),
                "byte {byte} decoded to {sample}"
            );
        }
    }

    #[test]
    fn decode_chunk_matches_individual() {
        let input: Vec<u8> = (0..=255).collect();
        let mut output = vec![0.0f32; 256];
        decode_chunk(&input, &mut output);
        for (i, &byte) in input.iter().enumerate() {
            assert_eq!(output[i], decode_sample(byte));
        }
    }
}

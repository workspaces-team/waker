//! Weight loading from the extracted Silero VAD binary blob.
//!
//! The weights are loaded at runtime from a flat f32 binary file (`silero_vad_16k.bin`)
//! passed from JavaScript, plus a JSON manifest that maps tensor names to byte offsets.

use serde::Deserialize;
use std::collections::HashMap;

/// Shape and location of a single weight tensor in the binary blob.
#[derive(Debug, Clone, Deserialize)]
pub struct WeightEntry {
    pub shape: Vec<usize>,
    pub offset: usize,
    pub size: usize,
}

/// Manifest mapping weight names to their location in the binary blob.
pub type WeightManifest = HashMap<String, WeightEntry>;

/// All loaded weight tensors for the 16 kHz Silero VAD model.
pub struct SileroWeights {
    // STFT basis: [258, 1, 256]
    pub stft_basis: Vec<f32>,

    // Encoder conv blocks
    pub enc0_w: Vec<f32>, // [128, 129, 3]
    pub enc0_b: Vec<f32>, // [128]
    pub enc1_w: Vec<f32>, // [64, 128, 3]
    pub enc1_b: Vec<f32>, // [64]
    pub enc2_w: Vec<f32>, // [64, 64, 3]
    pub enc2_b: Vec<f32>, // [64]
    pub enc3_w: Vec<f32>, // [128, 64, 3]
    pub enc3_b: Vec<f32>, // [128]

    // LSTM weights (hidden_size = 128)
    pub lstm_w_ih: Vec<f32>, // [512, 128]
    pub lstm_w_hh: Vec<f32>, // [512, 128]
    pub lstm_b_ih: Vec<f32>, // [512]
    pub lstm_b_hh: Vec<f32>, // [512]

    // Decoder
    pub dec_w: Vec<f32>, // [1, 128, 1]
    pub dec_b: Vec<f32>, // [1]
}

impl SileroWeights {
    /// Load weights from a binary blob using the manifest.
    pub fn from_binary(data: &[u8], manifest_json: &str) -> Result<Self, String> {
        let manifest: WeightManifest =
            serde_json::from_str(manifest_json).map_err(|e| format!("Bad manifest: {e}"))?;

        let load = |name: &str| -> Result<Vec<f32>, String> {
            let entry = manifest
                .get(name)
                .ok_or_else(|| format!("Missing weight: {name}"))?;
            let byte_offset = entry.offset;
            let byte_len = entry.size * 4;
            if byte_offset + byte_len > data.len() {
                return Err(format!(
                    "Weight {name} out of bounds: offset={byte_offset}, len={byte_len}, blob={}",
                    data.len()
                ));
            }
            let slice = &data[byte_offset..byte_offset + byte_len];
            let floats: Vec<f32> = slice
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();
            if floats.len() != entry.size {
                return Err(format!(
                    "Weight {name}: expected {} floats, got {}",
                    entry.size,
                    floats.len()
                ));
            }
            Ok(floats)
        };

        Ok(Self {
            stft_basis: load("stft_basis")?,
            enc0_w: load("enc0_w")?,
            enc0_b: load("enc0_b")?,
            enc1_w: load("enc1_w")?,
            enc1_b: load("enc1_b")?,
            enc2_w: load("enc2_w")?,
            enc2_b: load("enc2_b")?,
            enc3_w: load("enc3_w")?,
            enc3_b: load("enc3_b")?,
            lstm_w_ih: load("lstm_w_ih")?,
            lstm_w_hh: load("lstm_w_hh")?,
            lstm_b_ih: load("lstm_b_ih")?,
            lstm_b_hh: load("lstm_b_hh")?,
            dec_w: load("dec_w")?,
            dec_b: load("dec_b")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_manifest() {
        let json = r#"{"stft_basis": {"shape": [258,1,256], "offset": 0, "size": 66048}}"#;
        let manifest: WeightManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest["stft_basis"].size, 66048);
    }
}

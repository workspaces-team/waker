//! Backbone weight loading from extracted binary blob.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct WeightEntry {
    pub shape: Vec<usize>,
    pub offset: usize,
    pub size: usize,
}

pub type WeightManifest = HashMap<String, WeightEntry>;

/// Block weights for one depthwise-separable conv block.
pub struct BlockWeights {
    pub dw_w: Vec<f32>,     // [128, 1, 5] depthwise
    pub dw_b: Vec<f32>,     // [128]
    pub pw_in_w: Vec<f32>,  // [256, 128, 1] point-wise expand
    pub pw_in_b: Vec<f32>,  // [256]
    pub pw_out_w: Vec<f32>, // [128, 256, 1] point-wise project
    pub pw_out_b: Vec<f32>, // [128]
}

/// GroupNorm (InstanceNorm + scale/shift) parameters.
pub struct GroupNormWeights {
    pub scale: Vec<f32>, // [128]
    pub shift: Vec<f32>, // [128]
}

/// All backbone weights.
pub struct BackboneWeights {
    pub input_proj_w: Vec<f32>, // [32, 128]
    pub input_proj_b: Vec<f32>, // [128]

    pub gnorms: [GroupNormWeights; 5], // before each block + final

    pub blocks: [BlockWeights; 4],

    pub output_proj_w: Vec<f32>, // [128, 96]
    pub output_proj_b: Vec<f32>, // [96]
}

impl BackboneWeights {
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
                return Err(format!("Weight {name} out of bounds"));
            }
            let slice = &data[byte_offset..byte_offset + byte_len];
            let floats: Vec<f32> = slice
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            Ok(floats)
        };

        // Flatten [128,1] → [128] for gnorm scale/shift
        let load_flat = |name: &str| -> Result<Vec<f32>, String> {
            let v = load(name)?;
            Ok(v)
        };

        let gnorm_names = [
            ("val_23", "val_25"),
            ("val_52", "val_54"),
            ("val_81", "val_83"),
            ("val_110", "val_112"),
            ("val_139", "val_141"),
        ];

        // Need to use array initialization since GroupNormWeights doesn't impl Default
        let mut gnorms_vec: Vec<GroupNormWeights> = Vec::with_capacity(5);
        for (s, sh) in &gnorm_names {
            gnorms_vec.push(GroupNormWeights {
                scale: load_flat(s)?,
                shift: load_flat(sh)?,
            });
        }

        let mut blocks_vec: Vec<BlockWeights> = Vec::with_capacity(4);
        for i in 0..4 {
            blocks_vec.push(BlockWeights {
                dw_w: load(&format!("blocks_{i}_depthwise_weight"))?,
                dw_b: load(&format!("blocks_{i}_depthwise_bias"))?,
                pw_in_w: load(&format!("blocks_{i}_pointwise_in_weight"))?,
                pw_in_b: load(&format!("blocks_{i}_pointwise_in_bias"))?,
                pw_out_w: load(&format!("blocks_{i}_pointwise_out_weight"))?,
                pw_out_b: load(&format!("blocks_{i}_pointwise_out_bias"))?,
            });
        }

        Ok(Self {
            input_proj_w: load("val_0")?,
            input_proj_b: load("input_proj_bias")?,
            gnorms: [
                gnorms_vec.remove(0),
                gnorms_vec.remove(0),
                gnorms_vec.remove(0),
                gnorms_vec.remove(0),
                gnorms_vec.remove(0),
            ],
            blocks: [
                blocks_vec.remove(0),
                blocks_vec.remove(0),
                blocks_vec.remove(0),
                blocks_vec.remove(0),
            ],
            output_proj_w: load("val_782")?,
            output_proj_b: load("output_proj_bias")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_manifest() {
        let json = r#"{"val_0": {"shape": [32,128], "offset": 0, "size": 4096}}"#;
        let manifest: WeightManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest["val_0"].size, 4096);
    }
}

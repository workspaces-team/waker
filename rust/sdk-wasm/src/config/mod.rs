//! Configuration types deserialized from the registration bundle JSON files.
//!
//! These types match the JSON shapes used in the `@waker/sdk-web` runtime bundle.

use serde::{Deserialize, Serialize};

/// Top-level registration.json
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Registration {
    pub registration_id: String,
    pub requested_keyword: String,
    pub chosen_wake_form: String,
    pub registration_policy: String,
    #[serde(default)]
    pub accepted_wake_forms: Vec<String>,
    #[serde(default)]
    pub sibling_negative_forms: Vec<String>,
    #[serde(default)]
    pub structural_confusables: Vec<String>,
    pub detector_config_path: String,
    pub backbone_model_path: Option<String>,
    pub runtime_config_path: Option<String>,
    pub policy_path: Option<String>,
    pub bundle_manifest_path: Option<String>,
    pub backbone_package_manifest_path: Option<String>,
}

/// Decision policy within detector.json
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionPolicy {
    #[serde(default = "default_threshold")]
    pub threshold: f32,
    #[serde(default = "default_confirmation_hits")]
    pub confirmation_hits: u32,
    #[serde(default = "default_cooldown_seconds")]
    pub cooldown_seconds: f32,
}

fn default_threshold() -> f32 {
    0.5
}
fn default_confirmation_hits() -> u32 {
    1
}
fn default_cooldown_seconds() -> f32 {
    1.0
}

impl Default for DecisionPolicy {
    fn default() -> Self {
        Self {
            threshold: default_threshold(),
            confirmation_hits: default_confirmation_hits(),
            cooldown_seconds: default_cooldown_seconds(),
        }
    }
}

/// wEffective matrix shape and data within detector.json
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WEffective {
    pub shape: [usize; 2],
    pub data: Vec<f32>,
}

/// Temperature calibration within detector.json
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemperatureConfig {
    pub temperature: Option<f32>,
    pub validation_loss: Option<f32>,
}

/// Head configuration within detector.json
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadJsonConfig {
    pub hidden_width: usize,
    pub dilations: Vec<usize>,
    pub smooth_scale: f32,
    pub edge_scale: f32,
    pub accel_scale: f32,
    pub classifier_weight: Vec<f32>,
    pub classifier_bias: f32,
    pub implementation: String,
}

/// Runtime backbone configuration within detector.json
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBackboneConfig {
    pub sample_rate: Option<u32>,
    pub clip_duration_seconds: Option<f32>,
    pub input_dim: Option<usize>,
    pub input_mel_frames: Option<usize>,
    pub sequence_length: Option<usize>,
    pub embedding_dim: Option<usize>,
    pub model_path: Option<String>,
}

/// Top-level detector.json (registration/\<slug\>/detector.json)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectorConfig {
    pub schema_version: u32,
    pub detector_format: String,
    pub keyword: String,
    pub sequence_length: Option<usize>,
    pub embedding_dim: Option<usize>,
    #[serde(default)]
    pub decision_policy: Option<DecisionPolicy>,
    pub head: HeadJsonConfig,
    pub w_effective: WEffective,
    pub temperature: Option<TemperatureConfig>,
    pub runtime_backbone: Option<RuntimeBackboneConfig>,
}

/// Frontend configuration (frontend.json)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendConfig {
    pub schema_version: u32,
    pub frontend_format: String,
    pub sample_rate: u32,
    pub clip_duration_seconds: f32,
    pub frame_length: usize,
    pub hop_length: usize,
    pub n_mels: usize,
    pub input_mel_frames: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_minimal_registration() {
        let json = r#"{
            "registrationId": "test",
            "requestedKeyword": "Operator",
            "chosenWakeForm": "operator",
            "registrationPolicy": "single_word_only",
            "acceptedWakeForms": ["operator"],
            "detectorConfigPath": "registration/operator/detector.json"
        }"#;
        let reg: Registration = serde_json::from_str(json).unwrap();
        assert_eq!(reg.registration_id, "test");
        assert_eq!(reg.chosen_wake_form, "operator");
    }

    #[test]
    fn decision_policy_defaults() {
        let policy = DecisionPolicy::default();
        assert_eq!(policy.threshold, 0.5);
        assert_eq!(policy.confirmation_hits, 1);
        assert_eq!(policy.cooldown_seconds, 1.0);
    }
}

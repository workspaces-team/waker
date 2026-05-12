//! Browser-side temporal-conv head training utilities.
//!
//! The goal here is not to recreate the entire Academy pipeline in the browser.
//! Instead, we expose a compact registration-oriented trainer that can fit a
//! lightweight classifier head on top of frozen backbone embeddings and emit a
//! single JSON artifact the web runtime can load directly.

use serde::{Deserialize, Serialize};

use crate::config::{
    DecisionPolicy, DetectorConfig, HeadJsonConfig, Registration, RuntimeBackboneConfig,
    TemperatureConfig, WEffective,
};
use crate::detector::{head, projection};

const DEFAULT_SAMPLE_RATE: u32 = 16_000;
const DEFAULT_CLIP_DURATION_SECONDS: f32 = 2.0;
const DEFAULT_INPUT_MEL_FRAMES: usize = 198;
const DEFAULT_SEQUENCE_LENGTH: usize = 49;
const DEFAULT_EMBEDDING_DIM: usize = 96;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserHeadTrainingConfig {
    keyword: String,
    #[serde(default)]
    registration_policy: Option<String>,
    #[serde(default)]
    accepted_wake_forms: Option<Vec<String>>,
    #[serde(default)]
    sibling_negative_forms: Option<Vec<String>>,
    #[serde(default)]
    structural_confusables: Option<Vec<String>>,
    #[serde(default)]
    detector: Option<TrainingDetectorConfig>,
    #[serde(default)]
    runtime_backbone: Option<RuntimeBackboneConfig>,
    #[serde(default)]
    w_effective: Option<WEffective>,
    #[serde(default)]
    learning_rate: Option<f32>,
    #[serde(default)]
    epochs: Option<u32>,
    #[serde(default)]
    focal_gamma: Option<f32>,
    #[serde(default)]
    negative_weight: Option<f32>,
    #[serde(default)]
    l2_reg: Option<f32>,
    #[serde(default)]
    validation_split: Option<f32>,
    #[serde(default)]
    threshold: Option<f32>,
    #[serde(default)]
    threshold_grid: Option<Vec<f32>>,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    confirmation_hits: Option<u32>,
    #[serde(default)]
    cooldown_seconds: Option<f32>,
    #[serde(default)]
    duplicate_suppression_seconds: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrainingDetectorConfig {
    #[serde(default)]
    hidden_width: Option<usize>,
    #[serde(default)]
    dilations: Option<Vec<usize>>,
    #[serde(default)]
    smooth_scale: Option<f32>,
    #[serde(default)]
    edge_scale: Option<f32>,
    #[serde(default)]
    accel_scale: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserHeadArtifact {
    schema_version: u32,
    artifact_format: String,
    registration: Registration,
    detector: DetectorConfig,
    training: BrowserHeadTrainingSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserHeadTrainingSummary {
    example_count: usize,
    positive_count: usize,
    negative_count: usize,
    epochs: u32,
    learning_rate: f32,
    focal_gamma: f32,
    negative_weight: f32,
    l2_reg: f32,
    validation_split: f32,
    feature_dim: usize,
    selected_threshold: f32,
    selected_temperature: f32,
    train_accuracy: f32,
    validation_accuracy: Option<f32>,
}

#[derive(Debug, Clone)]
struct TrainingHyperParams {
    hidden_width: usize,
    dilations: Vec<usize>,
    smooth_scale: f32,
    edge_scale: f32,
    accel_scale: f32,
    learning_rate: f32,
    epochs: u32,
    focal_gamma: f32,
    negative_weight: f32,
    l2_reg: f32,
    validation_split: f32,
    threshold: Option<f32>,
    threshold_grid: Vec<f32>,
    temperature: f32,
    confirmation_hits: u32,
    cooldown_seconds: f32,
    duplicate_suppression_seconds: f32,
}

pub fn train_custom_head_artifact(
    flattened_sequences: &[f32],
    labels: &[u8],
    config_json: &str,
) -> Result<String, String> {
    let config: BrowserHeadTrainingConfig = serde_json::from_str(config_json)
        .map_err(|e| format!("Failed to parse training config: {e}"))?;
    let runtime_backbone = normalize_runtime_backbone(config.runtime_backbone.clone());
    let hyper = normalize_hyper_params(&config);
    let sequence_length = runtime_backbone
        .sequence_length
        .unwrap_or(DEFAULT_SEQUENCE_LENGTH);
    let embedding_dim = runtime_backbone
        .embedding_dim
        .unwrap_or(DEFAULT_EMBEDDING_DIM);
    let sample_size = sequence_length * embedding_dim;
    if sample_size == 0 {
        return Err("Training config resolved to an empty sample shape.".to_string());
    }
    if flattened_sequences.len() != labels.len().saturating_mul(sample_size) {
        return Err(format!(
            "Expected {} floats for {} examples, got {}.",
            labels.len().saturating_mul(sample_size),
            labels.len(),
            flattened_sequences.len()
        ));
    }
    if labels.is_empty() {
        return Err("At least one labeled example is required.".to_string());
    }

    let positive_count = labels.iter().filter(|&&label| label > 0).count();
    let negative_count = labels.len().saturating_sub(positive_count);
    if positive_count == 0 || negative_count == 0 {
        return Err(
            "Training requires at least one positive and one negative example.".to_string(),
        );
    }

    let w_effective = normalize_w_effective(config.w_effective.clone(), embedding_dim)?;
    let output_dim = w_effective.shape[0];
    let feature_dim = hyper.hidden_width * 2;
    let head_config = head::HeadConfig {
        hidden_width: hyper.hidden_width,
        dilations: hyper.dilations.clone(),
        smooth_scale: hyper.smooth_scale,
        edge_scale: hyper.edge_scale,
        accel_scale: hyper.accel_scale,
        classifier_weight: vec![0.0; feature_dim],
        classifier_bias: 0.0,
    };

    let features = build_feature_matrix(
        flattened_sequences,
        labels.len(),
        sequence_length,
        embedding_dim,
        &w_effective,
        output_dim,
        &head_config,
    );

    let (train_indices, validation_indices) =
        stratified_split(labels, hyper.validation_split.clamp(0.0, 0.49));
    let train_stats = compute_feature_stats(&features, &train_indices, feature_dim)?;
    let standardized_train = standardize_rows(
        &features,
        &train_indices,
        feature_dim,
        &train_stats.mean,
        &train_stats.std,
    );
    let positive_rate = (train_indices
        .iter()
        .filter(|&&index| labels[index] > 0)
        .count() as f32
        / train_indices.len().max(1) as f32)
        .clamp(1e-3, 1.0 - 1e-3);
    let mut weights = vec![0.0f32; feature_dim];
    let mut bias = (positive_rate / (1.0 - positive_rate)).ln();

    for _epoch in 0..hyper.epochs {
        let mut grad_w = vec![0.0f32; feature_dim];
        let mut grad_b = 0.0f32;
        for (row_position, &sample_index) in train_indices.iter().enumerate() {
            let row =
                &standardized_train[row_position * feature_dim..(row_position + 1) * feature_dim];
            let label = if labels[sample_index] > 0 {
                1.0f32
            } else {
                0.0f32
            };
            let sample_weight = if label > 0.5 {
                1.0
            } else {
                hyper.negative_weight
            };
            let logit = dot(row, &weights) + bias;
            let prob = sigmoid(logit);
            let pt = if label > 0.5 { prob } else { 1.0 - prob };
            let focal_scale = (1.0 - pt).clamp(1e-4, 1.0).powf(hyper.focal_gamma);
            let error = (prob - label) * focal_scale * sample_weight;
            for feature_index in 0..feature_dim {
                grad_w[feature_index] += row[feature_index] * error;
            }
            grad_b += error;
        }
        let denom = train_indices.len().max(1) as f32;
        for feature_index in 0..feature_dim {
            grad_w[feature_index] =
                grad_w[feature_index] / denom + hyper.l2_reg * weights[feature_index];
            weights[feature_index] -= hyper.learning_rate * grad_w[feature_index];
        }
        bias -= hyper.learning_rate * (grad_b / denom);
    }

    let mut weights_raw = vec![0.0f32; feature_dim];
    for feature_index in 0..feature_dim {
        weights_raw[feature_index] = weights[feature_index] / train_stats.std[feature_index];
    }
    let bias_raw = bias - dot(&train_stats.mean, &weights_raw);

    let train_scores = score_rows(
        &features,
        &train_indices,
        feature_dim,
        &weights_raw,
        bias_raw,
    );
    let validation_scores = if validation_indices.is_empty() {
        Vec::new()
    } else {
        score_rows(
            &features,
            &validation_indices,
            feature_dim,
            &weights_raw,
            bias_raw,
        )
    };

    let selected_threshold = hyper.threshold.unwrap_or_else(|| {
        let labels_for_selection: Vec<u8> = if validation_indices.is_empty() {
            train_indices.iter().map(|&index| labels[index]).collect()
        } else {
            validation_indices
                .iter()
                .map(|&index| labels[index])
                .collect()
        };
        let scores_for_selection: &[f32] = if validation_indices.is_empty() {
            &train_scores
        } else {
            &validation_scores
        };
        select_best_threshold(
            &labels_for_selection,
            scores_for_selection,
            &hyper.threshold_grid,
        )
    });

    let train_accuracy = accuracy(
        &train_indices
            .iter()
            .map(|&index| labels[index])
            .collect::<Vec<_>>(),
        &train_scores,
        selected_threshold,
    );
    let validation_accuracy = if validation_indices.is_empty() {
        None
    } else {
        Some(accuracy(
            &validation_indices
                .iter()
                .map(|&index| labels[index])
                .collect::<Vec<_>>(),
            &validation_scores,
            selected_threshold,
        ))
    };

    let normalized_keyword = normalize_keyword(&config.keyword);
    let registration_policy = config
        .registration_policy
        .clone()
        .unwrap_or_else(|| "single_word_only".to_string());
    let accepted_wake_forms = config
        .accepted_wake_forms
        .clone()
        .filter(|forms| !forms.is_empty())
        .unwrap_or_else(|| default_accepted_wake_forms(&normalized_keyword, &registration_policy));
    let registration = Registration {
        registration_id: format!(
            "browser-{}-{}",
            registration_policy.replace('_', "-"),
            normalized_keyword.replace(' ', "-")
        ),
        requested_keyword: config.keyword.clone(),
        chosen_wake_form: normalized_keyword.clone(),
        registration_policy,
        accepted_wake_forms,
        sibling_negative_forms: config
            .sibling_negative_forms
            .clone()
            .unwrap_or_else(|| default_sibling_negative_forms(&normalized_keyword)),
        structural_confusables: config.structural_confusables.clone().unwrap_or_default(),
        detector_config_path: "detector.json".to_string(),
        backbone_model_path: Some("backbone/model.bin".to_string()),
        runtime_config_path: Some("runtime-config.json".to_string()),
        policy_path: None,
        bundle_manifest_path: None,
        backbone_package_manifest_path: Some("backbone/model_manifest.json".to_string()),
    };

    let detector = DetectorConfig {
        schema_version: 1,
        detector_format: "waker-temporal-conv-registration-v1".to_string(),
        keyword: normalized_keyword,
        sequence_length: Some(sequence_length),
        embedding_dim: Some(embedding_dim),
        decision_policy: Some(DecisionPolicy {
            threshold: selected_threshold,
            confirmation_hits: hyper.confirmation_hits,
            cooldown_seconds: hyper.cooldown_seconds,
            duplicate_suppression_seconds: hyper.duplicate_suppression_seconds,
            score_modifier_policy: None,
        }),
        head: HeadJsonConfig {
            hidden_width: hyper.hidden_width,
            dilations: hyper.dilations.clone(),
            smooth_scale: hyper.smooth_scale,
            edge_scale: hyper.edge_scale,
            accel_scale: hyper.accel_scale,
            classifier_weight: weights_raw,
            classifier_bias: bias_raw,
            implementation: "temporal-conv".to_string(),
        },
        w_effective,
        temperature: Some(TemperatureConfig {
            temperature: Some(hyper.temperature),
            validation_loss: None,
        }),
        runtime_backbone: Some(runtime_backbone),
    };

    let artifact = BrowserHeadArtifact {
        schema_version: 1,
        artifact_format: "waker-browser-head-v1".to_string(),
        registration,
        detector,
        training: BrowserHeadTrainingSummary {
            example_count: labels.len(),
            positive_count,
            negative_count,
            epochs: hyper.epochs,
            learning_rate: hyper.learning_rate,
            focal_gamma: hyper.focal_gamma,
            negative_weight: hyper.negative_weight,
            l2_reg: hyper.l2_reg,
            validation_split: hyper.validation_split.clamp(0.0, 0.49),
            feature_dim,
            selected_threshold,
            selected_temperature: hyper.temperature,
            train_accuracy,
            validation_accuracy,
        },
    };

    serde_json::to_string(&artifact).map_err(|e| format!("Failed to serialize head artifact: {e}"))
}

fn normalize_runtime_backbone(config: Option<RuntimeBackboneConfig>) -> RuntimeBackboneConfig {
    let mut config = config.unwrap_or(RuntimeBackboneConfig {
        sample_rate: Some(DEFAULT_SAMPLE_RATE),
        clip_duration_seconds: Some(DEFAULT_CLIP_DURATION_SECONDS),
        input_dim: Some(32),
        input_mel_frames: Some(DEFAULT_INPUT_MEL_FRAMES),
        sequence_length: Some(DEFAULT_SEQUENCE_LENGTH),
        embedding_dim: Some(DEFAULT_EMBEDDING_DIM),
        model_path: Some("backbone/model.bin".to_string()),
    });
    if config.sample_rate.is_none() {
        config.sample_rate = Some(DEFAULT_SAMPLE_RATE);
    }
    if config.clip_duration_seconds.is_none() {
        config.clip_duration_seconds = Some(DEFAULT_CLIP_DURATION_SECONDS);
    }
    if config.input_mel_frames.is_none() {
        config.input_mel_frames = Some(DEFAULT_INPUT_MEL_FRAMES);
    }
    if config.sequence_length.is_none() {
        config.sequence_length = Some(DEFAULT_SEQUENCE_LENGTH);
    }
    if config.embedding_dim.is_none() {
        config.embedding_dim = Some(DEFAULT_EMBEDDING_DIM);
    }
    config
}

fn normalize_hyper_params(config: &BrowserHeadTrainingConfig) -> TrainingHyperParams {
    let detector = config.detector.clone();
    TrainingHyperParams {
        hidden_width: detector
            .as_ref()
            .and_then(|value| value.hidden_width)
            .unwrap_or(128),
        dilations: detector
            .as_ref()
            .and_then(|value| value.dilations.clone())
            .filter(|values| !values.is_empty())
            .unwrap_or_else(|| vec![1, 2, 4]),
        smooth_scale: detector
            .as_ref()
            .and_then(|value| value.smooth_scale)
            .unwrap_or(0.6),
        edge_scale: detector
            .as_ref()
            .and_then(|value| value.edge_scale)
            .unwrap_or(0.25),
        accel_scale: detector
            .as_ref()
            .and_then(|value| value.accel_scale)
            .unwrap_or(0.1),
        learning_rate: config.learning_rate.unwrap_or(0.08),
        epochs: config.epochs.unwrap_or(32).max(1),
        focal_gamma: config.focal_gamma.unwrap_or(0.0).max(0.0),
        negative_weight: config.negative_weight.unwrap_or(1.5).max(0.1),
        l2_reg: config.l2_reg.unwrap_or(1e-4).max(0.0),
        validation_split: config.validation_split.unwrap_or(0.2),
        threshold: config.threshold,
        threshold_grid: config
            .threshold_grid
            .clone()
            .filter(|values| !values.is_empty())
            .unwrap_or_else(default_threshold_grid),
        temperature: config.temperature.unwrap_or(1.0).max(1e-3),
        confirmation_hits: config.confirmation_hits.unwrap_or(1).max(1),
        cooldown_seconds: config.cooldown_seconds.unwrap_or(2.0).max(0.0),
        duplicate_suppression_seconds: config.duplicate_suppression_seconds.unwrap_or(4.0).max(0.0),
    }
}

fn normalize_w_effective(
    w_effective: Option<WEffective>,
    embedding_dim: usize,
) -> Result<WEffective, String> {
    match w_effective {
        Some(value) => {
            let [output_dim, input_dim] = value.shape;
            if input_dim != embedding_dim {
                return Err(format!(
                    "wEffective input dimension {input_dim} does not match embedding dimension {embedding_dim}."
                ));
            }
            if value.data.len() != output_dim * input_dim {
                return Err(format!(
                    "wEffective data length {} does not match shape {:?}.",
                    value.data.len(),
                    value.shape
                ));
            }
            Ok(value)
        }
        None => Ok(identity_w_effective(embedding_dim)),
    }
}

fn identity_w_effective(dim: usize) -> WEffective {
    let mut data = vec![0.0f32; dim * dim];
    for index in 0..dim {
        data[index * dim + index] = 1.0;
    }
    WEffective {
        shape: [dim, dim],
        data,
    }
}

fn build_feature_matrix(
    flattened_sequences: &[f32],
    sample_count: usize,
    sequence_length: usize,
    embedding_dim: usize,
    w_effective: &WEffective,
    projected_dim: usize,
    head_config: &head::HeadConfig,
) -> Vec<f32> {
    let feature_dim = head_config.hidden_width * 2;
    let sample_size = sequence_length * embedding_dim;
    let mut features = vec![0.0f32; sample_count * feature_dim];
    let mut projected = vec![0.0f32; sequence_length * projected_dim];
    for sample_index in 0..sample_count {
        let sample_offset = sample_index * sample_size;
        let sample = &flattened_sequences[sample_offset..sample_offset + sample_size];
        projection::apply_w_effective(
            sample,
            sequence_length,
            embedding_dim,
            &w_effective.data,
            projected_dim,
            &mut projected,
        );
        let sample_features =
            head::temporal_conv_features(&projected, sequence_length, projected_dim, head_config);
        let feature_offset = sample_index * feature_dim;
        features[feature_offset..feature_offset + feature_dim].copy_from_slice(&sample_features);
    }
    features
}

struct FeatureStats {
    mean: Vec<f32>,
    std: Vec<f32>,
}

fn compute_feature_stats(
    features: &[f32],
    train_indices: &[usize],
    feature_dim: usize,
) -> Result<FeatureStats, String> {
    if train_indices.is_empty() {
        return Err("Training split produced zero training examples.".to_string());
    }
    let mut mean = vec![0.0f32; feature_dim];
    for &sample_index in train_indices {
        let row = &features[sample_index * feature_dim..(sample_index + 1) * feature_dim];
        for feature_index in 0..feature_dim {
            mean[feature_index] += row[feature_index];
        }
    }
    let denom = train_indices.len() as f32;
    for feature_index in 0..feature_dim {
        mean[feature_index] /= denom;
    }
    let mut std = vec![0.0f32; feature_dim];
    for &sample_index in train_indices {
        let row = &features[sample_index * feature_dim..(sample_index + 1) * feature_dim];
        for feature_index in 0..feature_dim {
            let delta = row[feature_index] - mean[feature_index];
            std[feature_index] += delta * delta;
        }
    }
    for feature_index in 0..feature_dim {
        std[feature_index] = (std[feature_index] / denom).sqrt() + 1e-5;
    }
    Ok(FeatureStats { mean, std })
}

fn standardize_rows(
    features: &[f32],
    indices: &[usize],
    feature_dim: usize,
    mean: &[f32],
    std: &[f32],
) -> Vec<f32> {
    let mut output = vec![0.0f32; indices.len() * feature_dim];
    for (row_position, &sample_index) in indices.iter().enumerate() {
        let source = &features[sample_index * feature_dim..(sample_index + 1) * feature_dim];
        let destination = &mut output[row_position * feature_dim..(row_position + 1) * feature_dim];
        for feature_index in 0..feature_dim {
            destination[feature_index] =
                (source[feature_index] - mean[feature_index]) / std[feature_index];
        }
    }
    output
}

fn score_rows(
    features: &[f32],
    indices: &[usize],
    feature_dim: usize,
    weights: &[f32],
    bias: f32,
) -> Vec<f32> {
    indices
        .iter()
        .map(|&sample_index| {
            let row = &features[sample_index * feature_dim..(sample_index + 1) * feature_dim];
            sigmoid(dot(row, weights) + bias)
        })
        .collect()
}

fn stratified_split(labels: &[u8], validation_split: f32) -> (Vec<usize>, Vec<usize>) {
    if validation_split <= 0.0 {
        return ((0..labels.len()).collect(), Vec::new());
    }
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    for (index, &label) in labels.iter().enumerate() {
        if label > 0 {
            positive.push(index);
        } else {
            negative.push(index);
        }
    }

    let positive_val = desired_validation_count(positive.len(), validation_split);
    let negative_val = desired_validation_count(negative.len(), validation_split);
    let mut validation = Vec::with_capacity(positive_val + negative_val);
    validation.extend_from_slice(&positive[positive.len().saturating_sub(positive_val)..]);
    validation.extend_from_slice(&negative[negative.len().saturating_sub(negative_val)..]);

    let mut is_validation = vec![false; labels.len()];
    for &index in &validation {
        is_validation[index] = true;
    }
    let train = (0..labels.len())
        .filter(|&index| !is_validation[index])
        .collect();
    (train, validation)
}

fn desired_validation_count(class_count: usize, validation_split: f32) -> usize {
    if class_count < 3 {
        return 0;
    }
    let proposed = ((class_count as f32) * validation_split).round() as usize;
    proposed.clamp(1, class_count - 1)
}

fn default_threshold_grid() -> Vec<f32> {
    (0..=24)
        .map(|step| 0.2f32 + 0.025f32 * step as f32)
        .collect()
}

fn select_best_threshold(labels: &[u8], scores: &[f32], threshold_grid: &[f32]) -> f32 {
    let mut best_threshold = 0.5f32;
    let mut best_accuracy = -1.0f32;
    for &threshold in threshold_grid {
        let accuracy = accuracy(labels, scores, threshold);
        if accuracy > best_accuracy + 1e-6
            || ((accuracy - best_accuracy).abs() <= 1e-6 && threshold < best_threshold)
        {
            best_accuracy = accuracy;
            best_threshold = threshold;
        }
    }
    best_threshold
}

fn accuracy(labels: &[u8], scores: &[f32], threshold: f32) -> f32 {
    if labels.is_empty() || labels.len() != scores.len() {
        return 0.0;
    }
    let correct = labels
        .iter()
        .zip(scores.iter())
        .filter(|(label, score)| (**score >= threshold) == (**label > 0))
        .count();
    correct as f32 / labels.len() as f32
}

fn dot(lhs: &[f32], rhs: &[f32]) -> f32 {
    lhs.iter()
        .zip(rhs.iter())
        .map(|(left, right)| left * right)
        .sum()
}

fn sigmoid(x: f32) -> f32 {
    let clamped = x.clamp(-40.0, 40.0);
    1.0 / (1.0 + (-clamped).exp())
}

fn normalize_keyword(keyword: &str) -> String {
    keyword
        .split_whitespace()
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn default_accepted_wake_forms(keyword: &str, policy: &str) -> Vec<String> {
    let _ = policy;
    vec![keyword.to_string()]
}

fn default_sibling_negative_forms(keyword: &str) -> Vec<String> {
    if keyword.starts_with("hey ") {
        return Vec::new();
    }
    vec![
        format!("hi {keyword}"),
        format!("hello {keyword}"),
        format!("say {keyword}"),
        format!("hey {keyword} please"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_loadable_head_artifact() {
        let sequence_length = 49usize;
        let embedding_dim = 96usize;
        let mut flattened = vec![0.0f32; 6 * sequence_length * embedding_dim];
        for index in 0..3 {
            let offset = index * sequence_length * embedding_dim;
            for step in 0..sequence_length {
                flattened[offset + step * embedding_dim] = 1.5;
            }
        }
        for index in 3..6 {
            let offset = index * sequence_length * embedding_dim;
            for step in 0..sequence_length {
                flattened[offset + step * embedding_dim] = -1.0;
            }
        }
        let labels = vec![1u8, 1, 1, 0, 0, 0];
        let config = serde_json::json!({
            "keyword": "Navigator",
            "runtimeBackbone": {
                "sequenceLength": sequence_length,
                "embeddingDim": embedding_dim,
                "sampleRate": 16000,
                "clipDurationSeconds": 2.0,
                "inputDim": 32,
                "inputMelFrames": 198
            },
            "epochs": 8,
            "validationSplit": 0.33,
        });

        let artifact_json =
            train_custom_head_artifact(&flattened, &labels, &config.to_string()).unwrap();
        let artifact: BrowserHeadArtifact = serde_json::from_str(&artifact_json).unwrap();

        assert_eq!(artifact.artifact_format, "waker-browser-head-v1");
        assert_eq!(artifact.registration.chosen_wake_form, "navigator");
        assert_eq!(artifact.detector.sequence_length, Some(sequence_length));
        assert_eq!(artifact.detector.embedding_dim, Some(embedding_dim));
        assert_eq!(artifact.training.example_count, 6);
        assert!(!artifact.detector.head.classifier_weight.is_empty());
    }
}

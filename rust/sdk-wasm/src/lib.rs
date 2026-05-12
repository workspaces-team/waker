//! Waker WASM Runtime — a fully self-contained wake-word detector compiled to WebAssembly.
//!
//! This crate implements the **complete** Waker detection pipeline in Rust with
//! **zero dependency on onnxruntime-web**:
//!
//! - Mu-Law audio decoding
//! - Linear resampling (capture rate → 16 kHz)
//! - FFT-based log-mel spectrogram frontend
//! - **Bespoke backbone forward pass** (TCN with depthwise-separable convolutions)
//! - Detector head (wEffective projection, temporal conv features, linear classifier)
//! - Temperature calibration
//! - Confirmation hit counting and cooldown
//!
//! ```js
//! const detector = new WakerWasmDetector();
//! detector.loadConfig(registrationJson, detectorJson, 24000);
//! detector.loadBackboneWeights(weightsBinary, manifestJson);
//!
//! // Full pipeline — no JS, no ORT, everything in WASM:
//! const result = detector.processMuLawChunk(chunk, Date.now());
//! if (result?.detected) { /* wake! */ }
//! ```

// Public so integration tests in tests/ can access internals via the rlib.
pub mod audio;
pub mod backbone;
pub mod config;
pub mod detector;
pub mod frontend;
pub mod trainer;

use wasm_bindgen::prelude::*;

use audio::mulaw;
use audio::resample;
use audio::ring_buffer::RingBuffer;
use backbone::forward as backbone_forward;
use backbone::weights::BackboneWeights;
use config::{
    DetectorConfig, Registration, RuntimeBackboneConfig, ScoreModifierModel, ScoreModifierPolicy,
    ScoreModifierTargetModels,
};
use detector::decision::DecisionState;
use detector::head::{self, HeadConfig};
use detector::projection;
use detector::temperature;
use frontend::log_mel::{LogMelConfig, LogMelFrontend};

// ─── Constants ───────────────────────────────────────────────────────────────

const DEFAULT_CAPTURE_SAMPLE_RATE: u32 = 24_000;
const DEFAULT_SAMPLE_RATE: u32 = 16_000;
const DEFAULT_CLIP_DURATION_SECONDS: f32 = 2.0;
const DEFAULT_INPUT_MEL_FRAMES: usize = 198;
const DEFAULT_N_MELS: usize = 32;
const DEFAULT_SEQUENCE_LENGTH: usize = 49;
const DEFAULT_EMBEDDING_DIM: usize = 96;

fn bounded_logit_adjust(score: f32, risk: f32, penalty: f32) -> f32 {
    let clipped = score.clamp(1.0e-5, 1.0 - 1.0e-5);
    let logit = (clipped / (1.0 - clipped)).ln();
    let adjusted = (logit - penalty * risk).clamp(-40.0, 40.0);
    1.0 / (1.0 + (-adjusted).exp())
}

fn unit_dot(lhs: &[f32], rhs: &[f32]) -> Option<f32> {
    if lhs.len() != rhs.len() || lhs.is_empty() {
        return None;
    }
    Some(lhs.iter().zip(rhs.iter()).map(|(a, b)| a * b).sum())
}

fn score_modifier_frame_dot(
    sequence: &[f32],
    frame: usize,
    embedding_dim: usize,
    direction: &[f32],
) -> Option<f32> {
    let start = frame.checked_mul(embedding_dim)?;
    let end = start.checked_add(embedding_dim)?;
    unit_dot(sequence.get(start..end)?, direction)
}

fn score_modifier_mean_pool_dot(
    sequence: &[f32],
    sequence_length: usize,
    embedding_dim: usize,
    direction: &[f32],
    start_frame: usize,
    end_frame: usize,
) -> Option<f32> {
    if direction.len() != embedding_dim || sequence.len() < sequence_length * embedding_dim {
        return None;
    }
    let start = start_frame.min(sequence_length.saturating_sub(1));
    let end = end_frame.min(sequence_length).max(start + 1);
    let mut total = 0.0f32;
    for frame in start..end {
        total += score_modifier_frame_dot(sequence, frame, embedding_dim, direction)?;
    }
    Some(total / (end - start) as f32)
}

fn score_modifier_model_risk(
    sequence: &[f32],
    sequence_length: usize,
    embedding_dim: usize,
    model: &ScoreModifierModel,
    risk_cap: f32,
    sliding_window_fraction: f32,
) -> f32 {
    let direction = model.direction.as_slice();
    if direction.len() != embedding_dim || sequence.len() < sequence_length * embedding_dim {
        return 0.0;
    }
    let center = model.center.unwrap_or(0.0);
    let scale = model.scale.unwrap_or(1.0).max(1.0e-6);
    let pool = model.pool.as_deref().unwrap_or("mean");
    let raw = match pool {
        "recent" => {
            let frames = ((0.35 * sequence_length as f32).ceil() as usize)
                .max(2)
                .min(sequence_length);
            score_modifier_mean_pool_dot(
                sequence,
                sequence_length,
                embedding_dim,
                direction,
                sequence_length.saturating_sub(frames),
                sequence_length,
            )
        }
        "sliding_onset" => {
            let frames = ((sliding_window_fraction * sequence_length as f32).ceil() as usize)
                .max(2)
                .min(sequence_length);
            let mut best: Option<f32> = None;
            for start in 0..=sequence_length.saturating_sub(frames) {
                if let Some(value) = score_modifier_mean_pool_dot(
                    sequence,
                    sequence_length,
                    embedding_dim,
                    direction,
                    start,
                    start + frames,
                ) {
                    best = Some(best.map_or(value, |current| current.max(value)));
                }
            }
            best
        }
        "centroid_peak_pre" => {
            if let Some(locator) = model.locator_direction.as_deref() {
                if locator.len() == embedding_dim {
                    let frames = ((sliding_window_fraction * sequence_length as f32).ceil()
                        as usize)
                        .max(2)
                        .min(sequence_length);
                    let mut peak_frame = 0usize;
                    let mut peak_score = f32::NEG_INFINITY;
                    for frame in 0..sequence_length {
                        if let Some(value) =
                            score_modifier_frame_dot(sequence, frame, embedding_dim, locator)
                        {
                            if value > peak_score {
                                peak_score = value;
                                peak_frame = frame;
                            }
                        }
                    }
                    let start = peak_frame.saturating_sub(frames.saturating_sub(1));
                    score_modifier_mean_pool_dot(
                        sequence,
                        sequence_length,
                        embedding_dim,
                        direction,
                        start,
                        peak_frame + 1,
                    )
                } else {
                    None
                }
            } else {
                score_modifier_mean_pool_dot(
                    sequence,
                    sequence_length,
                    embedding_dim,
                    direction,
                    0,
                    sequence_length,
                )
            }
        }
        _ => score_modifier_mean_pool_dot(
            sequence,
            sequence_length,
            embedding_dim,
            direction,
            0,
            sequence_length,
        ),
    };
    raw.map(|value| ((value - center) / scale).max(0.0).min(risk_cap))
        .unwrap_or(0.0)
}

// ─── Detection result ────────────────────────────────────────────────────────

#[wasm_bindgen]
pub struct WakerDetectionResult {
    detected: bool,
    score: f32,
    threshold: f32,
    keyword: String,
    chosen_wake_form: String,
    accepted_wake_forms: Vec<String>,
}

#[wasm_bindgen]
impl WakerDetectionResult {
    #[wasm_bindgen(getter)]
    pub fn detected(&self) -> bool {
        self.detected
    }

    #[wasm_bindgen(getter)]
    pub fn score(&self) -> f32 {
        self.score
    }

    #[wasm_bindgen(getter)]
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    #[wasm_bindgen(getter)]
    pub fn keyword(&self) -> String {
        self.keyword.clone()
    }

    #[wasm_bindgen(getter, js_name = "chosenWakeForm")]
    pub fn chosen_wake_form(&self) -> String {
        self.chosen_wake_form.clone()
    }

    /// The wake forms accepted under the active registration policy.
    ///
    /// Mirrors `WakerWebDetectionResult.acceptedWakeForms` in `@waker/sdk-web`.
    #[wasm_bindgen(getter, js_name = "acceptedWakeForms")]
    pub fn accepted_wake_forms(&self) -> Vec<String> {
        self.accepted_wake_forms.clone()
    }
}

// ─── Main detector ───────────────────────────────────────────────────────────

/// The main WASM-based wake-word detector.
///
/// Handles the full audio → detection pipeline or accepts pre-computed backbone
/// embeddings for the detector head only.
#[wasm_bindgen]
pub struct WakerWasmDetector {
    // Audio processing state
    ring_buffer: RingBuffer,
    capture_sample_rate: u32,
    model_sample_rate: u32,
    clip_duration_seconds: f32,

    // Frontend
    frontend: Option<LogMelFrontend>,

    // Detector head config
    head_config: Option<HeadConfig>,
    w_effective: Option<Vec<f32>>,
    w_effective_shape: [usize; 2],
    temperature: Option<f32>,

    // Decision state
    decision_state: DecisionState,
    threshold: f32,
    confirmation_hits: u32,
    cooldown_seconds: f32,
    duplicate_suppression_seconds: f32,
    score_modifier_policy: Option<ScoreModifierPolicy>,

    // Registration metadata
    keyword: String,
    chosen_wake_form: String,
    accepted_wake_forms: Vec<String>,

    // Dimensions
    sequence_length: usize,
    embedding_dim: usize,
    input_mel_frames: usize,
    n_mels: usize,

    // Backbone weights (bespoke forward pass)
    backbone_weights: Option<BackboneWeights>,
    backbone_loaded: bool,

    // Reusable work buffers
    decoded_buf: Vec<f32>,
    resampled_buf: Vec<f32>,
    clip_buf: Vec<f32>,
    mel_buf: Vec<f32>,
    projected_buf: Vec<f32>,

    // Loaded state
    loaded: bool,
}

#[wasm_bindgen]
impl WakerWasmDetector {
    /// Create a new detector instance.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let buffer_size = (DEFAULT_SAMPLE_RATE as f32 * DEFAULT_CLIP_DURATION_SECONDS) as usize;
        Self {
            ring_buffer: RingBuffer::new(buffer_size),
            capture_sample_rate: DEFAULT_CAPTURE_SAMPLE_RATE,
            model_sample_rate: DEFAULT_SAMPLE_RATE,
            clip_duration_seconds: DEFAULT_CLIP_DURATION_SECONDS,
            frontend: None,
            head_config: None,
            w_effective: None,
            w_effective_shape: [DEFAULT_EMBEDDING_DIM, DEFAULT_EMBEDDING_DIM],
            temperature: None,
            decision_state: DecisionState::new(),
            threshold: 0.5,
            confirmation_hits: 1,
            cooldown_seconds: 2.0,
            duplicate_suppression_seconds: 4.0,
            score_modifier_policy: None,
            keyword: String::new(),
            chosen_wake_form: String::new(),
            accepted_wake_forms: Vec::new(),
            sequence_length: DEFAULT_SEQUENCE_LENGTH,
            embedding_dim: DEFAULT_EMBEDDING_DIM,
            input_mel_frames: DEFAULT_INPUT_MEL_FRAMES,
            n_mels: DEFAULT_N_MELS,
            backbone_weights: None,
            backbone_loaded: false,
            decoded_buf: Vec::with_capacity(8000),
            resampled_buf: Vec::new(),
            clip_buf: vec![0.0; buffer_size],
            mel_buf: vec![0.0; DEFAULT_INPUT_MEL_FRAMES * DEFAULT_N_MELS],
            projected_buf: vec![0.0; DEFAULT_SEQUENCE_LENGTH * DEFAULT_EMBEDDING_DIM],
            loaded: false,
        }
    }

    fn apply_runtime_backbone_config(
        &mut self,
        runtime_backbone: Option<&RuntimeBackboneConfig>,
        detector_sequence_length: Option<usize>,
        detector_embedding_dim: Option<usize>,
        capture_sample_rate: u32,
    ) {
        self.model_sample_rate = runtime_backbone
            .and_then(|config| config.sample_rate)
            .unwrap_or(DEFAULT_SAMPLE_RATE);
        self.clip_duration_seconds = runtime_backbone
            .and_then(|config| config.clip_duration_seconds)
            .unwrap_or(DEFAULT_CLIP_DURATION_SECONDS)
            .max(0.5);
        self.n_mels = runtime_backbone
            .and_then(|config| config.input_dim)
            .unwrap_or(DEFAULT_N_MELS);
        self.input_mel_frames = runtime_backbone
            .and_then(|config| config.input_mel_frames)
            .unwrap_or(DEFAULT_INPUT_MEL_FRAMES);
        self.sequence_length = runtime_backbone
            .and_then(|config| config.sequence_length)
            .or(detector_sequence_length)
            .unwrap_or(DEFAULT_SEQUENCE_LENGTH);
        self.embedding_dim = runtime_backbone
            .and_then(|config| config.embedding_dim)
            .or(detector_embedding_dim)
            .unwrap_or(DEFAULT_EMBEDDING_DIM);
        self.capture_sample_rate = capture_sample_rate;

        let buffer_size = (self.model_sample_rate as f32 * self.clip_duration_seconds) as usize;
        self.ring_buffer = RingBuffer::new(buffer_size);
        self.clip_buf = vec![0.0; buffer_size];

        let mel_config = LogMelConfig {
            sample_rate: self.model_sample_rate,
            frame_length: 400,
            hop_length: 160,
            n_mels: self.n_mels,
            input_mel_frames: self.input_mel_frames,
            min_hz: 60.0,
            max_hz: 3800.0,
        };
        self.mel_buf = vec![0.0; self.input_mel_frames * self.n_mels];
        self.frontend = Some(LogMelFrontend::new(mel_config));
    }

    /// Load detector configuration from JSON strings.
    ///
    /// `registration_json`: contents of registration.json
    /// `detector_json`: contents of detector.json
    /// `capture_sample_rate`: the browser capture sample rate (typically 24000)
    #[wasm_bindgen(js_name = "loadConfig")]
    pub fn load_config(
        &mut self,
        registration_json: &str,
        detector_json: &str,
        capture_sample_rate: u32,
    ) -> Result<(), JsValue> {
        let registration: Registration = serde_json::from_str(registration_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse registration.json: {e}")))?;

        let detector: DetectorConfig = serde_json::from_str(detector_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse detector.json: {e}")))?;

        self.apply_runtime_backbone_config(
            detector.runtime_backbone.as_ref(),
            detector.sequence_length,
            detector.embedding_dim,
            capture_sample_rate,
        );

        // Set up detector head
        self.head_config = Some(HeadConfig {
            hidden_width: detector.head.hidden_width,
            dilations: detector.head.dilations.clone(),
            smooth_scale: detector.head.smooth_scale,
            edge_scale: detector.head.edge_scale,
            accel_scale: detector.head.accel_scale,
            classifier_weight: detector.head.classifier_weight.clone(),
            classifier_bias: detector.head.classifier_bias,
        });

        self.w_effective = Some(detector.w_effective.data.clone());
        self.w_effective_shape = detector.w_effective.shape;

        self.temperature = detector.temperature.as_ref().and_then(|t| t.temperature);

        // Decision policy
        let policy = detector.decision_policy.unwrap_or_default();
        self.threshold = policy.threshold;
        self.confirmation_hits = policy.confirmation_hits;
        self.cooldown_seconds = policy.cooldown_seconds;
        self.duplicate_suppression_seconds = policy.duplicate_suppression_seconds;
        self.score_modifier_policy = policy.score_modifier_policy.clone();

        // Registration metadata
        self.keyword = registration.requested_keyword;
        self.chosen_wake_form = registration.chosen_wake_form;
        self.accepted_wake_forms = registration.accepted_wake_forms;

        // Work buffer for embedding projection
        self.projected_buf = vec![0.0; self.sequence_length * self.w_effective_shape[0]];

        self.decision_state.reset();
        self.loaded = true;

        Ok(())
    }

    /// Configure just the frontend/backbone path for browser-side embedding extraction.
    ///
    /// `runtime_backbone_json` should match the `runtimeBackbone` shape from detector.json.
    /// This method seeds a no-op identity head so callers can reuse the same WASM package
    /// for clip embedding and later for a trained detector.
    #[wasm_bindgen(js_name = "configureBackbone")]
    pub fn configure_backbone(
        &mut self,
        runtime_backbone_json: &str,
        capture_sample_rate: u32,
    ) -> Result<(), JsValue> {
        let runtime_backbone: RuntimeBackboneConfig = serde_json::from_str(runtime_backbone_json)
            .map_err(|e| {
            JsValue::from_str(&format!("Failed to parse runtime backbone config: {e}"))
        })?;

        self.apply_runtime_backbone_config(
            Some(&runtime_backbone),
            None,
            None,
            capture_sample_rate,
        );

        let projected_dim = self.embedding_dim;
        let mut identity = vec![0.0f32; projected_dim * projected_dim];
        for index in 0..projected_dim {
            identity[index * projected_dim + index] = 1.0;
        }
        self.head_config = Some(HeadConfig {
            hidden_width: 128,
            dilations: vec![1, 2, 4],
            smooth_scale: 0.6,
            edge_scale: 0.25,
            accel_scale: 0.1,
            classifier_weight: vec![0.0; 256],
            classifier_bias: 0.0,
        });
        self.w_effective = Some(identity);
        self.w_effective_shape = [projected_dim, projected_dim];
        self.temperature = Some(1.0);
        self.threshold = 0.5;
        self.confirmation_hits = 1;
        self.cooldown_seconds = 0.0;
        self.duplicate_suppression_seconds = 0.0;
        self.keyword.clear();
        self.chosen_wake_form.clear();
        self.accepted_wake_forms.clear();
        self.projected_buf = vec![0.0; self.sequence_length * projected_dim];
        self.decision_state.reset();
        self.loaded = true;

        Ok(())
    }

    /// Run a fully buffered 16 kHz mono clip through the frontend and backbone and return
    /// the flat embedding sequence `[sequenceLength * embeddingDim]`.
    #[wasm_bindgen(js_name = "embedPcm16kClip")]
    pub fn embed_pcm16k_clip(&mut self, pcm16k: &[f32]) -> Result<Vec<f32>, JsValue> {
        if !self.loaded {
            return Err(JsValue::from_str(
                "Detector not configured. Call configureBackbone() or loadConfig() first.",
            ));
        }
        if !self.backbone_loaded {
            return Err(JsValue::from_str(
                "Backbone not loaded. Call loadBackboneWeights() first.",
            ));
        }

        let expected_len = self.clip_buf.len();
        if pcm16k.len() >= expected_len {
            let start = pcm16k.len() - expected_len;
            self.clip_buf
                .copy_from_slice(&pcm16k[start..start + expected_len]);
        } else {
            self.clip_buf.fill(0.0);
            self.clip_buf[..pcm16k.len()].copy_from_slice(pcm16k);
        }

        let frontend = self.frontend.as_mut().unwrap();
        frontend.compute(&self.clip_buf, &mut self.mel_buf);
        let backbone_weights = self.backbone_weights.as_ref().unwrap();
        Ok(backbone_forward::forward(&self.mel_buf, backbone_weights))
    }

    /// Process a Mu-Law encoded audio chunk from the browser microphone.
    ///
    /// Returns a detection result once the ring buffer is full, or null if
    /// the buffer is still filling. The backbone inference is handled externally
    /// (by the JS ONNX runtime). Call `processBackboneOutput` instead when the
    /// backbone output is available.
    ///
    /// This method handles: Mu-Law decode → resample → ring buffer → mel frontend.
    /// It returns the mel spectrogram as a flat Float32Array for the JS side to
    /// pass to the ONNX backbone.
    #[wasm_bindgen(js_name = "processAudioToMel")]
    pub fn process_audio_to_mel(&mut self, chunk: &[u8]) -> Result<Option<Vec<f32>>, JsValue> {
        if !self.loaded {
            return Err(JsValue::from_str(
                "Detector not loaded. Call loadConfig() first.",
            ));
        }

        // Decode Mu-Law
        self.decoded_buf.resize(chunk.len(), 0.0);
        mulaw::decode_chunk(chunk, &mut self.decoded_buf);

        // Resample
        resample::resample(
            &self.decoded_buf,
            self.capture_sample_rate,
            self.model_sample_rate,
            &mut self.resampled_buf,
        );

        // Append to ring buffer
        self.ring_buffer.append(&self.resampled_buf);

        if !self.ring_buffer.is_full() {
            return Ok(None);
        }

        // Snapshot ring buffer
        self.ring_buffer.snapshot(&mut self.clip_buf);

        // Compute mel spectrogram
        let frontend = self.frontend.as_mut().unwrap();
        frontend.compute(&self.clip_buf, &mut self.mel_buf);

        Ok(Some(self.mel_buf.clone()))
    }

    /// Score a backbone embedding sequence through the detector head.
    ///
    /// `backbone_output`: flat Float32Array of shape [seq_len × embedding_dim]
    ///     from the ONNX backbone inference on the JS side.
    /// `now_ms`: current timestamp in milliseconds (from Date.now()).
    ///
    /// Returns the detection result with score, threshold, and detected flag.
    #[wasm_bindgen(js_name = "processBackboneOutput")]
    pub fn process_backbone_output(
        &mut self,
        backbone_output: &[f32],
        now_ms: f64,
    ) -> Result<WakerDetectionResult, JsValue> {
        if !self.loaded {
            return Err(JsValue::from_str(
                "Detector not loaded. Call loadConfig() first.",
            ));
        }

        let expected_len = self.sequence_length * self.embedding_dim;
        if backbone_output.len() != expected_len {
            return Err(JsValue::from_str(&format!(
                "Expected backbone output length {expected_len}, got {}",
                backbone_output.len()
            )));
        }

        let head_config = self.head_config.as_ref().unwrap();
        let w_effective = self.w_effective.as_ref().unwrap();
        let [output_dim, input_dim] = self.w_effective_shape;

        // Apply wEffective projection
        projection::apply_w_effective(
            backbone_output,
            self.sequence_length,
            input_dim,
            w_effective,
            output_dim,
            &mut self.projected_buf,
        );

        // Compute temporal conv features
        let features = head::temporal_conv_features(
            &self.projected_buf,
            self.sequence_length,
            output_dim,
            head_config,
        );

        // Classify
        let raw_score = head::classify(&features, head_config);

        // Apply temperature calibration
        let calibrated_score = temperature::apply_temperature(raw_score, self.temperature);
        let adjusted_score = self.apply_score_modifier(calibrated_score, backbone_output);

        // Apply decision logic
        let detected = self.decision_state.observe(
            adjusted_score,
            self.threshold,
            self.confirmation_hits,
            self.cooldown_seconds,
            self.duplicate_suppression_seconds,
            now_ms,
        );

        Ok(WakerDetectionResult {
            detected,
            score: adjusted_score,
            threshold: self.threshold,
            keyword: self.keyword.clone(),
            chosen_wake_form: self.chosen_wake_form.clone(),
            accepted_wake_forms: self.accepted_wake_forms.clone(),
        })
    }

    /// Load backbone weights from the extracted binary blob and manifest.
    ///
    /// This enables the fully self-contained pipeline — no onnxruntime-web needed.
    ///
    /// `weights_binary`: contents of `backbone_16k.bin`
    /// `manifest_json`: contents of `backbone_16k_manifest.json`
    #[wasm_bindgen(js_name = "loadBackboneWeights")]
    pub fn load_backbone_weights(
        &mut self,
        weights_binary: &[u8],
        manifest_json: &str,
    ) -> Result<(), JsValue> {
        let w = BackboneWeights::from_binary(weights_binary, manifest_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to load backbone weights: {e}")))?;
        self.backbone_weights = Some(w);
        self.backbone_loaded = true;
        Ok(())
    }

    /// Process a Mu-Law encoded audio chunk through the **complete** pipeline.
    ///
    /// Mu-Law decode → resample → ring buffer → mel frontend → backbone → detector head → decision.
    ///
    /// **No onnxruntime-web needed.** Everything runs in WASM.
    ///
    /// Returns `None` if the ring buffer is still filling, or a `WakerDetectionResult`
    /// once enough audio has been buffered.
    #[wasm_bindgen(js_name = "processMuLawChunk")]
    pub fn process_mu_law_chunk(
        &mut self,
        chunk: &[u8],
        now_ms: f64,
    ) -> Result<Option<WakerDetectionResult>, JsValue> {
        if !self.loaded {
            return Err(JsValue::from_str(
                "Detector not loaded. Call loadConfig() first.",
            ));
        }
        if !self.backbone_loaded {
            return Err(JsValue::from_str(
                "Backbone not loaded. Call loadBackboneWeights() first.",
            ));
        }

        // Decode Mu-Law
        self.decoded_buf.resize(chunk.len(), 0.0);
        mulaw::decode_chunk(chunk, &mut self.decoded_buf);

        // Resample
        resample::resample(
            &self.decoded_buf,
            self.capture_sample_rate,
            self.model_sample_rate,
            &mut self.resampled_buf,
        );

        // Append to ring buffer
        self.ring_buffer.append(&self.resampled_buf);

        if !self.ring_buffer.is_full() {
            return Ok(None);
        }

        // Snapshot ring buffer
        self.ring_buffer.snapshot(&mut self.clip_buf);

        // Compute mel spectrogram
        let frontend = self.frontend.as_mut().unwrap();
        frontend.compute(&self.clip_buf, &mut self.mel_buf);

        // Run backbone forward pass (bespoke, no ORT)
        let backbone_weights = self.backbone_weights.as_ref().unwrap();
        let backbone_output = backbone_forward::forward(&self.mel_buf, backbone_weights);

        // Score through detector head
        let head_config = self.head_config.as_ref().unwrap();
        let w_effective = self.w_effective.as_ref().unwrap();
        let [output_dim, input_dim] = self.w_effective_shape;

        projection::apply_w_effective(
            &backbone_output,
            self.sequence_length,
            input_dim,
            w_effective,
            output_dim,
            &mut self.projected_buf,
        );

        let features = head::temporal_conv_features(
            &self.projected_buf,
            self.sequence_length,
            output_dim,
            head_config,
        );

        let raw_score = head::classify(&features, head_config);
        let calibrated_score = temperature::apply_temperature(raw_score, self.temperature);
        let adjusted_score = self.apply_score_modifier(calibrated_score, &backbone_output);

        let detected = self.decision_state.observe(
            adjusted_score,
            self.threshold,
            self.confirmation_hits,
            self.cooldown_seconds,
            self.duplicate_suppression_seconds,
            now_ms,
        );

        Ok(Some(WakerDetectionResult {
            detected,
            score: adjusted_score,
            threshold: self.threshold,
            keyword: self.keyword.clone(),
            chosen_wake_form: self.chosen_wake_form.clone(),
            accepted_wake_forms: self.accepted_wake_forms.clone(),
        }))
    }

    fn apply_score_modifier(&self, score: f32, backbone_output: &[f32]) -> f32 {
        let Some(policy) = self.score_modifier_policy.as_ref() else {
            return score;
        };
        let kind = policy.kind.as_deref().unwrap_or("");
        if !matches!(
            kind,
            "bounded_logit_penalty"
                | "v15_score_modifier"
                | "target_conditioned_bounded_logit_penalty"
        ) {
            return score;
        }
        let penalty = policy.penalty.unwrap_or(0.0).max(0.0);
        if penalty <= 0.0 {
            return score;
        }
        let risk_cap = policy.risk_cap.unwrap_or(3.0).max(0.0);
        let sliding_window_fraction = policy
            .sliding_window_fraction
            .unwrap_or(0.18)
            .clamp(0.01, 1.0);
        let mut total_risk = 0.0f32;
        for model in self.score_modifier_models(policy) {
            let weight = self.score_modifier_component_weight(policy, model);
            total_risk += weight
                * score_modifier_model_risk(
                    backbone_output,
                    self.sequence_length,
                    self.embedding_dim,
                    model,
                    risk_cap,
                    sliding_window_fraction,
                );
        }
        if total_risk <= 0.0 {
            return score;
        }
        bounded_logit_adjust(score, total_risk, penalty)
    }

    fn score_modifier_models<'a>(
        &'a self,
        policy: &'a ScoreModifierPolicy,
    ) -> Vec<&'a ScoreModifierModel> {
        match &policy.target_models {
            ScoreModifierTargetModels::List(models) => models.iter().collect(),
            ScoreModifierTargetModels::ByTarget(by_target) => by_target
                .get(&self.keyword)
                .or_else(|| by_target.get(&self.chosen_wake_form))
                .into_iter()
                .flat_map(|models| models.iter())
                .collect(),
        }
    }

    fn score_modifier_component_weight(
        &self,
        policy: &ScoreModifierPolicy,
        model: &ScoreModifierModel,
    ) -> f32 {
        let Some(model_component_id) = model.component_id.as_deref() else {
            return 1.0;
        };
        policy
            .components
            .iter()
            .find(|component| component.component_id.as_deref() == Some(model_component_id))
            .and_then(|component| component.weight)
            .unwrap_or(1.0)
    }

    /// Reset the detector state (ring buffer, decision state).
    pub fn reset(&mut self) {
        self.ring_buffer.reset();
        self.decision_state.reset();
    }

    /// Check if the detector is loaded and ready (config + backbone weights).
    #[wasm_bindgen(getter, js_name = "isLoaded")]
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Check if the backbone weights are loaded.
    #[wasm_bindgen(getter, js_name = "isBackboneLoaded")]
    pub fn is_backbone_loaded(&self) -> bool {
        self.backbone_loaded
    }

    /// Check if the detector is fully ready for the complete pipeline.
    #[wasm_bindgen(getter, js_name = "isFullyReady")]
    pub fn is_fully_ready(&self) -> bool {
        self.loaded && self.backbone_loaded
    }

    /// Get the number of mel features the frontend produces per chunk.
    #[wasm_bindgen(getter, js_name = "melTensorLength")]
    pub fn mel_tensor_length(&self) -> usize {
        self.input_mel_frames * self.n_mels
    }

    /// Get the expected backbone output length (seq_len × embedding_dim).
    #[wasm_bindgen(getter, js_name = "backboneOutputLength")]
    pub fn backbone_output_length(&self) -> usize {
        self.sequence_length * self.embedding_dim
    }
}

impl Default for WakerWasmDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_name = "trainTemporalConvHead")]
pub fn train_temporal_conv_head(
    flattened_sequences: &[f32],
    labels: &[u8],
    config_json: &str,
) -> Result<String, JsValue> {
    trainer::train_custom_head_artifact(flattened_sequences, labels, config_json)
        .map_err(|message| JsValue::from_str(&message))
}

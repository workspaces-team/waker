//! VAD WASM — Silero VAD v5 compiled to WebAssembly.
//!
//! Implements the complete Silero VAD v5 model as a bespoke forward pass in pure Rust.
//! No ONNX Runtime, no Tract — just hand-coded Conv1d, LSTM, and dense operations.
//!
//! ## Usage from JavaScript:
//! ```js
//! import init, { SileroVadDetector } from "./vad_wasm.js";
//!
//! await init();
//! const detector = new SileroVadDetector();
//! await detector.loadModel(weightsBinary, manifestJson);
//!
//! // Feed 512-sample chunks at 16kHz:
//! const prob = detector.process(audioChunk512);
//! if (prob > 0.5) { /* speech detected */ }
//! ```

// Public so integration tests in tests/ can access internals via the rlib.
pub mod model;
pub mod nn;
pub mod weights;

use wasm_bindgen::prelude::*;

/// Voice Activity Detection result.
#[wasm_bindgen]
pub struct VadResult {
    probability: f32,
    is_speech: bool,
}

#[wasm_bindgen]
impl VadResult {
    #[wasm_bindgen(getter)]
    pub fn probability(&self) -> f32 {
        self.probability
    }

    #[wasm_bindgen(getter, js_name = "isSpeech")]
    pub fn is_speech(&self) -> bool {
        self.is_speech
    }
}

/// Configuration for the VAD detector.
#[wasm_bindgen]
pub struct VadConfig {
    /// Speech probability threshold (default: 0.5).
    threshold: f32,
    /// Minimum number of consecutive speech chunks to confirm speech start.
    min_speech_chunks: u32,
    /// Minimum number of consecutive silence chunks to confirm speech end.
    min_silence_chunks: u32,
}

#[wasm_bindgen]
impl VadConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            threshold: 0.5,
            min_speech_chunks: 1,
            min_silence_chunks: 6,
        }
    }

    #[wasm_bindgen(js_name = "setThreshold")]
    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold.clamp(0.01, 0.99);
    }

    #[wasm_bindgen(js_name = "setMinSpeechChunks")]
    pub fn set_min_speech_chunks(&mut self, chunks: u32) {
        self.min_speech_chunks = chunks.max(1);
    }

    #[wasm_bindgen(js_name = "setMinSilenceChunks")]
    pub fn set_min_silence_chunks(&mut self, chunks: u32) {
        self.min_silence_chunks = chunks.max(1);
    }
}

impl Default for VadConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Speech segment tracking state.
struct SpeechTracker {
    in_speech: bool,
    speech_count: u32,
    silence_count: u32,
}

impl SpeechTracker {
    fn new() -> Self {
        Self {
            in_speech: false,
            speech_count: 0,
            silence_count: 0,
        }
    }

    fn update(&mut self, is_speech: bool, config: &VadConfig) -> SpeechEvent {
        if is_speech {
            self.speech_count += 1;
            self.silence_count = 0;

            if !self.in_speech && self.speech_count >= config.min_speech_chunks {
                self.in_speech = true;
                return SpeechEvent::SpeechStart;
            }
        } else {
            self.silence_count += 1;
            self.speech_count = 0;

            if self.in_speech && self.silence_count >= config.min_silence_chunks {
                self.in_speech = false;
                return SpeechEvent::SpeechEnd;
            }
        }

        if self.in_speech {
            SpeechEvent::Speaking
        } else {
            SpeechEvent::Silence
        }
    }

    fn reset(&mut self) {
        self.in_speech = false;
        self.speech_count = 0;
        self.silence_count = 0;
    }
}

/// Speech events returned to JS.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechEvent {
    Silence = 0,
    SpeechStart = 1,
    Speaking = 2,
    SpeechEnd = 3,
}

/// Extended result with speech event tracking.
#[wasm_bindgen]
pub struct VadEventResult {
    probability: f32,
    event: SpeechEvent,
}

#[wasm_bindgen]
impl VadEventResult {
    #[wasm_bindgen(getter)]
    pub fn probability(&self) -> f32 {
        self.probability
    }

    #[wasm_bindgen(getter)]
    pub fn event(&self) -> SpeechEvent {
        self.event
    }

    #[wasm_bindgen(getter, js_name = "isSpeech")]
    pub fn is_speech(&self) -> bool {
        matches!(self.event, SpeechEvent::SpeechStart | SpeechEvent::Speaking)
    }
}

/// The main WASM-based Silero VAD detector.
#[wasm_bindgen]
pub struct SileroVadDetector {
    weights: Option<weights::SileroWeights>,
    state: model::VadState,
    config: VadConfig,
    tracker: SpeechTracker,
    loaded: bool,
}

#[wasm_bindgen]
impl SileroVadDetector {
    /// Create a new VAD detector instance.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            weights: None,
            state: model::VadState::new(),
            config: VadConfig::new(),
            tracker: SpeechTracker::new(),
            loaded: false,
        }
    }

    /// Load model weights from the binary blob and manifest JSON.
    ///
    /// `weights_binary`: contents of `silero_vad_16k.bin`
    /// `manifest_json`: contents of `silero_vad_16k_manifest.json`
    #[wasm_bindgen(js_name = "loadModel")]
    pub fn load_model(
        &mut self,
        weights_binary: &[u8],
        manifest_json: &str,
    ) -> Result<(), JsValue> {
        let w = weights::SileroWeights::from_binary(weights_binary, manifest_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to load weights: {e}")))?;
        self.weights = Some(w);
        self.state.reset();
        self.tracker.reset();
        self.loaded = true;
        Ok(())
    }

    /// Configure the detector.
    #[wasm_bindgen(js_name = "configure")]
    pub fn configure(&mut self, config: VadConfig) {
        self.config = config;
    }

    /// Set the speech probability threshold.
    #[wasm_bindgen(js_name = "setThreshold")]
    pub fn set_threshold(&mut self, threshold: f32) {
        self.config.threshold = threshold.clamp(0.01, 0.99);
    }

    /// Process a 512-sample audio chunk (16 kHz, f32 PCM).
    ///
    /// Returns the speech probability.
    pub fn process(&mut self, chunk: &[f32]) -> Result<VadResult, JsValue> {
        if !self.loaded {
            return Err(JsValue::from_str("Model not loaded. Call loadModel() first."));
        }
        if chunk.len() != 512 {
            return Err(JsValue::from_str(&format!(
                "Expected 512 samples, got {}",
                chunk.len()
            )));
        }

        let weights = self.weights.as_ref().unwrap();
        let probability = model::forward(chunk, weights, &mut self.state);
        let is_speech = probability >= self.config.threshold;

        Ok(VadResult {
            probability,
            is_speech,
        })
    }

    /// Process a chunk and return speech event tracking (start/end/speaking/silence).
    #[wasm_bindgen(js_name = "processWithEvents")]
    pub fn process_with_events(&mut self, chunk: &[f32]) -> Result<VadEventResult, JsValue> {
        if !self.loaded {
            return Err(JsValue::from_str("Model not loaded. Call loadModel() first."));
        }
        if chunk.len() != 512 {
            return Err(JsValue::from_str(&format!(
                "Expected 512 samples, got {}",
                chunk.len()
            )));
        }

        let weights = self.weights.as_ref().unwrap();
        let probability = model::forward(chunk, weights, &mut self.state);
        let is_speech = probability >= self.config.threshold;
        let event = self.tracker.update(is_speech, &self.config);

        Ok(VadEventResult { probability, event })
    }

    /// Reset all state (LSTM hidden/cell, tracker, context buffer).
    pub fn reset(&mut self) {
        self.state.reset();
        self.tracker.reset();
    }

    /// Check if the model is loaded.
    #[wasm_bindgen(getter, js_name = "isLoaded")]
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Get the current speech threshold.
    #[wasm_bindgen(getter)]
    pub fn threshold(&self) -> f32 {
        self.config.threshold
    }

    /// Whether the detector currently considers the stream to be in a speech segment.
    #[wasm_bindgen(getter, js_name = "inSpeech")]
    pub fn in_speech(&self) -> bool {
        self.tracker.in_speech
    }
}

impl Default for SileroVadDetector {
    fn default() -> Self {
        Self::new()
    }
}

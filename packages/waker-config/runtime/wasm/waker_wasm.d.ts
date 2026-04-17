/* tslint:disable */
/* eslint-disable */

export class WakerDetectionResult {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The wake forms accepted under the active registration policy.
     *
     * Mirrors `WakerWebDetectionResult.acceptedWakeForms` in `@waker/sdk-web`.
     */
    readonly acceptedWakeForms: string[];
    readonly chosenWakeForm: string;
    readonly detected: boolean;
    readonly keyword: string;
    readonly score: number;
    readonly threshold: number;
}

/**
 * The main WASM-based wake-word detector.
 *
 * Handles the full audio → detection pipeline or accepts pre-computed backbone
 * embeddings for the detector head only.
 */
export class WakerWasmDetector {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Configure just the frontend/backbone path for browser-side embedding extraction.
     *
     * `runtime_backbone_json` should match the `runtimeBackbone` shape from detector.json.
     * This method seeds a no-op identity head so callers can reuse the same WASM package
     * for clip embedding and later for a trained detector.
     */
    configureBackbone(runtime_backbone_json: string, capture_sample_rate: number): void;
    /**
     * Run a fully buffered 16 kHz mono clip through the frontend and backbone and return
     * the flat embedding sequence `[sequenceLength * embeddingDim]`.
     */
    embedPcm16kClip(pcm16k: Float32Array): Float32Array;
    /**
     * Load backbone weights from the extracted binary blob and manifest.
     *
     * This enables the fully self-contained pipeline — no onnxruntime-web needed.
     *
     * `weights_binary`: contents of `backbone_16k.bin`
     * `manifest_json`: contents of `backbone_16k_manifest.json`
     */
    loadBackboneWeights(weights_binary: Uint8Array, manifest_json: string): void;
    /**
     * Load detector configuration from JSON strings.
     *
     * `registration_json`: contents of registration.json
     * `detector_json`: contents of detector.json
     * `capture_sample_rate`: the browser capture sample rate (typically 24000)
     */
    loadConfig(registration_json: string, detector_json: string, capture_sample_rate: number): void;
    /**
     * Create a new detector instance.
     */
    constructor();
    /**
     * Process a Mu-Law encoded audio chunk from the browser microphone.
     *
     * Returns a detection result once the ring buffer is full, or null if
     * the buffer is still filling. The backbone inference is handled externally
     * (by the JS ONNX runtime). Call `processBackboneOutput` instead when the
     * backbone output is available.
     *
     * This method handles: Mu-Law decode → resample → ring buffer → mel frontend.
     * It returns the mel spectrogram as a flat Float32Array for the JS side to
     * pass to the ONNX backbone.
     */
    processAudioToMel(chunk: Uint8Array): Float32Array | undefined;
    /**
     * Score a backbone embedding sequence through the detector head.
     *
     * `backbone_output`: flat Float32Array of shape [seq_len × embedding_dim]
     *     from the ONNX backbone inference on the JS side.
     * `now_ms`: current timestamp in milliseconds (from Date.now()).
     *
     * Returns the detection result with score, threshold, and detected flag.
     */
    processBackboneOutput(backbone_output: Float32Array, now_ms: number): WakerDetectionResult;
    /**
     * Process a Mu-Law encoded audio chunk through the **complete** pipeline.
     *
     * Mu-Law decode → resample → ring buffer → mel frontend → backbone → detector head → decision.
     *
     * **No onnxruntime-web needed.** Everything runs in WASM.
     *
     * Returns `None` if the ring buffer is still filling, or a `WakerDetectionResult`
     * once enough audio has been buffered.
     */
    processMuLawChunk(chunk: Uint8Array, now_ms: number): WakerDetectionResult | undefined;
    /**
     * Reset the detector state (ring buffer, decision state).
     */
    reset(): void;
    /**
     * Get the expected backbone output length (seq_len × embedding_dim).
     */
    readonly backboneOutputLength: number;
    /**
     * Check if the backbone weights are loaded.
     */
    readonly isBackboneLoaded: boolean;
    /**
     * Check if the detector is fully ready for the complete pipeline.
     */
    readonly isFullyReady: boolean;
    /**
     * Check if the detector is loaded and ready (config + backbone weights).
     */
    readonly isLoaded: boolean;
    /**
     * Get the number of mel features the frontend produces per chunk.
     */
    readonly melTensorLength: number;
}

export function trainTemporalConvHead(flattened_sequences: Float32Array, labels: Uint8Array, config_json: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wakerdetectionresult_free: (a: number, b: number) => void;
    readonly __wbg_wakerwasmdetector_free: (a: number, b: number) => void;
    readonly trainTemporalConvHead: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly wakerdetectionresult_acceptedWakeForms: (a: number) => [number, number];
    readonly wakerdetectionresult_chosenWakeForm: (a: number) => [number, number];
    readonly wakerdetectionresult_detected: (a: number) => number;
    readonly wakerdetectionresult_keyword: (a: number) => [number, number];
    readonly wakerdetectionresult_score: (a: number) => number;
    readonly wakerdetectionresult_threshold: (a: number) => number;
    readonly wakerwasmdetector_backboneOutputLength: (a: number) => number;
    readonly wakerwasmdetector_configureBackbone: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wakerwasmdetector_embedPcm16kClip: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wakerwasmdetector_isBackboneLoaded: (a: number) => number;
    readonly wakerwasmdetector_isFullyReady: (a: number) => number;
    readonly wakerwasmdetector_isLoaded: (a: number) => number;
    readonly wakerwasmdetector_loadBackboneWeights: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wakerwasmdetector_loadConfig: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wakerwasmdetector_melTensorLength: (a: number) => number;
    readonly wakerwasmdetector_new: () => number;
    readonly wakerwasmdetector_processAudioToMel: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wakerwasmdetector_processBackboneOutput: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly wakerwasmdetector_processMuLawChunk: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly wakerwasmdetector_reset: (a: number) => void;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_drop_slice: (a: number, b: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;

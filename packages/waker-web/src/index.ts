import {
  DEFAULT_CAPTURE_SAMPLE_RATE,
  fetchText,
  getBundledWakerRegistrationUrl,
  getBundledWakerWasmBinaryUrl,
  getBundledWakerWasmModuleUrl,
  loadBundledBackboneAssets,
  loadWakerWasmModule,
  resolveUrl,
  type WakerWasmDetectorShape,
} from "./runtime-common";
import type {
  WakerBrowserHeadTrainingConfig,
  WakerBundledRegistrationPolicy,
  WakerHeadArtifact,
  WakerHeadDetectorConfig,
  WakerHeadTrainingClipExample,
  WakerHeadTrainingEmbeddingExample,
  WakerHeadTrainingSummary,
  WakerRuntimeBackboneConfig,
  WakerWebDetectionResult,
  WakerWebRegistration,
} from "./types";

export type {
  WakerHeadDetectorConfig,
  WakerWebDetectionResult,
  WakerWebRegistration,
  WakerBundledRegistrationPolicy,
  WakerHeadArtifact,
  WakerBrowserHeadTrainingConfig,
  WakerHeadTrainingClipExample,
  WakerHeadTrainingEmbeddingExample,
  WakerHeadTrainingSummary,
  WakerRuntimeBackboneConfig,
} from "./types";
export { getBundledWakerRegistrationUrl, getBundledWakerRuntimeBasePath } from "./runtime-common";
export {
  createWakerWebCustomDetector,
  WakerWebCustomDetector,
} from "./custom-detector";

type WakerWebDetectorLoadOptions = {
  captureSampleRate?: number;
};

export interface WakerWebDetector {
  dispose(): void;
  getRegistration(): WakerWebRegistration | null;
  load(registrationUrl: string, options?: WakerWebDetectorLoadOptions): Promise<void>;
  processChunk(pcm16k: Float32Array): Promise<WakerWebDetectionResult | null>;
  processMuLawChunk(chunk: Uint8Array): Promise<WakerWebDetectionResult | null>;
  reset(): void;
}

export class WakerWebRuntimeDetector implements WakerWebDetector {
  private captureSampleRate = DEFAULT_CAPTURE_SAMPLE_RATE;
  private registration: WakerWebRegistration | null = null;
  private wasmDetector: WakerWasmDetectorShape | null = null;

  dispose(): void {
    this.registration = null;
    this.wasmDetector?.free?.();
    this.wasmDetector = null;
  }

  getRegistration(): WakerWebRegistration | null {
    return this.registration;
  }

  async load(registrationUrl: string, options: WakerWebDetectorLoadOptions = {}): Promise<void> {
    this.dispose();
    this.captureSampleRate = options.captureSampleRate ?? DEFAULT_CAPTURE_SAMPLE_RATE;

    const resolvedRegistrationUrl = resolveUrl(registrationUrl);
    const registrationJson = await fetchText(resolvedRegistrationUrl);
    const registration = JSON.parse(registrationJson) as WakerWebRegistration;
    const detectorJson = await fetchText(
      resolveUrl(registration.detectorConfigPath, resolvedRegistrationUrl),
    );
    const detector = JSON.parse(detectorJson) as WakerHeadDetectorConfig;
    const runtimeBaseUrl = resolveUrl("./", resolvedRegistrationUrl);
    const wasmModule = await loadWakerWasmModule(
      getBundledWakerWasmModuleUrl(runtimeBaseUrl),
      getBundledWakerWasmBinaryUrl(runtimeBaseUrl),
    );
    const wasmDetector = new wasmModule.WakerWasmDetector();
    const { weightsBinary, manifestJson } = await loadBundledBackboneAssets(
      runtimeBaseUrl,
      detector.runtimeBackbone ?? null,
    );

    wasmDetector.loadConfig(registrationJson, detectorJson, this.captureSampleRate);
    wasmDetector.loadBackboneWeights(weightsBinary, manifestJson);

    if (wasmDetector.isFullyReady === false || !wasmDetector.isLoaded) {
      throw new Error("Waker WASM detector failed to initialize.");
    }

    this.registration = registration;
    this.wasmDetector = wasmDetector;
    this.reset();
  }

  async processChunk(_pcm16k: Float32Array): Promise<WakerWebDetectionResult | null> {
    throw new Error(
      "processChunk(Float32Array) is not supported in the web runtime. Use processMuLawChunk(Uint8Array).",
    );
  }

  async processMuLawChunk(chunk: Uint8Array): Promise<WakerWebDetectionResult | null> {
    if (!this.registration || !this.wasmDetector) {
      throw new Error("Waker web detector is not loaded. Call load(registrationUrl) first.");
    }

    const result = this.wasmDetector.processMuLawChunk(chunk, Date.now());
    if (!result) {
      return null;
    }

    const webResult: WakerWebDetectionResult = {
      acceptedWakeForms: [...(result.acceptedWakeForms ?? this.registration.acceptedWakeForms ?? [])],
      chosenWakeForm: result.chosenWakeForm,
      detected: result.detected,
      keyword: result.keyword,
      score: result.score,
      threshold: result.threshold,
    };
    result.free?.();
    return webResult;
  }

  reset(): void {
    this.wasmDetector?.reset();
  }
}

export function createWakerWebDetector(): WakerWebDetector {
  return new WakerWebRuntimeDetector();
}

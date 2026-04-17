import {
  DEFAULT_CAPTURE_SAMPLE_RATE,
  getBundledWakerRuntimeBasePath,
  getBundledWakerWasmBinaryUrl,
  getBundledWakerWasmModuleUrl,
  loadHeadArtifactFromUrl,
  loadBundledBackboneAssets,
  loadWakerWasmModule,
  normalizeHeadArtifact,
  type WakerWasmDetectorShape,
} from "./runtime-common";
import type {
  WakerBundledRegistrationPolicy,
  WakerHeadArtifact,
  WakerWebDetectionResult,
  WakerWebRegistration,
} from "./types";

export type WakerCustomDetectorLoadOptions = {
  artifactBaseUrl?: string;
  basePath?: string;
  captureSampleRate?: number;
  policy?: WakerBundledRegistrationPolicy;
};

function policyForArtifact(
  artifact: WakerHeadArtifact,
  override?: WakerBundledRegistrationPolicy,
): WakerBundledRegistrationPolicy {
  if (override) {
    return override;
  }
  const registrationPolicy = artifact.registration.registrationPolicy;
  if (
    registrationPolicy === "single_word_only" ||
    registrationPolicy === "single_word_plus_prefix" ||
    registrationPolicy === "exact_only" ||
    registrationPolicy === "bare_plus_prefix"
  ) {
    return registrationPolicy;
  }
  return "single_word_only";
}

export class WakerWebCustomDetector {
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

  private async loadResolvedHead(
    artifact: WakerHeadArtifact,
    options: WakerCustomDetectorLoadOptions = {},
  ): Promise<void> {
    this.dispose();
    this.captureSampleRate = options.captureSampleRate ?? DEFAULT_CAPTURE_SAMPLE_RATE;
    const runtimeBaseUrl = getBundledWakerRuntimeBasePath(policyForArtifact(artifact, options.policy), {
      basePath: options.basePath,
    });
    const wasmModule = await loadWakerWasmModule(
      getBundledWakerWasmModuleUrl(runtimeBaseUrl),
      getBundledWakerWasmBinaryUrl(runtimeBaseUrl),
    );
    const wasmDetector = new wasmModule.WakerWasmDetector();
    const { weightsBinary, manifestJson } = await loadBundledBackboneAssets(
      runtimeBaseUrl,
      artifact.detector.runtimeBackbone ?? null,
    );
    const registrationJson = JSON.stringify(artifact.registration);
    const detectorJson = JSON.stringify(artifact.detector);

    wasmDetector.loadConfig(registrationJson, detectorJson, this.captureSampleRate);
    wasmDetector.loadBackboneWeights(weightsBinary, manifestJson);

    if (wasmDetector.isFullyReady === false || !wasmDetector.isLoaded) {
      throw new Error("Waker custom detector failed to initialize.");
    }

    this.registration = artifact.registration;
    this.wasmDetector = wasmDetector;
    this.reset();
  }

  async loadHead(
    input: string | WakerHeadArtifact,
    options: WakerCustomDetectorLoadOptions = {},
  ): Promise<void> {
    return this.loadResolvedHead(normalizeHeadArtifact(input), options);
  }

  async loadHeadFromUrl(
    artifactUrl: string,
    options: WakerCustomDetectorLoadOptions = {},
  ): Promise<void> {
    const artifact = await loadHeadArtifactFromUrl(artifactUrl, options.artifactBaseUrl);
    return this.loadResolvedHead(artifact, options);
  }

  async processChunk(_pcm16k: Float32Array): Promise<WakerWebDetectionResult | null> {
    throw new Error(
      "processChunk(Float32Array) is not supported in the web runtime. Use processMuLawChunk(Uint8Array).",
    );
  }

  async processMuLawChunk(chunk: Uint8Array): Promise<WakerWebDetectionResult | null> {
    if (!this.registration || !this.wasmDetector) {
      throw new Error(
        "Waker custom detector is not loaded. Call loadHead(...) or loadHeadFromUrl(...) first.",
      );
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

export function createWakerWebCustomDetector(): WakerWebCustomDetector {
  return new WakerWebCustomDetector();
}

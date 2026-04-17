import {
  DEFAULT_CAPTURE_SAMPLE_RATE,
  fetchText,
  getBundledWakerRuntimeBasePath,
  getBundledWakerWasmBinaryUrl,
  getBundledWakerWasmModuleUrl,
  loadBundledBackboneAssets,
  loadWakerWasmModule,
  resolveUrl,
  type WakerWasmDetectorShape,
  type WakerWasmModuleShape,
} from "./runtime-common";
import type {
  WakerBrowserHeadTrainingConfig,
  WakerBundledRegistrationPolicy,
  WakerHeadArtifact,
  WakerHeadTrainingClipExample,
  WakerHeadTrainingEmbeddingExample,
  WakerRuntimeBackboneConfig,
} from "./types";

type BundledRuntimeConfigShape = {
  backbone?: {
    clipDurationSeconds?: number;
    embeddingDim?: number;
    inputDim?: number;
    inputMelFrames?: number;
    modelPath?: string | null;
    packageManifestPath?: string | null;
    sampleRate?: number;
    sequenceLength?: number;
  };
};

export type WakerWebHeadTrainerLoadOptions = {
  basePath?: string;
  captureSampleRate?: number;
  policy?: WakerBundledRegistrationPolicy;
  runtimeBackbone?: WakerRuntimeBackboneConfig;
};

function toRuntimeBackboneConfig(
  runtimeConfig: BundledRuntimeConfigShape,
): WakerRuntimeBackboneConfig {
  return {
    clipDurationSeconds: runtimeConfig.backbone?.clipDurationSeconds,
    embeddingDim: runtimeConfig.backbone?.embeddingDim,
    inputDim: runtimeConfig.backbone?.inputDim,
    inputMelFrames: runtimeConfig.backbone?.inputMelFrames,
    modelPath: runtimeConfig.backbone?.modelPath ?? null,
    packageManifestPath: runtimeConfig.backbone?.packageManifestPath ?? null,
    sampleRate: runtimeConfig.backbone?.sampleRate,
    sequenceLength: runtimeConfig.backbone?.sequenceLength,
  };
}

function flattenEmbeddingExamples(
  examples: WakerHeadTrainingEmbeddingExample[],
): { flattened: Float32Array; labels: Uint8Array } {
  if (examples.length === 0) {
    throw new Error("At least one embedding example is required.");
  }
  const sampleLength = examples[0].embedding.length;
  if (sampleLength === 0) {
    throw new Error("Embedding examples must not be empty.");
  }
  const flattened = new Float32Array(examples.length * sampleLength);
  const labels = new Uint8Array(examples.length);
  for (const [index, example] of examples.entries()) {
    if (example.embedding.length !== sampleLength) {
      throw new Error("All embedding examples must have the same length.");
    }
    flattened.set(example.embedding, index * sampleLength);
    labels[index] = example.label;
  }
  return { flattened, labels };
}

export class WakerWebHeadTrainer {
  private captureSampleRate = DEFAULT_CAPTURE_SAMPLE_RATE;
  private runtimeBackbone: WakerRuntimeBackboneConfig | null = null;
  private wasmDetector: WakerWasmDetectorShape | null = null;
  private wasmModule: WakerWasmModuleShape | null = null;

  dispose(): void {
    this.runtimeBackbone = null;
    this.wasmDetector?.free?.();
    this.wasmDetector = null;
    this.wasmModule = null;
  }

  async load(options: WakerWebHeadTrainerLoadOptions = {}): Promise<void> {
    this.dispose();
    this.captureSampleRate = options.captureSampleRate ?? DEFAULT_CAPTURE_SAMPLE_RATE;
    const runtimeBaseUrl = getBundledWakerRuntimeBasePath(options.policy ?? "single_word_only", {
      basePath: options.basePath,
    });
    const runtimeBackbone =
      options.runtimeBackbone ??
      toRuntimeBackboneConfig(
        JSON.parse(
          await fetchText(resolveUrl("runtime-config.json", runtimeBaseUrl)),
        ) as BundledRuntimeConfigShape,
      );
    const wasmModule = await loadWakerWasmModule(
      getBundledWakerWasmModuleUrl(runtimeBaseUrl),
      getBundledWakerWasmBinaryUrl(runtimeBaseUrl),
    );
    const wasmDetector = new wasmModule.WakerWasmDetector();
    const { weightsBinary, manifestJson } = await loadBundledBackboneAssets(
      runtimeBaseUrl,
      runtimeBackbone,
    );

    wasmDetector.configureBackbone(JSON.stringify(runtimeBackbone), this.captureSampleRate);
    wasmDetector.loadBackboneWeights(weightsBinary, manifestJson);

    if (wasmDetector.isFullyReady === false || !wasmDetector.isLoaded) {
      throw new Error("Waker WASM head trainer failed to initialize.");
    }

    this.runtimeBackbone = runtimeBackbone;
    this.wasmModule = wasmModule;
    this.wasmDetector = wasmDetector;
  }

  embedPcm16kClip(pcm16k: Float32Array): Float32Array {
    if (!this.wasmDetector) {
      throw new Error("Waker head trainer is not loaded. Call load() first.");
    }
    return new Float32Array(this.wasmDetector.embedPcm16kClip(pcm16k));
  }

  trainFromEmbeddings(
    examples: WakerHeadTrainingEmbeddingExample[],
    config: WakerBrowserHeadTrainingConfig,
  ): WakerHeadArtifact {
    if (!this.wasmModule) {
      throw new Error("Waker head trainer is not loaded. Call load() first.");
    }
    const { flattened, labels } = flattenEmbeddingExamples(examples);
    const trainingConfig = {
      ...config,
      runtimeBackbone: config.runtimeBackbone ?? this.runtimeBackbone ?? undefined,
    };
    return JSON.parse(
      this.wasmModule.trainTemporalConvHead(
        flattened,
        labels,
        JSON.stringify(trainingConfig),
      ),
    ) as WakerHeadArtifact;
  }

  trainFromClips(
    examples: WakerHeadTrainingClipExample[],
    config: WakerBrowserHeadTrainingConfig,
  ): WakerHeadArtifact {
    const embeddingExamples = examples.map((example) => ({
      embedding: this.embedPcm16kClip(example.pcm16k),
      label: example.label,
    }));
    return this.trainFromEmbeddings(embeddingExamples, config);
  }
}

export function createWakerWebHeadTrainer(): WakerWebHeadTrainer {
  return new WakerWebHeadTrainer();
}

export function serializeWakerHeadArtifact(artifact: WakerHeadArtifact): string {
  return JSON.stringify(artifact, null, 2);
}

export function createWakerHeadArtifactBlob(artifact: WakerHeadArtifact): Blob {
  return new Blob([serializeWakerHeadArtifact(artifact)], {
    type: "application/json",
  });
}

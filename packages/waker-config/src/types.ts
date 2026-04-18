export type WakerBundledRegistrationPolicy = "single_word_only";

export interface WakerWebDetectionResult {
  acceptedWakeForms: string[];
  chosenWakeForm: string;
  detected: boolean;
  keyword: string;
  score: number;
  threshold: number;
}

export interface WakerWebRegistration {
  acceptedWakeForms: string[];
  chosenWakeForm: string;
  detectorConfigPath: string;
  normalizedWakeForm?: string | null;
  registrationId: string;
  registrationPolicy: string;
  requestedKeyword: string;
  siblingNegativeForms?: string[];
  structuralConfusables?: string[];
}

export interface WakerRuntimeBackboneConfig {
  clipDurationSeconds?: number;
  embeddingDim?: number;
  inputDim?: number;
  inputMelFrames?: number;
  modelPath?: string | null;
  packageManifestPath?: string | null;
  sampleRate?: number;
  sequenceLength?: number;
}

export interface WakerHeadDetectorHeadConfig {
  accelScale: number;
  classifierBias: number;
  classifierWeight: number[];
  dilations: number[];
  edgeScale: number;
  hiddenWidth: number;
  implementation: string;
  smoothScale: number;
}

export interface WakerHeadDetectorConfig {
  decisionPolicy?: {
    confirmationHits?: number;
    cooldownSeconds?: number;
    threshold?: number;
  };
  detectorFormat: string;
  embeddingDim?: number;
  head: WakerHeadDetectorHeadConfig;
  keyword: string;
  runtimeBackbone?: WakerRuntimeBackboneConfig | null;
  schemaVersion: number;
  sequenceLength?: number;
  temperature?: {
    temperature?: number | null;
    validationLoss?: number | null;
  } | null;
  wEffective: {
    data: number[];
    shape: [number, number];
  };
}

export interface WakerHeadTrainingSummary {
  epochs: number;
  exampleCount: number;
  featureDim: number;
  focalGamma: number;
  l2Reg: number;
  learningRate: number;
  negativeCount: number;
  negativeWeight: number;
  positiveCount: number;
  selectedTemperature: number;
  selectedThreshold: number;
  trainAccuracy: number;
  validationAccuracy?: number | null;
  validationSplit: number;
}

export interface WakerHeadArtifact {
  artifactFormat: string;
  detector: WakerHeadDetectorConfig;
  registration: WakerWebRegistration & {
    backboneModelPath?: string | null;
    backbonePackageManifestPath?: string | null;
    bundleManifestPath?: string | null;
    policyPath?: string | null;
    runtimeConfigPath?: string | null;
  };
  schemaVersion: number;
  training: WakerHeadTrainingSummary;
}

export interface WakerBrowserHeadTrainingConfig {
  acceptedWakeForms?: string[];
  confirmationHits?: number;
  cooldownSeconds?: number;
  detector?: {
    accelScale?: number;
    dilations?: number[];
    edgeScale?: number;
    hiddenWidth?: number;
    smoothScale?: number;
  };
  epochs?: number;
  focalGamma?: number;
  keyword: string;
  l2Reg?: number;
  learningRate?: number;
  negativeWeight?: number;
  registrationPolicy?: WakerBundledRegistrationPolicy | string;
  runtimeBackbone?: WakerRuntimeBackboneConfig;
  siblingNegativeForms?: string[];
  structuralConfusables?: string[];
  temperature?: number;
  threshold?: number;
  thresholdGrid?: number[];
  validationSplit?: number;
  wEffective?: {
    data: number[];
    shape: [number, number];
  };
}

export interface WakerHeadTrainingClipExample {
  label: 0 | 1;
  pcm16k: Float32Array;
}

export interface WakerHeadTrainingEmbeddingExample {
  embedding: Float32Array;
  label: 0 | 1;
}

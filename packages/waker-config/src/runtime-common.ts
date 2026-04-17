import type {
  WakerBundledRegistrationPolicy,
  WakerHeadArtifact,
  WakerRuntimeBackboneConfig,
} from "./types";

export const DEFAULT_CAPTURE_SAMPLE_RATE = 24_000;
const DEFAULT_BUNDLED_RUNTIME_BASE_PATH = "/waker-config/";

export type WakerWasmDetectionResultShape = {
  acceptedWakeForms?: string[];
  chosenWakeForm: string;
  detected: boolean;
  free?: () => void;
  keyword: string;
  score: number;
  threshold: number;
};

export type WakerWasmDetectorShape = {
  configureBackbone(runtimeBackboneJson: string, captureSampleRate: number): void;
  embedPcm16kClip(pcm16k: Float32Array): Float32Array;
  free?: () => void;
  isFullyReady?: boolean;
  isLoaded: boolean;
  loadBackboneWeights(weightsBinary: Uint8Array, manifestJson: string): void;
  loadConfig(registrationJson: string, detectorJson: string, captureSampleRate: number): void;
  processMuLawChunk(chunk: Uint8Array, nowMs: number): WakerWasmDetectionResultShape | undefined;
  reset(): void;
};

export type WakerWasmModuleShape = {
  default: (moduleOrPath?: unknown) => Promise<unknown>;
  WakerWasmDetector: new () => WakerWasmDetectorShape;
  trainTemporalConvHead(
    flattenedSequences: Float32Array,
    labels: Uint8Array,
    configJson: string,
  ): string;
};

export async function fetchText(url: string): Promise<string> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to load ${url}: ${response.status} ${response.statusText}`);
  }
  return response.text();
}

export async function fetchUint8Array(url: string): Promise<Uint8Array> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to load ${url}: ${response.status} ${response.statusText}`);
  }
  return new Uint8Array(await response.arrayBuffer());
}

function getDocumentBaseUrl(): string {
  if (typeof document !== "undefined" && document.baseURI) {
    return document.baseURI;
  }
  if (typeof window !== "undefined" && window.location?.href) {
    return window.location.href;
  }
  return "http://localhost/";
}

export function resolveUrl(relativeOrAbsolute: string, baseUrl?: string): string {
  return new URL(relativeOrAbsolute, baseUrl ?? getDocumentBaseUrl()).toString();
}

function normalizeRuntimeBasePath(basePath: string): string {
  const withLeadingSlash = basePath.startsWith("/") ? basePath : `/${basePath}`;
  return withLeadingSlash.endsWith("/") ? withLeadingSlash : `${withLeadingSlash}/`;
}

function bundledPolicyDirectory(policy: WakerBundledRegistrationPolicy): string {
  switch (policy) {
    case "single_word_only":
      return "single-word-only";
    case "single_word_plus_prefix":
      return "single-word-plus-prefix";
    case "exact_only":
      return "exact-only";
    case "bare_plus_prefix":
    default:
      return "bare-plus-prefix";
  }
}

export function getBundledWakerRuntimeBasePath(
  policy: WakerBundledRegistrationPolicy = "single_word_only",
  options: { basePath?: string } = {},
): string {
  const basePath = normalizeRuntimeBasePath(options.basePath ?? DEFAULT_BUNDLED_RUNTIME_BASE_PATH);
  return `${basePath}${bundledPolicyDirectory(policy)}/`;
}

export function getBundledWakerRegistrationUrl(
  policy: WakerBundledRegistrationPolicy = "single_word_only",
  options: { basePath?: string } = {},
): string {
  return `${getBundledWakerRuntimeBasePath(policy, options)}registration.json`;
}

export function getBundledWakerWasmModuleUrl(runtimeBaseUrl: string): string {
  return new URL("../wasm/waker_wasm.js", runtimeBaseUrl).toString();
}

export function getBundledWakerWasmBinaryUrl(runtimeBaseUrl: string): string {
  return new URL("../wasm/waker_wasm_bg.wasm", runtimeBaseUrl).toString();
}

export async function loadWakerWasmModule(
  moduleUrl: string,
  wasmBinaryUrl: string,
): Promise<WakerWasmModuleShape> {
  const module = (await import(/* @vite-ignore */ moduleUrl)) as WakerWasmModuleShape;
  await module.default({ module_or_path: wasmBinaryUrl });
  return module;
}

export async function loadBundledBackboneAssets(
  runtimeBaseUrl: string,
  runtimeBackbone?: WakerRuntimeBackboneConfig | null,
): Promise<{ manifestJson: string; weightsBinary: Uint8Array }> {
  const modelPath = runtimeBackbone?.modelPath ?? "backbone/model.bin";
  const packageManifestPath =
    runtimeBackbone?.packageManifestPath ?? "backbone/model_manifest.json";
  const [weightsBinary, manifestJson] = await Promise.all([
    fetchUint8Array(resolveUrl(modelPath, runtimeBaseUrl)),
    fetchText(resolveUrl(packageManifestPath, runtimeBaseUrl)),
  ]);
  return { weightsBinary, manifestJson };
}

export function normalizeHeadArtifact(input: string | WakerHeadArtifact): WakerHeadArtifact {
  return typeof input === "string" ? (JSON.parse(input) as WakerHeadArtifact) : input;
}

export async function loadHeadArtifactFromUrl(
  artifactUrl: string,
  baseUrl?: string,
): Promise<WakerHeadArtifact> {
  return normalizeHeadArtifact(await fetchText(resolveUrl(artifactUrl, baseUrl)));
}

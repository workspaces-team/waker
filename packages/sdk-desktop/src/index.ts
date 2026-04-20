import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";

import { localNativeCandidates, nativeOutputPath, platformTag } from "./platform";

const require = createRequire(import.meta.url);

export type WakerBundleFamily = "waker-web" | "waker-desktop" | "waker-micro";

export interface WakerBundleManifest {
  family: WakerBundleFamily;
  frontendContract: string;
  backboneContract: string;
  detectorContract: string;
  verifierContract?: string | null;
  detectorPath?: string;
  verifierPath?: string | null;
  decisionPolicyPath?: string;
  benchmarkSummaryPath?: string;
}

export interface WakerDesktopBundleManifest extends Omit<WakerBundleManifest, "family"> {
  family: "waker-desktop";
}

export interface WakerDesktopDetectionResult {
  detected: boolean;
  score: number;
}

export interface WakerDesktopDetector {
  load(bundleUrl: string): Promise<void>;
  processChunk(pcm: Float32Array): WakerDesktopDetectionResult;
  dispose(): void;
}

export interface WakerDesktopNativeBindingOptions {
  bindingPath?: string;
  binaryPath?: string;
}

type NativeDesktopBinding = {
  WakerDesktopNativeDetector: new () => NativeDesktopDetector;
};

type NativeDesktopDetector = {
  readonly isLoaded?: boolean;
  dispose(): void;
  load(bundleUrl: string): string;
  processChunk(pcm: Float32Array): WakerDesktopDetectionResult;
};

class WakerDesktopRuntime implements WakerDesktopDetector {
  private readonly native: NativeDesktopDetector;
  private bundleUrl: string | null = null;
  private manifest: WakerDesktopBundleManifest | null = null;

  constructor(options: WakerDesktopNativeBindingOptions = {}) {
    const binding = loadNativeBinding(options);
    this.native = new binding.WakerDesktopNativeDetector();
  }

  async load(bundleUrl: string): Promise<void> {
    this.bundleUrl = bundleUrl;
    this.manifest = validateBundleManifest(JSON.parse(this.native.load(bundleUrl)) as WakerBundleManifest);
  }

  processChunk(pcm: Float32Array): WakerDesktopDetectionResult {
    if (!this.bundleUrl || !this.manifest) {
      throw new Error("Waker desktop detector is not loaded. Call load(bundleUrl) first.");
    }
    return this.native.processChunk(pcm);
  }

  dispose(): void {
    this.native.dispose();
    this.bundleUrl = null;
    this.manifest = null;
  }
}

function ensureFileReadable(filePath: string): string {
  fs.accessSync(filePath, fs.constants.R_OK);
  return filePath;
}

function isLoadable(filePath: string): boolean {
  try {
    ensureFileReadable(filePath);
    return true;
  } catch {
    return false;
  }
}

function loadNativeBinding(options: WakerDesktopNativeBindingOptions = {}): NativeDesktopBinding {
  return require(resolveNativeBindingPath(options)) as NativeDesktopBinding;
}

export const DEFAULT_WAKER_DESKTOP_MANIFEST: WakerDesktopBundleManifest = {
  family: "waker-desktop",
  frontendContract: "waker-mel-frontend-v1",
  backboneContract: "waker-web-backbone-v1",
  detectorContract: "waker-detector-head-v1",
  verifierContract: null,
  detectorPath: "detector.onnx",
  verifierPath: null,
  decisionPolicyPath: "decision-policy.json",
  benchmarkSummaryPath: "benchmark-summary.json",
};

export function validateBundleManifest(manifest: WakerBundleManifest): WakerDesktopBundleManifest {
  if (manifest.family !== "waker-desktop") {
    throw new Error(`Waker desktop SDK requires a waker-desktop bundle, got ${manifest.family}`);
  }
  return manifest as WakerDesktopBundleManifest;
}

export function resolveNativeBindingPath(options: WakerDesktopNativeBindingOptions = {}): string {
  if (options.bindingPath || options.binaryPath) {
    return path.resolve(String(options.bindingPath ?? options.binaryPath));
  }

  for (const candidate of localNativeCandidates()) {
    if (isLoadable(candidate)) {
      return candidate;
    }
  }

  const packaged = nativeOutputPath();
  if (isLoadable(packaged)) {
    return packaged;
  }

  throw new Error(
    [
      `No @waker/sdk-desktop native binding was found for ${platformTag()}.`,
      `Expected a packaged native addon at ${nativeOutputPath()}.`,
      "For local repo development, run: pnpm --filter @waker/sdk-desktop run build:native:debug",
      "Rust app-facing deliverables in this repo use napi-rs with a thin TypeScript wrapper.",
    ].join("\n"),
  );
}

export function createWakerDesktopDetector(options: WakerDesktopNativeBindingOptions = {}): WakerDesktopDetector {
  return new WakerDesktopRuntime(options);
}

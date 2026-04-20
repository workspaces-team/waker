import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

type PolicyConfig = {
  cohortId: string;
  policy: "single_word_only";
  runtimeDir: string;
};

type RegistrationPayload = {
  backboneModelPath?: string;
  backbonePackageManifestPath?: string | null;
  bundleManifestPath?: string | null;
  detectorConfigPath: string;
  policyPath?: string | null;
  runtimeConfigPath?: string | null;
};

type RuntimeConfigPayload = {
  frontend?: {
    configPath?: string | null;
  } | null;
};

const packageRoot = resolve(import.meta.dirname, "..");
const repoRoot = resolve(packageRoot, "../../..");
const sdkRuntimeRoot = resolve(packageRoot, "runtime");
const wasmBackboneModelDir = resolve(repoRoot, "lib/extensions/sdk-wasm/model");

// Active product runtime assets are synced only from the retained single-word deliverable bundle.
const POLICY_CONFIGS: PolicyConfig[] = [
  {
    cohortId: "2026-04-16-cohort-adapter-official-first-run-single-word-registration-v1",
    policy: "single_word_only",
    runtimeDir: "single-word-only",
  },
];

function readJsonFile<T>(filePath: string): T {
  return JSON.parse(readFileSync(filePath, "utf8")) as T;
}

function writeJsonFile(filePath: string, payload: unknown) {
  writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

function latestBundleRootWithRegistration(cohortId: string): string | null {
  const currentRoot = resolve(repoRoot, "data/academy/deliverables", cohortId, "current");
  const currentRegistrationPath = resolve(currentRoot, "registration.json");
  if (existsSync(currentRegistrationPath)) {
    return currentRoot;
  }

  const runsRoot = resolve(repoRoot, "data/academy/deliverables", cohortId, "runs");
  if (!existsSync(runsRoot)) {
    return null;
  }

  const runs = readdirSync(runsRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => resolve(runsRoot, entry.name))
    .sort()
    .reverse();

  for (const runRoot of runs) {
    if (existsSync(resolve(runRoot, "registration.json"))) {
      return runRoot;
    }
  }

  return null;
}

function copyRelativeFile({
  destinationRoot,
  relativePath,
  sourceRoot,
}: {
  destinationRoot: string;
  relativePath: string;
  sourceRoot: string;
}) {
  const sourcePath = resolve(sourceRoot, relativePath);
  if (!existsSync(sourcePath)) {
    throw new Error(`Missing source runtime file: ${sourcePath}`);
  }
  const destinationPath = resolve(destinationRoot, relativePath);
  mkdirSync(dirname(destinationPath), { recursive: true });
  copyFileSync(sourcePath, destinationPath);
}

function installWasmBackboneCompatibility(destinationRoot: string) {
  const destinationBackboneRoot = resolve(destinationRoot, "backbone");
  mkdirSync(destinationBackboneRoot, { recursive: true });
  copyFileSync(resolve(wasmBackboneModelDir, "backbone_16k.bin"), resolve(destinationBackboneRoot, "model.bin"));
  copyFileSync(
    resolve(wasmBackboneModelDir, "backbone_16k_manifest.json"),
    resolve(destinationBackboneRoot, "model_manifest.json"),
  );
}

function rewriteRuntimeBackbonePaths(destinationRoot: string) {
  const wasmModelPath = "backbone/model.bin";
  const wasmManifestPath = "backbone/model_manifest.json";

  const registrationPath = resolve(destinationRoot, "registration.json");
  if (existsSync(registrationPath)) {
    const registration = readJsonFile<Record<string, unknown>>(registrationPath);
    registration.backboneModelPath = wasmModelPath;
    registration.backbonePackageManifestPath = wasmManifestPath;
    writeJsonFile(registrationPath, registration);
  }

  const runtimeConfigPath = resolve(destinationRoot, "runtime-config.json");
  if (existsSync(runtimeConfigPath)) {
    const runtimeConfig = readJsonFile<Record<string, unknown>>(runtimeConfigPath);
    const backbone =
      typeof runtimeConfig.backbone === "object" && runtimeConfig.backbone !== null
        ? { ...(runtimeConfig.backbone as Record<string, unknown>) }
        : {};
    backbone.modelPath = wasmModelPath;
    backbone.packageManifestPath = wasmManifestPath;
    runtimeConfig.backbone = backbone;
    writeJsonFile(runtimeConfigPath, runtimeConfig);
  }

  const bundleManifestPath = resolve(destinationRoot, "bundle.manifest.json");
  if (existsSync(bundleManifestPath)) {
    const bundleManifest = readJsonFile<Record<string, unknown>>(bundleManifestPath);
    const backbone =
      typeof bundleManifest.backbone === "object" && bundleManifest.backbone !== null
        ? { ...(bundleManifest.backbone as Record<string, unknown>) }
        : {};
    backbone.modelPath = wasmModelPath;
    backbone.packageManifestPath = wasmManifestPath;
    bundleManifest.backbone = backbone;
    writeJsonFile(bundleManifestPath, bundleManifest);
  }

  const registrationDir = resolve(destinationRoot, "registration");
  if (!existsSync(registrationDir)) return;
  for (const entry of readdirSync(registrationDir, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const detectorPath = resolve(registrationDir, entry.name, "detector.json");
    if (!existsSync(detectorPath)) continue;
    const detector = readJsonFile<Record<string, unknown>>(detectorPath);
    const runtimeBackbone =
      typeof detector.runtimeBackbone === "object" && detector.runtimeBackbone !== null
        ? { ...(detector.runtimeBackbone as Record<string, unknown>) }
        : {};
    runtimeBackbone.modelPath = wasmModelPath;
    runtimeBackbone.packageManifestPath = wasmManifestPath;
    detector.runtimeBackbone = runtimeBackbone;
    writeJsonFile(detectorPath, detector);
  }
}

function syncPolicyRuntime(config: PolicyConfig) {
  const sourceRoot = latestBundleRootWithRegistration(config.cohortId);
  if (!sourceRoot || !existsSync(sourceRoot)) {
    console.warn(`Skipping sdk-web runtime sync for ${config.policy}: missing bundle at ${sourceRoot}`);
    return;
  }

  const registrationEntryPath = resolve(sourceRoot, "registration.json");
  if (!existsSync(registrationEntryPath)) {
    console.warn(
      `Skipping sdk-web runtime sync for ${config.policy}: missing registration at ${registrationEntryPath}`,
    );
    return;
  }

  const destinationRoot = resolve(sdkRuntimeRoot, config.runtimeDir);
  rmSync(destinationRoot, { force: true, recursive: true });

  const registration = readJsonFile<RegistrationPayload>(registrationEntryPath);
  const runtimeConfigPath = registration.runtimeConfigPath ?? "runtime-config.json";
  const runtimeConfig = readJsonFile<RuntimeConfigPayload>(resolve(sourceRoot, runtimeConfigPath));

  const relativePaths = new Set<string>([
    "registration.json",
    registration.detectorConfigPath,
    registration.backboneModelPath ?? "backbone/model.onnx",
    runtimeConfigPath,
  ]);

  if (registration.backbonePackageManifestPath) {
    relativePaths.add(registration.backbonePackageManifestPath);
  }
  if (registration.bundleManifestPath) {
    relativePaths.add(registration.bundleManifestPath);
  }
  if (registration.policyPath) {
    relativePaths.add(registration.policyPath);
  }
  if (runtimeConfig.frontend?.configPath) {
    relativePaths.add(runtimeConfig.frontend.configPath);
  }

  const backboneDataCandidate = `${dirname(registration.backboneModelPath ?? "backbone/model.onnx")}/model.onnx.data`;
  if (existsSync(resolve(sourceRoot, backboneDataCandidate))) {
    relativePaths.add(backboneDataCandidate);
  }

  for (const relativePath of relativePaths) {
    copyRelativeFile({ destinationRoot, relativePath, sourceRoot });
  }

  installWasmBackboneCompatibility(destinationRoot);
  rewriteRuntimeBackbonePaths(destinationRoot);

  console.log(
    [
      `Synced sdk-web runtime assets`,
      `policy: ${config.policy}`,
      `source: ${sourceRoot}`,
      `destination: ${destinationRoot}`,
    ].join("\n"),
  );
}

for (const config of POLICY_CONFIGS) {
  syncPolicyRuntime(config);
}

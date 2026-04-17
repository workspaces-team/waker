import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, resolve } from "node:path";

type PolicyConfig = {
  cohortId: string;
  policy: "single_word_only" | "single_word_plus_prefix";
  runtimeDir: string;
};

type RuntimeAssetsManifest = {
  compatibilityBackbone: {
    destinationManifestPath: string;
    destinationModelPath: string;
    manifestJson: string;
    weightsBinary: string;
  };
  policies: PolicyConfig[];
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
const repoRoot = resolve(packageRoot, process.env.WAKER_SOURCE_REPO ?? "../../../waker");
const runtimeRoot = resolve(packageRoot, "runtime");
const runtimeAssetsManifest = JSON.parse(
  readFileSync(resolve(packageRoot, "runtime-assets.manifest.json"), "utf8"),
) as RuntimeAssetsManifest;
const wasmBackboneWeightsPath = resolve(repoRoot, runtimeAssetsManifest.compatibilityBackbone.weightsBinary);
const wasmBackboneManifestPath = resolve(repoRoot, runtimeAssetsManifest.compatibilityBackbone.manifestJson);

function readJsonFile<T>(filePath: string): T {
  return JSON.parse(readFileSync(filePath, "utf8")) as T;
}

function writeJsonFile(filePath: string, payload: unknown) {
  writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

function latestBundleRootWithRegistration(cohortId: string): string | null {
  const currentRoot = resolve(repoRoot, "data/academy/deliverables", cohortId, "current");
  if (existsSync(resolve(currentRoot, "registration.json"))) {
    return currentRoot;
  }
  return null;
}

function copyRelativeFile(sourceRoot: string, destinationRoot: string, relativePath: string) {
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
  copyFileSync(
    wasmBackboneWeightsPath,
    resolve(
      destinationBackboneRoot,
      runtimeAssetsManifest.compatibilityBackbone.destinationModelPath.split("/").pop() ?? "model.bin",
    ),
  );
  copyFileSync(
    wasmBackboneManifestPath,
    resolve(
      destinationBackboneRoot,
      runtimeAssetsManifest.compatibilityBackbone.destinationManifestPath.split("/").pop() ??
        "model_manifest.json",
    ),
  );
}

function rewriteRuntimeBackbonePaths(destinationRoot: string) {
  const wasmModelPath = runtimeAssetsManifest.compatibilityBackbone.destinationModelPath;
  const wasmManifestPath = runtimeAssetsManifest.compatibilityBackbone.destinationManifestPath;

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
  if (!sourceRoot) {
    console.warn(`Skipping waker-config runtime sync for ${config.policy}: missing current registration bundle`);
    return;
  }

  const destinationRoot = resolve(runtimeRoot, config.runtimeDir);
  rmSync(destinationRoot, { force: true, recursive: true });

  const registration = readJsonFile<RegistrationPayload>(resolve(sourceRoot, "registration.json"));
  const runtimeConfigPath = registration.runtimeConfigPath ?? "runtime-config.json";
  const runtimeConfig = readJsonFile<RuntimeConfigPayload>(resolve(sourceRoot, runtimeConfigPath));

  const relativePaths = new Set<string>([
    "registration.json",
    registration.detectorConfigPath,
    runtimeConfigPath,
  ]);

  if (registration.bundleManifestPath) relativePaths.add(registration.bundleManifestPath);
  if (registration.policyPath) relativePaths.add(registration.policyPath);
  if (runtimeConfig.frontend?.configPath) relativePaths.add(runtimeConfig.frontend.configPath);

  for (const relativePath of relativePaths) {
    copyRelativeFile(sourceRoot, destinationRoot, relativePath);
  }

  installWasmBackboneCompatibility(destinationRoot);
  rewriteRuntimeBackbonePaths(destinationRoot);

  console.log(
    [
      `Synced waker-config runtime assets`,
      `policy: ${config.policy}`,
      `source: ${sourceRoot}`,
      `destination: ${destinationRoot}`,
    ].join("\n"),
  );
}

for (const config of runtimeAssetsManifest.policies) {
  syncPolicyRuntime(config);
}

for (const entry of readdirSync(runtimeRoot, { withFileTypes: true })) {
  if (!entry.isDirectory()) continue;
  if (entry.name === "wasm") continue;
  if (runtimeAssetsManifest.policies.some((policy) => policy.runtimeDir === entry.name)) continue;
  rmSync(resolve(runtimeRoot, entry.name), { recursive: true, force: true });
}

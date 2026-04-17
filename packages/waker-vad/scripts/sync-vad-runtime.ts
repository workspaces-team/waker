import { copyFileSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { resolve } from "node:path";

const packageRoot = resolve(import.meta.dirname, "..");
const sourceRootCandidates = [
  resolve(packageRoot, "../../rust/vad-wasm"),
  process.env.WAKER_SOURCE_REPO ? resolve(process.env.WAKER_SOURCE_REPO) : null,
  resolve(packageRoot, "../../../../"),
  resolve(packageRoot, "../../../waker"),
].filter((value): value is string => Boolean(value));
const repoRoot = sourceRootCandidates.find((candidateRoot) =>
  existsSync(resolve(candidateRoot, "pkg")) ||
  existsSync(resolve(candidateRoot, "lib/extensions/vad-wasm/pkg")),
);
const localSourceRoot = repoRoot ? resolve(repoRoot) : null;
const vadPkgDir = localSourceRoot && existsSync(resolve(localSourceRoot, "pkg"))
  ? resolve(localSourceRoot, "pkg")
  : resolve(repoRoot, "lib/extensions/vad-wasm/pkg");
const vadModelDir = localSourceRoot && existsSync(resolve(localSourceRoot, "model"))
  ? resolve(localSourceRoot, "model")
  : resolve(repoRoot, "lib/extensions/vad-wasm/model");
const runtimeVadDir = resolve(packageRoot, "runtime/vad");

const PKG_FILES = [
  "vad_wasm_bg.wasm",
  "vad_wasm.js",
  "vad_wasm.d.ts",
];

const MODEL_FILES = [
  "silero_vad_16k.bin",
  "silero_vad_16k_manifest.json",
];

if (!repoRoot || !existsSync(vadPkgDir)) {
  console.error(
    [
      "vad-wasm pkg directory not found.",
      "Set WAKER_SOURCE_REPO to a compatible private workspace or run this package from the managed monorepo checkout.",
      `Checked roots: ${sourceRootCandidates.join(", ")}`,
    ].join("\n"),
  );
  process.exit(1);
}

if (!existsSync(vadModelDir)) {
  console.error(`vad-wasm model directory not found: ${vadModelDir}`);
  process.exit(1);
}

rmSync(runtimeVadDir, { force: true, recursive: true });
mkdirSync(runtimeVadDir, { recursive: true });

for (const file of PKG_FILES) {
  const src = resolve(vadPkgDir, file);
  if (!existsSync(src)) {
    console.error(`Missing expected wasm-pack output: ${src}`);
    process.exit(1);
  }
  copyFileSync(src, resolve(runtimeVadDir, file));
}

for (const file of MODEL_FILES) {
  const src = resolve(vadModelDir, file);
  if (!existsSync(src)) {
    console.error(`Missing model file: ${src}`);
    process.exit(1);
  }
  copyFileSync(src, resolve(runtimeVadDir, file));
}

console.log(
  [
    `Synced vad-wasm package into waker-vad runtime`,
    `source pkg: ${vadPkgDir}`,
    `source model: ${vadModelDir}`,
    `destination: ${runtimeVadDir}`,
    `files: ${[...PKG_FILES, ...MODEL_FILES].join(", ")}`,
  ].join("\n"),
);

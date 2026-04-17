/**
 * Sync the compiled VAD WASM binary, JS glue, and model weights from vad-wasm/pkg
 * and vad-wasm/model into waker-web/runtime/vad/.
 *
 * Run after `pnpm run vad-wasm:build:release`:
 *   pnpm run waker-web:sync:vad-wasm
 *
 * The Vite plugin (waker-web/src/vite.ts) serves the vad/ directory at:
 *   /waker-sdk-web/vad/vad_wasm_bg.wasm
 *   /waker-sdk-web/vad/vad_wasm.js
 *   /waker-sdk-web/vad/silero_vad_16k.bin
 *   /waker-sdk-web/vad/silero_vad_16k_manifest.json
 */

import { copyFileSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { resolve } from "node:path";

const packageRoot = resolve(import.meta.dirname, "..");
const repoRoot = resolve(packageRoot, process.env.WAKER_SOURCE_REPO ?? "../../../waker");
const vadPkgDir = resolve(repoRoot, "lib/extensions/vad-wasm/pkg");
const vadModelDir = resolve(repoRoot, "lib/extensions/vad-wasm/model");
const wakerWebVadDir = resolve(packageRoot, "runtime/vad");

const PKG_FILES = [
  "vad_wasm_bg.wasm",
  "vad_wasm.js",
  "vad_wasm.d.ts",
];

const MODEL_FILES = [
  "silero_vad_16k.bin",
  "silero_vad_16k_manifest.json",
];

if (!existsSync(vadPkgDir)) {
  console.error(
    `vad-wasm pkg directory not found: ${vadPkgDir}\n` +
      `Run 'pnpm run vad-wasm:build:release' first.`,
  );
  process.exit(1);
}

if (!existsSync(vadModelDir)) {
  console.error(`vad-wasm model directory not found: ${vadModelDir}`);
  process.exit(1);
}

rmSync(wakerWebVadDir, { force: true, recursive: true });
mkdirSync(wakerWebVadDir, { recursive: true });

for (const file of PKG_FILES) {
  const src = resolve(vadPkgDir, file);
  if (!existsSync(src)) {
    console.error(`Missing expected wasm-pack output: ${src}`);
    process.exit(1);
  }
  copyFileSync(src, resolve(wakerWebVadDir, file));
}

for (const file of MODEL_FILES) {
  const src = resolve(vadModelDir, file);
  if (!existsSync(src)) {
    console.error(`Missing model file: ${src}`);
    process.exit(1);
  }
  copyFileSync(src, resolve(wakerWebVadDir, file));
}

console.log(
  [
    `Synced vad-wasm package into waker-web runtime`,
    `source pkg: ${vadPkgDir}`,
    `source model: ${vadModelDir}`,
    `destination: ${wakerWebVadDir}`,
    `files: ${[...PKG_FILES, ...MODEL_FILES].join(", ")}`,
  ].join("\n"),
);

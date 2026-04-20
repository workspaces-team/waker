/**
 * Sync the compiled WASM binary and JS glue from sdk-wasm/pkg into sdk-web/runtime/wasm/.
 *
 * Run after `pnpm run sdk-wasm:build:release`:
 *   pnpm run sdk-web:sync:wasm
 *
 * The Vite plugin (sdk-web/src/vite.ts) serves the wasm/ directory at:
 *   /waker-sdk-web/wasm/waker_wasm_bg.wasm
 *   /waker-sdk-web/wasm/waker_wasm.js
 */

import { copyFileSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { resolve } from "node:path";

const repoRoot = resolve(import.meta.dirname, "..");
const wasmPkgDir = resolve(repoRoot, "lib/extensions/sdk-wasm/pkg");
const sdkWebWasmDir = resolve(repoRoot, "lib/extensions/sdk-web/runtime/wasm");

const FILES_TO_SYNC = ["waker_wasm_bg.wasm", "waker_wasm.js", "waker_wasm.d.ts"];

if (!existsSync(wasmPkgDir)) {
  console.error(`sdk-wasm pkg directory not found: ${wasmPkgDir}\nRun 'pnpm run sdk-wasm:build:release' first.`);
  process.exit(1);
}

rmSync(sdkWebWasmDir, { force: true, recursive: true });
mkdirSync(sdkWebWasmDir, { recursive: true });

for (const file of FILES_TO_SYNC) {
  const src = resolve(wasmPkgDir, file);
  if (!existsSync(src)) {
    console.error(`Missing expected wasm-pack output: ${src}`);
    process.exit(1);
  }
  copyFileSync(src, resolve(sdkWebWasmDir, file));
}

console.log(
  [
    `Synced sdk-wasm package into sdk-web runtime`,
    `source: ${wasmPkgDir}`,
    `destination: ${sdkWebWasmDir}`,
    `files: ${FILES_TO_SYNC.join(", ")}`,
  ].join("\n"),
);

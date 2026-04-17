/**
 * Sync the compiled WASM binary and JS glue from sdk-wasm/pkg into waker-web/runtime/wasm/.
 *
 * Run after `pnpm run sdk-wasm:build:release`:
 *   pnpm run waker-web:sync:wasm
 *
 * The Vite plugin (waker-web/src/vite.ts) serves the wasm/ directory at:
 *   /waker-sdk-web/wasm/waker_wasm_bg.wasm
 *   /waker-sdk-web/wasm/waker_wasm.js
 */

import { copyFileSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { resolve } from "node:path";

const packageRoot = resolve(import.meta.dirname, "..");
const repoRoot = resolve(packageRoot, process.env.WAKER_SOURCE_REPO ?? "../../../waker");
const wasmPkgDir = resolve(repoRoot, "lib/extensions/sdk-wasm/pkg");
const wakerWebWasmDir = resolve(packageRoot, "runtime/wasm");

const FILES_TO_SYNC = [
  "waker_wasm_bg.wasm",
  "waker_wasm.js",
  "waker_wasm.d.ts",
];

if (!existsSync(wasmPkgDir)) {
  console.error(
    `sdk-wasm pkg directory not found: ${wasmPkgDir}\n` +
      `Run 'pnpm run sdk-wasm:build:release' first.`,
  );
  process.exit(1);
}

rmSync(wakerWebWasmDir, { force: true, recursive: true });
mkdirSync(wakerWebWasmDir, { recursive: true });

for (const file of FILES_TO_SYNC) {
  const src = resolve(wasmPkgDir, file);
  if (!existsSync(src)) {
    console.error(`Missing expected wasm-pack output: ${src}`);
    process.exit(1);
  }
  copyFileSync(src, resolve(wakerWebWasmDir, file));
}

console.log(
  [
    `Synced sdk-wasm package into waker-web runtime`,
    `source: ${wasmPkgDir}`,
    `destination: ${wakerWebWasmDir}`,
    `files: ${FILES_TO_SYNC.join(", ")}`,
  ].join("\n"),
);

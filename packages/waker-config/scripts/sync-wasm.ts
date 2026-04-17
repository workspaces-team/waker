import { copyFileSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { resolve } from "node:path";

const packageRoot = resolve(import.meta.dirname, "..");
const repoRoot = resolve(packageRoot, process.env.WAKER_SOURCE_REPO ?? "../../../waker");
const wasmPkgDir = resolve(repoRoot, "lib/extensions/sdk-wasm/pkg");
const wasmRuntimeDir = resolve(packageRoot, "runtime/wasm");

const files = ["waker_wasm_bg.wasm", "waker_wasm.js", "waker_wasm.d.ts"];

if (!existsSync(wasmPkgDir)) {
  console.error(`sdk-wasm pkg directory not found: ${wasmPkgDir}`);
  process.exit(1);
}

rmSync(wasmRuntimeDir, { force: true, recursive: true });
mkdirSync(wasmRuntimeDir, { recursive: true });

for (const file of files) {
  copyFileSync(resolve(wasmPkgDir, file), resolve(wasmRuntimeDir, file));
}

console.log(
  [
    `Synced sdk-wasm package into waker-config runtime`,
    `source: ${wasmPkgDir}`,
    `destination: ${wasmRuntimeDir}`,
    `files: ${files.join(", ")}`,
  ].join("\n"),
);

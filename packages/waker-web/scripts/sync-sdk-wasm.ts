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
const sourceRootCandidates = [
  resolve(packageRoot, "../../rust/sdk-wasm"),
  process.env.WAKER_SOURCE_REPO ? resolve(process.env.WAKER_SOURCE_REPO) : null,
  resolve(packageRoot, "../../../../"),
  resolve(packageRoot, "../../../waker"),
].filter((value): value is string => Boolean(value));
const repoRoot = sourceRootCandidates.find((candidateRoot) =>
  existsSync(resolve(candidateRoot, "pkg")) ||
  existsSync(resolve(candidateRoot, "lib/extensions/sdk-wasm/pkg")),
);
const localSourceRoot = repoRoot ? resolve(repoRoot) : null;
const wasmPkgDir = localSourceRoot && existsSync(resolve(localSourceRoot, "pkg"))
  ? resolve(localSourceRoot, "pkg")
  : resolve(repoRoot, "lib/extensions/sdk-wasm/pkg");
const wakerWebWasmDir = resolve(packageRoot, "runtime/wasm");

const FILES_TO_SYNC = [
  "waker_wasm_bg.wasm",
  "waker_wasm.js",
  "waker_wasm.d.ts",
];

if (!repoRoot || !existsSync(wasmPkgDir)) {
  console.error(
    [
      "sdk-wasm pkg directory not found.",
      "Set WAKER_SOURCE_REPO to a compatible private workspace or run this package from the managed monorepo checkout.",
      `Checked roots: ${sourceRootCandidates.join(", ")}`,
    ].join("\n"),
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

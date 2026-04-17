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
const wasmRuntimeDir = resolve(packageRoot, "runtime/wasm");

const files = ["waker_wasm_bg.wasm", "waker_wasm.js", "waker_wasm.d.ts"];

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

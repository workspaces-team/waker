# @workspaces-team/waker-vad

[![npm version](https://img.shields.io/npm/v/%40workspaces-team%2Fwaker-vad?color=111827&label=npm)](https://www.npmjs.com/package/@workspaces-team/waker-vad)
[![CI](https://github.com/workspaces-team/waker/actions/workflows/ci.yml/badge.svg)](https://github.com/workspaces-team/waker/actions/workflows/ci.yml)

Browser-side VAD runtime assets for Waker.

This package carries the public VAD payload separately from `@workspaces-team/waker-web`:

- Silero VAD weights
- VAD WASM binary and JS glue
- Vite runtime asset plugin

Website: `https://waker.live`  
Issues: `https://github.com/workspaces-team/waker/issues`

Why use it:

- keep speech activity detection as a standalone browser asset package
- ship smaller detection bundles when an app needs VAD and wake logic separately
- mount the VAD runtime in Vite without pulling wake-specific code into the same package

## Install

```bash
npm install @workspaces-team/waker-vad
```

For Vite projects:

```bash
npm install @workspaces-team/waker-vad vite
```

## Runtime Assets

The generated payload under `runtime/vad/*` is emitted during `prepack` / `npm publish` and is not
committed to git.

The published runtime includes:

- `runtime/vad/vad_wasm.js`
- `runtime/vad/vad_wasm_bg.wasm`
- `runtime/vad/silero_vad_16k.bin`
- `runtime/vad/silero_vad_16k_manifest.json`

## Vite Integration

```ts
import { defineConfig } from "vite";
import { wakerVadRuntimeAssetsPlugin } from "@workspaces-team/waker-vad/vite";

export default defineConfig({
  plugins: [wakerVadRuntimeAssetsPlugin()],
});
```

The default runtime mount base is:

```text
/waker-vad/
```

## Asset URLs

```ts
import {
  getBundledWakerVadManifestUrl,
  getBundledWakerVadWeightsUrl,
  getBundledWakerVadWasmBinaryUrl,
  getBundledWakerVadWasmModuleUrl,
} from "@workspaces-team/waker-vad";
```

## Maintainer Refresh Flow

```bash
pnpm install
pnpm run sync:vad-wasm:source
pnpm --filter @workspaces-team/waker-vad run sync:runtime
npm pack --dry-run
```

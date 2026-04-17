# @workspaces-team/waker-web

[![npm version](https://img.shields.io/npm/v/%40workspaces-team%2Fwaker-web?color=111827&label=npm)](https://www.npmjs.com/package/@workspaces-team/waker-web)
[![CI](https://github.com/workspaces-team/waker/actions/workflows/ci.yml/badge.svg)](https://github.com/workspaces-team/waker/actions/workflows/ci.yml)

Single-word browser wake-word detection runtime for Waker.

This is the runtime-side package in the Waker web stack. It loads the bundled detector surface,
streams browser audio into the detector, and can also load custom trained heads produced by
[`@workspaces-team/waker-config`](https://github.com/workspaces-team/waker/tree/main/packages/waker-config#readme).

Website: `https://waker.live`  
Issues: `https://github.com/workspaces-team/waker/issues`

Why use it:

- keep single-word wake detection close to the interface that needs to respond
- ship a compact browser runtime with mirrored Rust/WASM internals hidden behind a small TS surface
- load custom trained heads from `@workspaces-team/waker-config` without changing the detector API

## Install

```bash
npm install @workspaces-team/waker-web
```

For Vite projects:

```bash
npm install @workspaces-team/waker-web vite
```

## Quick Start

```ts
import {
  createWakerWebDetector,
  getBundledWakerRegistrationUrl,
} from "@workspaces-team/waker-web";

const detector = createWakerWebDetector();
await detector.load(getBundledWakerRegistrationUrl("single_word_only"));
```

The active browser runtime is Mu-Law streaming first:

```ts
await detector.processMuLawChunk(chunk);
```

`processChunk(Float32Array)` is intentionally not part of this browser runtime surface.

## What You Get

- bundled single-word detector runtime
- custom trained-head loading
- Vite runtime asset plugin
- vendored runtime assets for the active single-word surface

If you need browser VAD assets, use [`@workspaces-team/waker-vad`](https://github.com/workspaces-team/waker/tree/main/packages/waker-vad#readme).

## Custom Trained Heads

```ts
import { createWakerWebCustomDetector } from "@workspaces-team/waker-web/custom-detector";

const detector = createWakerWebCustomDetector();
await detector.loadHead(artifactJson);
```

## Vite Integration

```ts
import { wakerWebRuntimeAssetsPlugin } from "@workspaces-team/waker-web/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [wakerWebRuntimeAssetsPlugin()],
});
```

The default runtime mount base is:

```text
/waker-web/
```

Minimal runtime loading with the default mount base:

```ts
import {
  createWakerWebDetector,
  getBundledWakerRegistrationUrl,
} from "@workspaces-team/waker-web";

const detector = createWakerWebDetector();
await detector.load(getBundledWakerRegistrationUrl("single_word_only", {
  basePath: "/waker-web/",
}));
```

## Bundled Policy Surface

- `single_word_only`

## Runtime Assets

This package tracks:

- `runtime/single-word-only/*`

The generated payload under `runtime/wasm/*` is emitted during `prepack` / `npm publish` and is
not committed to git.

The published runtime includes:

- `runtime/wasm/waker_wasm.js`
- `runtime/wasm/waker_wasm_bg.wasm`
- `runtime/single-word-only/backbone/model.bin`
- `runtime/single-word-only/backbone/model_manifest.json`
- `runtime/single-word-only/runtime-config.json`
- `runtime/single-word-only/registration.json`
- `runtime/single-word-only/registration/<keyword>/detector.json`

Source-of-truth asset manifest:

- [runtime-assets.manifest.json](https://github.com/workspaces-team/waker/blob/main/packages/waker-web/runtime-assets.manifest.json)

## Deployment Notes

- Keep the runtime files together. `registration.json` assumes stable relative paths to detector config and backbone weights.
- Serve `.wasm` with `application/wasm`.
- Use the Vite plugin when possible so dev and build output share the same asset layout.
- If you change the Vite mount base, pass the same `basePath` to `getBundledWakerRegistrationUrl(...)`.
- Preserve cross-origin isolation headers where your deployment depends on them.
- Do not assume any bundled policy other than `single_word_only`.

## Maintainer Refresh Flow

```bash
pnpm install
pnpm run sync:sdk-wasm:source
pnpm run sync:runtime-assets
npm pack --dry-run
```

`npm pack` builds the generated runtime payload from mirrored `rust/sdk-wasm/`.

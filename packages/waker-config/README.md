# @workspaces-team/waker-config

[![npm version](https://img.shields.io/npm/v/%40workspaces-team%2Fwaker-config?color=111827&label=npm)](https://www.npmjs.com/package/@workspaces-team/waker-config)
[![CI](https://github.com/workspaces-team/waker/actions/workflows/ci.yml/badge.svg)](https://github.com/workspaces-team/waker/actions/workflows/ci.yml)

Single-word browser wake-word training, config generation, and artifact utilities for Waker.

This is the training-side package in the Waker web stack. It builds starter configs, trains browser
tiny heads, and serializes the resulting artifacts for later use in
[`@workspaces-team/waker-web`](https://github.com/workspaces-team/waker/tree/main/packages/waker-web#readme).

Website: `https://waker.live`  
Issues: `https://github.com/workspaces-team/waker/issues`

Why use it:

- train single-word wake targets without standing up a hosted training stack
- keep artifact generation in the same browser environment that will consume the runtime
- export portable tiny-head artifacts that can move cleanly into product surfaces

## Install

```bash
npm install @workspaces-team/waker-config
```

For Vite projects:

```bash
npm install @workspaces-team/waker-config vite
```

## Quick Start

Generate a starter single-word config:

```bash
npx @workspaces-team/waker-config --keyword "Operator"
```

Write to a custom path:

```bash
npx @workspaces-team/waker-config --keyword "Operator" --out ./config/waker-head.config.json
```

Print the generated config to stdout:

```bash
npx @workspaces-team/waker-config --keyword "Operator" --stdout
```

## What You Get

- single-word tiny-head config and policy types
- browser/WASM-backed tiny-head trainer
- serialized head artifact helpers
- runtime path helpers for the bundled browser assets

## Use

```ts
import {
  createWakerWebHeadTrainer,
  serializeWakerHeadArtifact,
} from "@workspaces-team/waker-config";

const trainer = createWakerWebHeadTrainer();
await trainer.load();

const artifact = trainer.trainFromClips(
  [
    { pcm16k: positiveClip, label: 1 },
    { pcm16k: negativeClip, label: 0 },
  ],
  {
    keyword: "Navigator",
    registrationPolicy: "single_word_only",
  },
);

const json = serializeWakerHeadArtifact(artifact);
```

Use a generated config file directly:

```ts
import config from "./waker-head.config.json";
import { createWakerWebHeadTrainer } from "@workspaces-team/waker-config";

const trainer = createWakerWebHeadTrainer();
await trainer.load({ basePath: "/waker-config/" });

const artifact = trainer.trainFromClips(examples, config);
```

## Vite Integration

```ts
import { defineConfig } from "vite";
import { wakerConfigRuntimeAssetsPlugin } from "@workspaces-team/waker-config/vite";

export default defineConfig({
  plugins: [wakerConfigRuntimeAssetsPlugin()],
});
```

The default runtime mount base is:

```text
/waker-config/
```

If you override `mountBase`, pass the same base path when loading the trainer:

```ts
import { createWakerWebHeadTrainer } from "@workspaces-team/waker-config";

const trainer = createWakerWebHeadTrainer();
await trainer.load({
  basePath: "/waker-config/",
  policy: "single_word_only",
});
```

## Runtime Assets

This package is intended to be publishable and self-contained. It depends on:

- `runtime/wasm/*` for the generated browser detector runtime
- `runtime/single-word-only/*` for the tracked active single-word bundle

Preview the published package:

```bash
npm pack --dry-run
```

The `prepack` hook builds `runtime/wasm/*` from the mirrored `rust/sdk-wasm/` source tree at pack
time.

Refresh the tracked runtime bundle:

```bash
pnpm install
pnpm run sync:sdk-wasm:source
pnpm run sync:runtime-assets
```

Source-of-truth asset manifest:

- [runtime-assets.manifest.json](https://github.com/workspaces-team/waker/blob/main/packages/waker-config/runtime-assets.manifest.json)

Only the `single_word_only` runtime bundle is tracked in git.

## Deployment Notes

- Serve the JS wrapper and `.wasm` binary from stable static URLs.
- Keep `registration.json`, `runtime-config.json`, `backbone/model.bin`, and `backbone/model_manifest.json` together under the same runtime base.
- Use the Vite plugin when possible so dev and build output keep the same asset layout.
- If you serve the assets yourself, ensure `.wasm` is served with `application/wasm`.
- Preserve cross-origin isolation headers where your app depends on them.
- Do not assume any policy directory other than `single_word_only`.

## Scope

Belongs here:

- single-word training and artifact generation
- config generation and policy typing
- browser runtime/backbone asset resolution used by training
- runtime asset mounting for browser-side training

Does not belong here:

- live wake detection
- custom detector scoring at runtime
- browser-side inference APIs

Use `@workspaces-team/waker-web` for detection.

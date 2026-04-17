# @workspaces-team/waker-config

> Active development, very early release.
>
> If you hit a bug or confusing edge case, please open an issue. We are actively validating and
> verifying this public package shape.

Website: `https://waker.live`  
Issues: `https://github.com/workspaces-team/waker/issues`

Browser-side tiny-head configuration, artifact, and training utilities for Waker.

This package is the publishable training/config half of the web stack. It contains:

- tiny-head training types
- head artifact serialization helpers
- bundled runtime path helpers
- the browser/WASM-backed head trainer

It does **not** provide the runtime wake-word detector itself. For detection, use
[`@workspaces-team/waker-web`](../waker-web/README.md).

## Quick start

```bash
npx @workspaces-team/waker-config --keyword "Operator"
```

That writes a starter `waker-head.config.json` using the current single-word defaults.

If you want a different output path:

```bash
npx @workspaces-team/waker-config --keyword "Operator" --out ./config/waker-head.config.json
```

If you want to inspect the JSON without writing a file:

```bash
npx @workspaces-team/waker-config --keyword "Operator" --stdout
```

## Install

```bash
npm install @workspaces-team/waker-config
```

For a Vite app you will usually install it alongside `vite`:

```bash
npm install @workspaces-team/waker-config vite
```

## Package goals

This package is designed to stand on its own as an open-source npm package. It vendors the runtime
assets needed for browser-side tiny-head training so consumers can install it and build it without
depending on a larger private workspace at runtime.

## Package runtime assets

This package is intended to be publishable and self-contained. Its browser-side trainer depends on:

- `runtime/wasm/*` — the synced Rust/WASM module
- `runtime/single-word-only/*` — active single-word runtime bundle

To refresh those assets from a compatible source workspace:

```bash
pnpm install
pnpm run sync:wasm
pnpm run sync:runtime-assets
pnpm run build
```

Set `WAKER_SOURCE_REPO` to the absolute path of that source workspace before running the sync
commands.

The exact source-of-truth copy contract is documented in:

- [runtime-assets.manifest.json](./runtime-assets.manifest.json)

Only the `single_word_only` runtime is bundled in the publishable package right now.

The publishable package intentionally vendors only the browser assets it actually needs:

- `runtime/wasm/waker_wasm.js`
- `runtime/wasm/waker_wasm_bg.wasm`
- `runtime/single-word-only/backbone/model.bin`
- `runtime/single-word-only/backbone/model_manifest.json`
- `runtime/single-word-only/runtime-config.json`
- `runtime/single-word-only/registration.json`
- `runtime/single-word-only/registration/<keyword>/detector.json`

Those files should be treated as a unit. If you host them yourself, keep their relative paths
intact.

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

You can also start from the generated config file:

```ts
import config from "./waker-head.config.json";
import { createWakerWebHeadTrainer } from "@workspaces-team/waker-config";

const trainer = createWakerWebHeadTrainer();
await trainer.load({ basePath: "/waker-config/" });

const artifact = trainer.trainFromClips(examples, config);
```

## Vite integration

If you want the package to serve its vendored runtime assets directly inside a Vite app:

```ts
import { defineConfig } from "vite";
import { wakerConfigRuntimeAssetsPlugin } from "@workspaces-team/waker-config/vite";

export default defineConfig({
  plugins: [wakerConfigRuntimeAssetsPlugin()],
});
```

The plugin serves and copies the package runtime at:

```text
/waker-config/
```

That base path matters. If you override `mountBase`, use the same base path when loading the
trainer.

Then in the client:

```ts
import { createWakerWebHeadTrainer } from "@workspaces-team/waker-config";

const trainer = createWakerWebHeadTrainer();
await trainer.load({
  basePath: "/waker-config/",
  policy: "single_word_only",
});
```

## Static asset considerations

These are the deployment concerns this package expects:

- Serve the JS wrapper and `.wasm` binary from stable static URLs.
- Keep `registration.json`, `runtime-config.json`, `backbone/model.bin`, and
  `backbone/model_manifest.json` together under the same runtime base.
- Use the Vite plugin if possible so content types and copy behavior stay correct.
- If you serve the assets yourself, ensure `.wasm` is served with `application/wasm`.
- The dev plugin sets `Cross-Origin-Opener-Policy: same-origin` and
  `Cross-Origin-Embedder-Policy: require-corp`; preserve equivalent headers in deployments where
  your app depends on cross-origin isolation.
- The bundled runtime is single-word-only right now. Do not assume other policy directories are
  present.

## What belongs here

- Tiny-head training and artifact generation
- Tiny-head config and policy typing
- Shared browser runtime/backbone asset resolution used by training
- Runtime asset mounting for browser-side training

## What does not

- Live wake-word detection
- Custom detector loading / runtime scoring
- Vite/browser asset mounting

For those, use [`@workspaces-team/waker-web`](../waker-web/README.md).

## Open model note

The Waker tiny backbone and its weights are intended to be fully open and open-weight soon. This
package is structured so that transition can happen cleanly when the backing model artifacts are
ready to publish.

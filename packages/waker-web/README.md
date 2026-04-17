# @workspaces-team/waker-web

> Active development, very early release.
>
> If you hit a bug or confusing edge case, please open an issue. We are actively validating and
> verifying this public package shape.

Website: `https://waker.live`  
Issues: `https://github.com/workspaces-team/waker/issues`

Browser-side wake-word detection runtime for Waker.

This package is designed to stand on its own as an open-source npm package. It vendors the runtime
assets needed to build and publish the browser detector without depending on a larger private
workspace at runtime.

## Install

```bash
npm install @workspaces-team/waker-web
```

For a Vite app you will usually install it alongside `vite`:

```bash
npm install @workspaces-team/waker-web vite
```

## Use

```ts
import {
  createWakerWebDetector,
  getBundledWakerRegistrationUrl,
} from "@workspaces-team/waker-web";

const detector = createWakerWebDetector();
await detector.load(getBundledWakerRegistrationUrl("single_word_only"));
```

## Custom trained heads

```ts
import { createWakerWebCustomDetector } from "@workspaces-team/waker-web/custom-detector";

const detector = createWakerWebCustomDetector();
await detector.loadHead(artifactJson);
```

## Vite integration

```ts
import { wakerWebRuntimeAssetsPlugin } from "@workspaces-team/waker-web/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [wakerWebRuntimeAssetsPlugin()],
});
```

In a Vite app, the default mount base is:

```text
/waker-web/
```

So a minimal integration looks like:

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

The detector runtime is streaming Mu-Law first. The active runtime method is:

```ts
await detector.processMuLawChunk(chunk);
```

The `processChunk(Float32Array)` entry point is intentionally not supported in this browser runtime.

## Bundled runtime policies

- `single_word_only`

## Runtime assets

This package vendors:

- `runtime/wasm/*`
- `runtime/single-word-only/*`

More concretely, the publishable runtime consists of:

- `runtime/wasm/waker_wasm.js`
- `runtime/wasm/waker_wasm_bg.wasm`
- `runtime/single-word-only/backbone/model.bin`
- `runtime/single-word-only/backbone/model_manifest.json`
- `runtime/single-word-only/runtime-config.json`
- `runtime/single-word-only/registration.json`
- `runtime/single-word-only/registration/<keyword>/detector.json`

The exact source-of-truth copy contract is documented in:

- [runtime-assets.manifest.json](./runtime-assets.manifest.json)

## Static asset considerations

These are the browser loading constraints this package expects:

- Keep the static runtime files together. `registration.json` assumes stable relative paths to the
  detector config and backbone weights.
- Serve `.wasm` with `application/wasm`.
- Use the Vite plugin when possible so dev and build output use the same asset layout.
- If you change the mount base in the Vite plugin, pass the same `basePath` to
  `getBundledWakerRegistrationUrl(...)`.
- The dev plugin sets `Cross-Origin-Opener-Policy: same-origin` and
  `Cross-Origin-Embedder-Policy: require-corp`; preserve equivalent headers where your deployment
  depends on cross-origin isolation.
- The public package currently bundles only `single_word_only`.

## Maintainer refresh flow

```bash
pnpm install
pnpm run sync:wasm
pnpm run sync:runtime-assets
pnpm run build
```

If you refresh assets from a compatible source workspace, set `WAKER_SOURCE_REPO` to the absolute
path of that workspace before running the sync scripts.

## Open model note

The Waker tiny backbone and its weights are intended to become fully open and open-weight soon. The
package layout is designed so the runtime can adopt those open artifacts cleanly when they are
ready.

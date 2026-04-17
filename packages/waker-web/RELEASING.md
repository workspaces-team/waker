# Releasing @workspaces-team/waker-web

1. Refresh tracked runtime manifests:

```bash
pnpm run sync:sdk-wasm:source
pnpm run sync:runtime-assets
```

2. Inspect package contents. `prepack` materializes `runtime/wasm/*` from the mirrored
`rust/sdk-wasm/` source tree:

```bash
npm pack --dry-run
```

3. Publish:

```bash
pnpm run publish:waker-web
```

The tracked runtime manifest source-of-truth is documented in `runtime-assets.manifest.json`.
The emitted JS/WASM payload under `runtime/wasm/` is generated during `prepack` and is not kept in
git.

# Releasing @workspaces-team/waker-web

1. Refresh vendored assets:

```bash
pnpm run sync:wasm
pnpm run sync:runtime-assets
pnpm run sync:vad-wasm
```

2. Build:

```bash
pnpm run build
```

3. Inspect package contents:

```bash
npm pack --dry-run
```

4. Publish:

```bash
npm publish --access public
```

The runtime asset source-of-truth is documented in `runtime-assets.manifest.json`.

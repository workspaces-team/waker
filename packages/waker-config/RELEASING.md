# Releasing @workspaces-team/waker-config

1. Refresh runtime assets:

```bash
pnpm run sync:wasm
pnpm run sync:runtime-assets
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

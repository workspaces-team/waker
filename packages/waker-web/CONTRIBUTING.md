# Contributing to @workspaces-team/waker-web

## Local setup

```bash
pnpm install
pnpm run sync:wasm
pnpm run sync:runtime-assets
pnpm run build
```

If you refresh assets from another workspace, point `WAKER_SOURCE_REPO` at that workspace before
running sync commands.

## Healthy patterns

1. Keep the runtime browser-only and client-side.
2. Treat runtime assets as vendored snapshots with a declared source-of-truth copy contract.
3. Keep runtime metadata explicit and versioned.
4. Preserve the single-word-only default until newer policy evidence displaces it.
5. Document every browser compatibility bridge, especially around backbone asset shape.

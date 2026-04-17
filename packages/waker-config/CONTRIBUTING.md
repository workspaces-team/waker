# Contributing to @workspaces-team/waker-config

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

1. Keep training fully client-side and browser-compatible.
2. Treat runtime assets as vendored snapshots with a declared source-of-truth copy contract.
3. Keep artifact/config schemas additive and versioned.
4. Prefer config-driven behavior over word-specific code paths.
5. Document every runtime asset assumption in the package itself.

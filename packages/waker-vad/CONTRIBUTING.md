# Contributing to @workspaces-team/waker-vad

## Local setup

```bash
pnpm install
pnpm run sync:runtime
pnpm run build
```

If you refresh assets from another workspace, point `WAKER_SOURCE_REPO` at that workspace before
running sync commands.

## Healthy patterns

1. Keep the VAD runtime browser-only and asset-oriented.
2. Treat VAD assets as generated publish artifacts, not handwritten package contents.
3. Keep package scope limited to VAD payload delivery and mounting.

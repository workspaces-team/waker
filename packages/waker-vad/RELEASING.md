# Releasing @workspaces-team/waker-vad

1. Refresh mirrored VAD source:

```bash
pnpm run sync:vad-wasm:source
```

2. Inspect package contents. `prepack` materializes `runtime/vad/*` from the mirrored
`rust/vad-wasm/` source tree:

```bash
npm pack --dry-run
```

3. Publish:

```bash
pnpm run publish:waker-vad
```

The emitted JS/WASM payload under `runtime/vad/` is generated during `prepack` and is not kept in
git.

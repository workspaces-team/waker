# Releasing

This workspace publishes two public npm packages:

- `@workspaces-team/waker-config`
- `@workspaces-team/waker-web`

Current release posture:

- active development
- very early alpha versions
- validate packaging carefully before each publish

## Pre-release checklist

1. Refresh vendored assets from the source workspace.
2. Rebuild both packages.
3. Verify both packages pack cleanly.
4. Review the runtime asset manifests.
5. Publish with public access.

## Commands

```bash
pnpm install
pnpm run waker-config:sync:wasm
pnpm run waker-config:sync:runtime-assets
pnpm run waker-web:sync:wasm
pnpm run waker-web:sync:runtime-assets
pnpm run waker-web:sync:vad-wasm
pnpm run build
pnpm run typecheck
```

Dry-run package verification:

```bash
cd packages/waker-config && npm pack --dry-run
cd ../waker-web && npm pack --dry-run
```

## Publish

```bash
cd packages/waker-config && npm publish --access public
cd ../waker-web && npm publish --access public
```

## Source workspace

If you are refreshing the vendored runtime assets, point the sync scripts at a compatible source
workspace by setting `WAKER_SOURCE_REPO` to the absolute path of that workspace before running the
sync commands.

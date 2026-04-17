# Contributing to Waker

Thanks for taking a look.

This repo is in active development and still very early. Please treat the current package APIs as
stabilizing rather than final.

## Before opening a PR

1. Open an issue first if the change is large, behavior-changing, or API-shaping.
2. Keep changes scoped to one package or one concern when possible.
3. Preserve the current browser-first, WASM-first runtime model.

## Development

```bash
pnpm install
pnpm run build
pnpm run typecheck
```

## Asset refresh flow

The published packages vendor browser runtime assets from a compatible source workspace.

Set `WAKER_SOURCE_REPO` to the absolute path of that source workspace before running the sync
commands.

Then refresh:

```bash
pnpm run waker-config:sync:wasm
pnpm run waker-config:sync:runtime-assets
pnpm run waker-web:sync:wasm
pnpm run waker-web:sync:runtime-assets
pnpm run waker-web:sync:vad-wasm
pnpm run build
```

## Reporting issues

Please open issues at:

`https://github.com/workspaces-team/waker/issues`

The project website is:

`https://waker.live`

## Open model note

The Waker tiny backbone model and weights are intended to become fully open and open-weight soon.

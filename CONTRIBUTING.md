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
pnpm run sdk-wasm:build:release
pnpm run vad-wasm:build:release
pnpm run verify
```

If your branch changes publishable files under `packages/waker-config/`, `packages/waker-vad/`, `packages/waker-web/`, or
the mirrored `rust/sdk-wasm/` / `rust/vad-wasm/` inputs for those packages, bump the affected
package version before merge:

```bash
pnpm run version:packages -- --package=all --bump=patch
pnpm run version:check:pr -- --base-ref origin/main
```

## Source Sync Flow

Tracked runtime manifests and mirrored Rust source sync from a compatible source workspace.

Set `WAKER_SOURCE_REPO` to the absolute path of that source workspace before running the sync
commands.

Then refresh:

```bash
pnpm run sync:sdk-wasm:source
pnpm run sync:vad-wasm:source
pnpm run waker-config:sync:runtime-assets
pnpm run waker-vad:sync:runtime
pnpm run waker-web:sync:runtime-assets
```

The generated payload under `packages/waker-config/runtime/wasm/`,
`packages/waker-web/runtime/wasm/`, and `packages/waker-vad/runtime/vad/` is built from mirrored
Rust source during package prepack and is not tracked in git.

## Reporting issues

Please open issues at:

`https://github.com/workspaces-team/waker/issues`

The project website is:

`https://waker.live`

## Open model note

The Waker tiny backbone model and weights are intended to become fully open and open-weight soon.

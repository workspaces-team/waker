# waker

Public pnpm workspace monorepo for the browser-side Waker packages.

> Active development, very early release.
>
> If you run into a problem, please open an issue. We are actively validating and verifying this
> release and expect some rough edges while the public package surface settles.

Website: `https://waker.live`  
Issues: `https://github.com/workspaces-team/waker/issues`

This repo groups two publishable npm packages under the `@workspaces-team` organization:

- `@workspaces-team/waker-config`
- `@workspaces-team/waker-web`

Both packages are intentionally versioned as very early alpha packages while we validate the
public release shape.

## Workspace layout

```text
packages/
  waker-config/
  waker-web/
```

## What each package does

### `@workspaces-team/waker-config`

Owns browser-side tiny-head config, artifact, and training utilities:

- tiny-head config/types
- browser/WASM-backed tiny-head trainer
- head artifact serialization
- trainer-side bundled runtime helpers

### `@workspaces-team/waker-web`

Owns browser-side wake-word detection:

- bundled detector runtime
- custom trained-head loading
- Vite runtime asset plugin
- vendored runtime assets for the active single-word policies

## Install

```bash
pnpm install
pnpm run build
```

## npm packages

### `@workspaces-team/waker-config`

Browser-side tiny-head training and artifact generation.

```bash
npx @workspaces-team/waker-config --keyword "Operator"
```

See [`packages/waker-config/README.md`](./packages/waker-config/README.md) for full usage and Vite
integration.

### `@workspaces-team/waker-web`

Browser-side wake-word detection runtime.

```bash
npm install @workspaces-team/waker-web
```

See [`packages/waker-web/README.md`](./packages/waker-web/README.md) for runtime loading and Vite
integration.

## Refresh vendored assets

Both packages vendor browser runtime assets from a compatible source workspace.

Set `WAKER_SOURCE_REPO` to the absolute path of that source workspace before running the sync
commands.

Then run:

```bash
pnpm run waker-config:sync:wasm
pnpm run waker-config:sync:runtime-assets
pnpm run waker-web:sync:wasm
pnpm run waker-web:sync:runtime-assets
pnpm run waker-web:sync:vad-wasm
```

## Open-source posture

This repo is organized to be healthy for open-source publication:

- clear package boundaries
- explicit vendored runtime assets
- source-of-truth copy manifests
- standalone package metadata and release docs

See also:

- [`CONTRIBUTING.md`](./CONTRIBUTING.md)
- [`RELEASING.md`](./RELEASING.md)

The Waker tiny backbone model and weights are also intended to become fully open and open-weight
soon, so these packages are structured to absorb that transition cleanly.

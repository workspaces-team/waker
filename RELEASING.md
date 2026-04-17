# Releasing

This workspace publishes two public npm packages:

- `@workspaces-team/waker-config`
- `@workspaces-team/waker-vad`
- `@workspaces-team/waker-web`

Current release posture:

- active development
- very early alpha versions
- validate packaging carefully before each publish

## Pre-release checklist

1. Refresh tracked runtime manifests and mirrored Rust source from the source workspace.
2. Bump package versions.
3. Run public mirror verification.
4. Verify both packages pack cleanly. Their `prepack` hooks materialize publish-only JS/WASM assets.
5. Review the runtime asset manifests.
6. Publish with public access.

## Commands

```bash
pnpm install
pnpm run sync:sdk-wasm:source
pnpm run sync:vad-wasm:source
pnpm run verify
pnpm run version:packages -- --package=all --bump=patch --dry-run
pnpm run version:check:pr -- --base-ref origin/main
pnpm run waker-config:sync:runtime-assets
pnpm run waker-vad:sync:runtime
pnpm run waker-web:sync:runtime-assets
```

Recommended versioning posture:

- default to lockstep versions for `@workspaces-team/waker-config`, `@workspaces-team/waker-vad`, and `@workspaces-team/waker-web`
- stay on `0.x` while the public API is still moving
- use `preminor` / `prerelease` with `--preid=alpha` or `--preid=rc` for unstable cuts
- only bump one package independently when the change is clearly isolated

Examples:

```bash
pnpm run version:packages -- --package=all --bump=patch
pnpm run version:packages -- --package=all --bump=preminor --preid=rc
pnpm run version:packages -- --package=waker-web --set=0.3.0
```

Dry-run package verification:

```bash
cd packages/waker-config && npm pack --dry-run
cd ../waker-web && npm pack --dry-run
```

## Publish

```bash
pnpm run publish:waker-config
pnpm run publish:waker-vad
pnpm run publish:waker-web
```

The publish scripts infer the npm dist-tag from the package version:

- stable versions publish to `latest`
- `*-alpha.*` publishes to `alpha`
- `*-rc.*` publishes to `rc`

The final `npm pack` / `npm publish` path now builds from mirrored Rust source in this public repo,
but you still need `WAKER_SOURCE_REPO` available for refreshing tracked runtime manifests from the
private Academy deliverables before a release cut.

## npm Auth

If this machine is not logged in to npm, you can store an npm access token in `~/.npmrc` with:

```bash
pnpm run auth:npm
```

Or non-interactively:

```bash
NPM_TOKEN=your_token_here pnpm run auth:npm
```

## Source workspace

If you are refreshing the vendored runtime assets, point the sync scripts at a compatible source
workspace by setting `WAKER_SOURCE_REPO` to the absolute path of that workspace before running the
sync commands or `npm pack` / `npm publish`.

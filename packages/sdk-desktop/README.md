# `@waker/sdk-desktop`

`@waker/sdk-desktop` is the in-repo reference package for native Node-facing Rust deliverables.

It follows the same boundary shape as `pretty-bench`:

- Rust addon crate lives in `rust/`
- packaged `.node` binaries live in `native/<platform>/`
- TypeScript stays thin and loads the addon from `src/`
- build scripts copy Cargo output into the packaged native layout

This package currently standardizes the native addon layout, manifest loading, and binding resolution for
desktop-targeted Waker runtimes. Browser-facing Rust deliverables stay on the `wasm-pack` path instead.

## Development

Build the local native addon in debug mode:

```bash
pnpm --filter @waker/sdk-desktop run build:native:debug
```

Build the package wrapper and release addon:

```bash
pnpm --filter @waker/sdk-desktop run build
```

Run the Rust tests:

```bash
pnpm --filter @waker/sdk-desktop run test:native
```

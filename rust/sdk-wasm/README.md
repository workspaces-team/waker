# sdk-wasm

Rust-to-WebAssembly wake-word detector runtime for Waker.

This crate compiles the audio frontend (FFT-based mel spectrogram) and detector head
(temporal convolution features, wEffective projection, classifier scoring, temperature
calibration, and decision logic) to WebAssembly for near-native performance in the browser.

## Architecture

For v1, this crate handles:

- **Audio ingestion**: Mu-Law decoding, resampling, ring buffer management
- **Audio frontend**: FFT-based log-mel spectrogram (replaces the JS O(N²) DFT)
- **Detector head**: wEffective projection, temporal conv features, classifier scoring

Backbone ONNX inference is delegated to the JS side (onnxruntime-web), which passes
the embedding sequence back to WASM for detector head scoring.

## Build

```bash
# Requires: rustup, wasm-pack
pnpm run sdk-wasm:build          # dev build
pnpm run sdk-wasm:build:release  # optimized release build
```

## Test

```bash
pnpm run sdk-wasm:test           # run all Rust unit tests
```

## Binary size

The release `.wasm` binary is approximately **341 KB** (without wasm-opt).

## Integration

The generated WASM package in `pkg/` is consumed by `@workspaces-team/waker-config` and
`@workspaces-team/waker-web` in the public mirror.

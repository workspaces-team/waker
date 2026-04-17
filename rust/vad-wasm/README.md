# vad-wasm

Silero VAD v5 compiled to WebAssembly as a bespoke forward pass in pure Rust.

**No ONNX Runtime. No ML framework. No npm dependencies.**

The entire neural network — STFT, Conv1d encoder, LSTM, and decoder — is hand-coded in
~870 lines of Rust, producing a **121 KB** WASM binary that replaces the 3–10 MB
`onnxruntime-web` engine used by existing browser VAD libraries.

## Comparison with `@ricky0123/vad-web`

Both packages run the same underlying model (Silero VAD v5). The difference is how.

| | **@ricky0123/vad-web** | **vad-wasm** |
|---|---|---|
| **Inference engine** | `onnxruntime-web` (general-purpose) | Bespoke Rust forward pass |
| **Library code** | ~1 MB (minified JS) | **17 KB** (auto-generated JS glue) |
| **Runtime engine** | 3–10 MB (ORT WASM binaries) | **121 KB** (compiled Rust) |
| **Model weights** | ~2.2 MB (`silero_vad.onnx`) | **1.18 MB** (flat f32 binary) |
| **Total download** | ~5–15 MB | **~1.32 MB** |
| **Live memory** | 5–15 MB | **~1.5 MB** |
| **GC pressure** | High (JS array alloc per chunk) | **Zero** (WASM linear memory) |
| **Cold start** | 500 ms–2 s (ORT init + graph compile) | **< 50 ms** (weight load only) |
| **npm dependencies** | `onnxruntime-web` + 50 transitive | **0** |
| **Bundler config** | CopyPlugin for `.wasm`, `.onnx`, worklet | 1 `.wasm` + 1 `.bin` + 1 `.json` |
| **Mobile stability** | OOM on iOS reported | Flat memory, no session overhead |

### Why ours is 25–80× smaller

`@ricky0123/vad-web` loads the full ONNX Runtime engine — designed to run *any* ONNX model — and
feeds it the Silero `.onnx` file at runtime. ORT parses the graph, materializes sessions, allocates
operator registries, and dispatches through generalized kernels.

`vad-wasm` implements *only* the exact math this specific model needs:

- 1 STFT convolution (258 filters, kernel=256, stride=128)
- 4 encoder Conv1d blocks (129→128→64→64→128)
- 1 LSTM step (hidden_size=128)
- 1 decoder Conv1d (128→1) + sigmoid

That is the entire inference engine. No graph interpreter, no session manager, no operator
registry, no dynamic dispatch.

### What `@ricky0123/vad-web` provides that we don't (yet)

| Feature | @ricky0123/vad-web | vad-wasm |
|---------|-------------------|-----------------|
| Core VAD inference | ✅ | ✅ |
| Speech probability | ✅ | ✅ |
| Speech start/end events | ✅ (`onSpeechStart`, `onSpeechEnd`) | ✅ (`SpeechEvent` enum) |
| Misfire detection | ✅ (`onVADMisfire`) | Configurable via `min_speech_chunks` |
| Speech padding (pre/post) | ✅ (configurable ms) | Not yet (planned for TS wrapper) |
| AudioWorklet integration | ✅ (built-in worklet) | Not yet (planned for TS wrapper) |
| React hooks | ✅ (`useMicVAD()`) | Not yet (planned for TS wrapper) |
| Pre-bundled model (CDN) | ✅ | Manual weight loading |
| Configurable threshold | ✅ | ✅ |
| Streaming state management | ✅ (implicit via ORT) | ✅ (explicit LSTM h/c state) |

The higher-level features (AudioWorklet wiring, React hooks, speech padding) are integration
concerns that belong in a TypeScript wrapper package consuming this WASM core. In this repo,
`@workspaces-team/waker-web` consumes the generated VAD payload built from this source tree.

## Architecture

The 16 kHz forward pass pipeline:

```
audio chunk (512 samples)
  │
  ├─ prepend 64-sample context from previous chunk
  │
  ▼
STFT: Conv1d(1, 258, k=256, s=128, no bias) → magnitude
  │    [258, T] → [129, T]
  ▼
Encoder:
  Conv1d(129, 128, k=3, s=1, p=1) → ReLU
  Conv1d(128,  64, k=3, s=2, p=1) → ReLU
  Conv1d( 64,  64, k=3, s=2, p=1) → ReLU
  Conv1d( 64, 128, k=3, s=1, p=1) → ReLU
  │    [128, T']
  ▼
LSTM: single-layer (128 → 128)
  │    state: h[128], c[128] (persisted across chunks)
  ▼
Decoder:
  ReLU → Conv1d(128, 1, k=1) → Sigmoid
  │
  ▼
speech probability ∈ [0.0, 1.0]
```

## Model weights

Weights are extracted from the official Silero VAD v5 ONNX model into a flat binary format:

| File | Size | Description |
|------|------|-------------|
| `silero_vad.onnx` | 2.2 MB | Original model (reference only) |
| `silero_vad_16k.bin` | 1.18 MB | Extracted f32 weights (309K params) |
| `silero_vad_16k_manifest.json` | 1.5 KB | Tensor name → byte offset mapping |

Weight breakdown:

| Layer | Shape | Params |
|-------|-------|--------|
| STFT basis | [258, 1, 256] | 66,048 |
| Encoder conv 0 (weight + bias) | [128, 129, 3] + [128] | 49,664 |
| Encoder conv 1 | [64, 128, 3] + [64] | 24,640 |
| Encoder conv 2 | [64, 64, 3] + [64] | 12,352 |
| Encoder conv 3 | [128, 64, 3] + [128] | 24,704 |
| LSTM (w_ih + w_hh + b_ih + b_hh) | [512,128]×2 + [512]×2 | 132,096 |
| Decoder conv (weight + bias) | [1, 128, 1] + [1] | 129 |
| **Total** | | **309,633** |

## Build

```bash
# Requires: rustup + wasm-pack
pnpm run vad-wasm:build          # dev build (fast, unoptimized)
pnpm run vad-wasm:build:release  # release build (121 KB, size-optimized)
```

## Test

```bash
pnpm run vad-wasm:test           # run all 11 Rust unit tests
```

## Usage from JavaScript

```js
import init, { SileroVadDetector, VadConfig } from "./vad_wasm.js";

// Initialize WASM module
await init();

// Create detector
const detector = new SileroVadDetector();

// Load weights
const weightsBin = await fetch("silero_vad_16k.bin").then(r => r.arrayBuffer());
const manifest = await fetch("silero_vad_16k_manifest.json").then(r => r.text());
detector.loadModel(new Uint8Array(weightsBin), manifest);

// Optional: configure thresholds
detector.setThreshold(0.5);

// --- Simple mode: get probability ---
const result = detector.process(audioChunk512);
console.log(result.probability); // 0.0 – 1.0
console.log(result.isSpeech);    // true/false

// --- Event mode: speech start/end tracking ---
const event = detector.processWithEvents(audioChunk512);
console.log(event.probability);  // 0.0 – 1.0
console.log(event.event);        // 0=Silence, 1=SpeechStart, 2=Speaking, 3=SpeechEnd
console.log(event.isSpeech);     // true during SpeechStart and Speaking

// Reset state between sessions
detector.reset();
```

## Speech event flow

```
Silence → SpeechStart → Speaking → ... → Speaking → SpeechEnd → Silence
   │          │                                          │          │
   │    (min_speech_chunks                    (min_silence_chunks    │
   │     consecutive hits)                     consecutive misses)   │
   ▼                                                                ▼
probability < threshold                            probability < threshold
```

## Source structure

```
vad-wasm/
├── Cargo.toml          # Zero ML framework dependencies
├── src/
│   ├── lib.rs          # SileroVadDetector (wasm-bindgen entry point)
│   ├── model.rs        # Bespoke forward pass (STFT → Enc → LSTM → Dec)
│   ├── nn.rs           # Pure Rust: Conv1d, LSTM step, ReLU, sigmoid, STFT magnitude
│   └── weights.rs      # Binary blob weight loader with JSON manifest
├── model/
│   ├── silero_vad.onnx             # Original Silero v5 model (reference)
│   ├── silero_vad_16k.bin          # Extracted weights (1.18 MB)
│   └── silero_vad_16k_manifest.json
└── pkg/                            # wasm-pack output (generated)
    ├── vad_wasm.js                 # JS glue (17 KB)
    ├── vad_wasm.d.ts               # TypeScript definitions
    └── vad_wasm_bg.wasm            # WASM binary (121 KB)
```

## License

The Silero VAD model is MIT licensed. This package is MIT licensed.

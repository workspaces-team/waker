/* @ts-self-types="./waker_wasm.d.ts" */

//#region exports

export class WakerDetectionResult {
    constructor() {
        throw new Error('cannot invoke `new` directly');
    }
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WakerDetectionResult.prototype);
        obj.__wbg_ptr = ptr;
        WakerDetectionResultFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WakerDetectionResultFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wakerdetectionresult_free(ptr, 0);
    }
    /**
     * The wake forms accepted under the active registration policy.
     *
     * Mirrors `WakerWebDetectionResult.acceptedWakeForms` in `@waker/sdk-web`.
     * @returns {string[]}
     */
    get acceptedWakeForms() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wakerdetectionresult_acceptedWakeForms(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {string}
     */
    get chosenWakeForm() {
        let deferred1_0;
        let deferred1_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wakerdetectionresult_chosenWakeForm(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {boolean}
     */
    get detected() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wakerdetectionresult_detected(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {string}
     */
    get keyword() {
        let deferred1_0;
        let deferred1_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wakerdetectionresult_keyword(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {number}
     */
    get score() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wakerdetectionresult_score(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get threshold() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wakerdetectionresult_threshold(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) WakerDetectionResult.prototype[Symbol.dispose] = WakerDetectionResult.prototype.free;

/**
 * The main WASM-based wake-word detector.
 *
 * Handles the full audio → detection pipeline or accepts pre-computed backbone
 * embeddings for the detector head only.
 */
export class WakerWasmDetector {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WakerWasmDetectorFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wakerwasmdetector_free(ptr, 0);
    }
    /**
     * Get the expected backbone output length (seq_len × embedding_dim).
     * @returns {number}
     */
    get backboneOutputLength() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wakerwasmdetector_backboneOutputLength(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Configure just the frontend/backbone path for browser-side embedding extraction.
     *
     * `runtime_backbone_json` should match the `runtimeBackbone` shape from detector.json.
     * This method seeds a no-op identity head so callers can reuse the same WASM package
     * for clip embedding and later for a trained detector.
     * @param {string} runtime_backbone_json
     * @param {number} capture_sample_rate
     */
    configureBackbone(runtime_backbone_json, capture_sample_rate) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(runtime_backbone_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(capture_sample_rate);
        const ret = wasm.wakerwasmdetector_configureBackbone(this.__wbg_ptr, ptr0, len0, capture_sample_rate);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Run a fully buffered 16 kHz mono clip through the frontend and backbone and return
     * the flat embedding sequence `[sequenceLength * embeddingDim]`.
     * @param {Float32Array} pcm16k
     * @returns {Float32Array}
     */
    embedPcm16kClip(pcm16k) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passArrayF32ToWasm0(pcm16k, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wakerwasmdetector_embedPcm16kClip(this.__wbg_ptr, ptr0, len0);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v2 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v2;
    }
    /**
     * Check if the backbone weights are loaded.
     * @returns {boolean}
     */
    get isBackboneLoaded() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wakerwasmdetector_isBackboneLoaded(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Check if the detector is fully ready for the complete pipeline.
     * @returns {boolean}
     */
    get isFullyReady() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wakerwasmdetector_isFullyReady(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Check if the detector is loaded and ready (config + backbone weights).
     * @returns {boolean}
     */
    get isLoaded() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wakerwasmdetector_isLoaded(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Load backbone weights from the extracted binary blob and manifest.
     *
     * This enables the fully self-contained pipeline — no onnxruntime-web needed.
     *
     * `weights_binary`: contents of `backbone_16k.bin`
     * `manifest_json`: contents of `backbone_16k_manifest.json`
     * @param {Uint8Array} weights_binary
     * @param {string} manifest_json
     */
    loadBackboneWeights(weights_binary, manifest_json) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passArray8ToWasm0(weights_binary, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(manifest_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wakerwasmdetector_loadBackboneWeights(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Load detector configuration from JSON strings.
     *
     * `registration_json`: contents of registration.json
     * `detector_json`: contents of detector.json
     * `capture_sample_rate`: the browser capture sample rate (typically 24000)
     * @param {string} registration_json
     * @param {string} detector_json
     * @param {number} capture_sample_rate
     */
    loadConfig(registration_json, detector_json, capture_sample_rate) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(registration_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(detector_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        _assertNum(capture_sample_rate);
        const ret = wasm.wakerwasmdetector_loadConfig(this.__wbg_ptr, ptr0, len0, ptr1, len1, capture_sample_rate);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Get the number of mel features the frontend produces per chunk.
     * @returns {number}
     */
    get melTensorLength() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wakerwasmdetector_melTensorLength(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Create a new detector instance.
     */
    constructor() {
        const ret = wasm.wakerwasmdetector_new();
        this.__wbg_ptr = ret >>> 0;
        WakerWasmDetectorFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Process a Mu-Law encoded audio chunk from the browser microphone.
     *
     * Returns a detection result once the ring buffer is full, or null if
     * the buffer is still filling. The backbone inference is handled externally
     * (by the JS ONNX runtime). Call `processBackboneOutput` instead when the
     * backbone output is available.
     *
     * This method handles: Mu-Law decode → resample → ring buffer → mel frontend.
     * It returns the mel spectrogram as a flat Float32Array for the JS side to
     * pass to the ONNX backbone.
     * @param {Uint8Array} chunk
     * @returns {Float32Array | undefined}
     */
    processAudioToMel(chunk) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passArray8ToWasm0(chunk, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wakerwasmdetector_processAudioToMel(this.__wbg_ptr, ptr0, len0);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        let v2;
        if (ret[0] !== 0) {
            v2 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        }
        return v2;
    }
    /**
     * Score a backbone embedding sequence through the detector head.
     *
     * `backbone_output`: flat Float32Array of shape [seq_len × embedding_dim]
     *     from the ONNX backbone inference on the JS side.
     * `now_ms`: current timestamp in milliseconds (from Date.now()).
     *
     * Returns the detection result with score, threshold, and detected flag.
     * @param {Float32Array} backbone_output
     * @param {number} now_ms
     * @returns {WakerDetectionResult}
     */
    processBackboneOutput(backbone_output, now_ms) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passArrayF32ToWasm0(backbone_output, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wakerwasmdetector_processBackboneOutput(this.__wbg_ptr, ptr0, len0, now_ms);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WakerDetectionResult.__wrap(ret[0]);
    }
    /**
     * Process a Mu-Law encoded audio chunk through the **complete** pipeline.
     *
     * Mu-Law decode → resample → ring buffer → mel frontend → backbone → detector head → decision.
     *
     * **No onnxruntime-web needed.** Everything runs in WASM.
     *
     * Returns `None` if the ring buffer is still filling, or a `WakerDetectionResult`
     * once enough audio has been buffered.
     * @param {Uint8Array} chunk
     * @param {number} now_ms
     * @returns {WakerDetectionResult | undefined}
     */
    processMuLawChunk(chunk, now_ms) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passArray8ToWasm0(chunk, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wakerwasmdetector_processMuLawChunk(this.__wbg_ptr, ptr0, len0, now_ms);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] === 0 ? undefined : WakerDetectionResult.__wrap(ret[0]);
    }
    /**
     * Reset the detector state (ring buffer, decision state).
     */
    reset() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        wasm.wakerwasmdetector_reset(this.__wbg_ptr);
    }
}
if (Symbol.dispose) WakerWasmDetector.prototype[Symbol.dispose] = WakerWasmDetector.prototype.free;

/**
 * @param {Float32Array} flattened_sequences
 * @param {Uint8Array} labels
 * @param {string} config_json
 * @returns {string}
 */
export function trainTemporalConvHead(flattened_sequences, labels, config_json) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passArrayF32ToWasm0(flattened_sequences, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(labels, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(config_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.trainTemporalConvHead(ptr0, len0, ptr1, len1, ptr2, len2);
        var ptr4 = ret[0];
        var len4 = ret[1];
        if (ret[3]) {
            ptr4 = 0; len4 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}

//#endregion

//#region wasm imports

function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_81fc77679af83bc6: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbindgen_cast_0000000000000001: function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        }, arguments); },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./waker_wasm_bg.js": import0,
    };
}


//#endregion
const WakerDetectionResultFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wakerdetectionresult_free(ptr >>> 0, 1));
const WakerWasmDetectorFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wakerwasmdetector_free(ptr >>> 0, 1));


//#region intrinsics
function _assertNum(n) {
    if (typeof(n) !== 'number') throw new Error(`expected a number argument, found ${typeof(n)}`);
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayJsValueFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getDataViewMemory0();
    const result = [];
    for (let i = ptr; i < ptr + 4 * len; i += 4) {
        result.push(wasm.__wbindgen_externrefs.get(mem.getUint32(i, true)));
    }
    wasm.__externref_drop_slice(ptr, len);
    return result;
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function logError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        let error = (function () {
            try {
                return e instanceof Error ? `${e.message}\n\nStack:\n${e.stack}` : e.toString();
            } catch(_) {
                return "<failed to stringify thrown value>";
            }
        }());
        console.error("wasm-bindgen: imported JS function that was not marked as `catch` threw an error:", error);
        throw e;
    }
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArrayF32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getFloat32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (typeof(arg) !== 'string') throw new Error(`expected a string argument, found ${typeof(arg)}`);
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);
        if (ret.read !== arg.length) throw new Error('failed to pass whole string');
        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;


//#endregion

//#region wasm loading
let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('waker_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
//#endregion
export { wasm as __wasm }

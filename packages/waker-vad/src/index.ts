export const DEFAULT_VAD_BASE_PATH = "/waker-vad/";

function normalizeBasePath(basePath: string): string {
  const withLeadingSlash = basePath.startsWith("/") ? basePath : `/${basePath}`;
  return withLeadingSlash.endsWith("/") ? withLeadingSlash : `${withLeadingSlash}/`;
}

export function getBundledWakerVadBasePath(options: { basePath?: string } = {}): string {
  return normalizeBasePath(options.basePath ?? DEFAULT_VAD_BASE_PATH);
}

export function getBundledWakerVadWasmModuleUrl(options: { basePath?: string } = {}): string {
  return `${getBundledWakerVadBasePath(options)}vad_wasm.js`;
}

export function getBundledWakerVadWasmBinaryUrl(options: { basePath?: string } = {}): string {
  return `${getBundledWakerVadBasePath(options)}vad_wasm_bg.wasm`;
}

export function getBundledWakerVadWeightsUrl(options: { basePath?: string } = {}): string {
  return `${getBundledWakerVadBasePath(options)}silero_vad_16k.bin`;
}

export function getBundledWakerVadManifestUrl(options: { basePath?: string } = {}): string {
  return `${getBundledWakerVadBasePath(options)}silero_vad_16k_manifest.json`;
}

export async function fetchText(url: string): Promise<string> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to load ${url}: ${response.status} ${response.statusText}`);
  }
  return response.text();
}

export async function fetchUint8Array(url: string): Promise<Uint8Array> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to load ${url}: ${response.status} ${response.statusText}`);
  }
  return new Uint8Array(await response.arrayBuffer());
}

export async function loadBundledWakerVadAssets(
  options: { basePath?: string } = {},
): Promise<{ manifestJson: string; weightsBinary: Uint8Array }> {
  const [weightsBinary, manifestJson] = await Promise.all([
    fetchUint8Array(getBundledWakerVadWeightsUrl(options)),
    fetchText(getBundledWakerVadManifestUrl(options)),
  ]);
  return { manifestJson, weightsBinary };
}

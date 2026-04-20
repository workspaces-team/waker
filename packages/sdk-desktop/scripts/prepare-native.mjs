import fs from "node:fs";
import path from "node:path";

const PACKAGE_ROOT = path.resolve(new URL("..", import.meta.url).pathname);

const PLATFORM_TAGS = {
  "darwin-arm64": "darwin-arm64",
  "darwin-x64": "darwin-x64",
  "linux-arm64": "linux-arm64",
  "linux-x64": "linux-x64",
  "win32-arm64": "win32-arm64",
  "win32-x64": "win32-x64",
};

function packageRoot() {
  return PACKAGE_ROOT;
}

function platformTag(platform = process.platform, arch = process.arch) {
  const key = `${platform}-${arch}`;
  const tag = PLATFORM_TAGS[key];
  if (!tag) {
    throw new Error(`Unsupported platform for @waker/sdk-desktop: ${key}`);
  }
  return tag;
}

function nativeOutputPath(root = packageRoot(), tag = platformTag()) {
  return path.join(root, "native", tag, "waker-sdk-desktop.node");
}

function cargoLibraryFileName(platform = process.platform) {
  if (platform === "win32") {
    return "waker_sdk_desktop_native.dll";
  }
  if (platform === "darwin") {
    return "libwaker_sdk_desktop_native.dylib";
  }
  return "libwaker_sdk_desktop_native.so";
}

const root = packageRoot();
const release = process.argv.includes("--release");
const targetDir = release ? "release" : "debug";
const sourceIndex = process.argv.indexOf("--source");
const sourceBinary =
  sourceIndex >= 0
    ? path.resolve(process.argv[sourceIndex + 1])
    : path.join(root, "rust", "target", targetDir, cargoLibraryFileName());

if (!fs.existsSync(sourceBinary)) {
  throw new Error(
    `No local native addon was found at ${sourceBinary}. Build it first with pnpm --filter @waker/sdk-desktop run build:native${release ? "" : ":debug"}.`,
  );
}

const outputPath = nativeOutputPath(root, platformTag());
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.copyFileSync(sourceBinary, outputPath);
if (process.platform !== "win32") {
  fs.chmodSync(outputPath, 0o755);
}

process.stdout.write(`Prepared native addon ${sourceBinary} -> ${outputPath}\n`);

import path from "node:path";
import { fileURLToPath } from "node:url";

const PACKAGE_ROOT = path.resolve(fileURLToPath(new URL("../", import.meta.url)));

const PLATFORM_TAGS: Record<string, string> = {
  "darwin-arm64": "darwin-arm64",
  "darwin-x64": "darwin-x64",
  "linux-arm64": "linux-arm64",
  "linux-x64": "linux-x64",
  "win32-arm64": "win32-arm64",
  "win32-x64": "win32-x64",
};

export function packageRoot(): string {
  return PACKAGE_ROOT;
}

export function platformTag(platform = process.platform, arch = process.arch): string {
  const key = `${platform}-${arch}`;
  const tag = PLATFORM_TAGS[key];
  if (!tag) {
    throw new Error(`Unsupported platform for @waker/sdk-desktop: ${key}`);
  }
  return tag;
}

export function nativeAddonName(): string {
  return "waker-sdk-desktop.node";
}

export function nativeOutputPath(root = packageRoot(), tag = platformTag()): string {
  return path.join(root, "native", tag, nativeAddonName());
}

export function localNativeCandidates(root = packageRoot()): string[] {
  return [nativeOutputPath(root, platformTag())];
}

export function cargoLibraryFileName(platform = process.platform): string {
  if (platform === "win32") {
    return "waker_sdk_desktop_native.dll";
  }
  if (platform === "darwin") {
    return "libwaker_sdk_desktop_native.dylib";
  }
  return "libwaker_sdk_desktop_native.so";
}

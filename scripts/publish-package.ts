import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

type PackageManifest = {
  name: string;
  version: string;
};

const repoRoot = resolve(import.meta.dirname, "..");

function readValue(flagName: string): string | null {
  const prefixed = process.argv.find((value) => value.startsWith(`${flagName}=`));
  if (prefixed) {
    return prefixed.slice(flagName.length + 1);
  }

  const index = process.argv.indexOf(flagName);
  if (index === -1) {
    return null;
  }

  return process.argv[index + 1] ?? null;
}

function detectTag(version: string): string {
  const prereleaseIndex = version.indexOf("-");
  if (prereleaseIndex === -1) {
    return "latest";
  }

  const prerelease = version.slice(prereleaseIndex + 1);
  const firstIdentifier = prerelease.split(".")[0];

  if (!firstIdentifier) {
    return "next";
  }
  if (/^\d+$/.test(firstIdentifier)) {
    return "next";
  }
  return firstIdentifier;
}

function loadManifest(packageDir: string): PackageManifest {
  return JSON.parse(readFileSync(resolve(repoRoot, packageDir, "package.json"), "utf8")) as PackageManifest;
}

const packageName = readValue("--package");

if (!packageName) {
  console.error("Missing --package=<npm-name>");
  process.exit(1);
}

const packageDirs = [
  "packages/waker-config",
  "packages/waker-vad",
  "packages/waker-web",
];

const packageDir = packageDirs.find((dir) => loadManifest(dir).name === packageName);
if (!packageDir) {
  console.error(`Unknown package: ${packageName}`);
  process.exit(1);
}

const manifest = loadManifest(packageDir);
const tag = detectTag(manifest.version);

console.log(`${manifest.name}@${manifest.version}`);
console.log(`npm dist-tag: ${tag}`);

execFileSync("npm", ["publish", "--access", "public", "--tag", tag], {
  cwd: resolve(repoRoot, packageDir),
  stdio: "inherit",
});

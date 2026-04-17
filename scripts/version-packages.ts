import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

type BumpKind =
  | "major"
  | "minor"
  | "patch"
  | "premajor"
  | "preminor"
  | "prepatch"
  | "prerelease";

type PackageJson = {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  name: string;
  optionalDependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
  version: string;
};

type Version = {
  major: number;
  minor: number;
  patch: number;
  prerelease: Array<number | string>;
};

type WorkspacePackage = {
  dir: string;
  manifest: PackageJson;
  manifestPath: string;
};

const BUMP_KINDS = new Set<BumpKind>([
  "major",
  "minor",
  "patch",
  "premajor",
  "preminor",
  "prepatch",
  "prerelease",
]);
const DEFAULT_PREID = "alpha";
const repoRoot = resolve(import.meta.dirname, "..");

function readFlag(flagName: string): boolean {
  return process.argv.includes(flagName);
}

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

function parseVersion(version: string): Version {
  const match = version.match(
    /^(?<major>\d+)\.(?<minor>\d+)\.(?<patch>\d+)(?:-(?<prerelease>[0-9A-Za-z.-]+))?$/,
  );
  if (!match?.groups) {
    throw new Error(`Unsupported version format: ${version}`);
  }

  return {
    major: Number.parseInt(match.groups.major, 10),
    minor: Number.parseInt(match.groups.minor, 10),
    patch: Number.parseInt(match.groups.patch, 10),
    prerelease: match.groups.prerelease
      ? match.groups.prerelease.split(".").map((part) => {
          const numericValue = Number.parseInt(part, 10);
          return Number.isNaN(numericValue) || `${numericValue}` !== part ? part : numericValue;
        })
      : [],
  };
}

function formatVersion(version: Version): string {
  const stable = `${version.major}.${version.minor}.${version.patch}`;
  return version.prerelease.length > 0 ? `${stable}-${version.prerelease.join(".")}` : stable;
}

function incrementPrerelease(prerelease: Array<number | string>, preid: string): Array<number | string> {
  if (prerelease.length === 0) {
    return [preid, 0];
  }

  const currentPreid = typeof prerelease[0] === "string" ? prerelease[0] : preid;
  if (currentPreid !== preid) {
    return [preid, 0];
  }

  const lastValue = prerelease.at(-1);
  if (typeof lastValue === "number") {
    return [...prerelease.slice(0, -1), lastValue + 1];
  }

  return [...prerelease, 0];
}

function bumpVersion(currentVersion: string, bumpKind: BumpKind, preid: string): string {
  const parsed = parseVersion(currentVersion);

  switch (bumpKind) {
    case "major":
      return formatVersion({ major: parsed.major + 1, minor: 0, patch: 0, prerelease: [] });
    case "minor":
      return formatVersion({ major: parsed.major, minor: parsed.minor + 1, patch: 0, prerelease: [] });
    case "patch":
      return formatVersion({ major: parsed.major, minor: parsed.minor, patch: parsed.patch + 1, prerelease: [] });
    case "premajor":
      return formatVersion({ major: parsed.major + 1, minor: 0, patch: 0, prerelease: [preid, 0] });
    case "preminor":
      return formatVersion({ major: parsed.major, minor: parsed.minor + 1, patch: 0, prerelease: [preid, 0] });
    case "prepatch":
      return formatVersion({ major: parsed.major, minor: parsed.minor, patch: parsed.patch + 1, prerelease: [preid, 0] });
    case "prerelease":
      if (parsed.prerelease.length === 0) {
        return formatVersion({
          major: parsed.major,
          minor: parsed.minor,
          patch: parsed.patch + 1,
          prerelease: [preid, 0],
        });
      }
      return formatVersion({
        major: parsed.major,
        minor: parsed.minor,
        patch: parsed.patch,
        prerelease: incrementPrerelease(parsed.prerelease, preid),
      });
  }
}

function normalizePackageSelector(value: string): string {
  if (value === "all") {
    return value;
  }
  if (value.startsWith("@")) {
    return value;
  }
  return `@workspaces-team/${value}`;
}

function readPackageJson(manifestPath: string): PackageJson {
  return JSON.parse(readFileSync(manifestPath, "utf8")) as PackageJson;
}

function writePackageJson(manifestPath: string, payload: PackageJson) {
  writeFileSync(manifestPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

function loadWorkspacePackages(): WorkspacePackage[] {
  const packageDirs = ["packages/waker-config", "packages/waker-vad", "packages/waker-web"];
  return packageDirs.map((dir) => {
    const manifestPath = resolve(repoRoot, dir, "package.json");
    return {
      dir,
      manifest: readPackageJson(manifestPath),
      manifestPath,
    };
  });
}

function rewriteDependencyVersions(
  manifest: PackageJson,
  renamedPackages: Map<string, string>,
): { changed: boolean; nextManifest: PackageJson } {
  let changed = false;
  const nextManifest = { ...manifest };

  for (const fieldName of [
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "optionalDependencies",
  ] satisfies Array<keyof PackageJson>) {
    const field = manifest[fieldName];
    if (!field) {
      continue;
    }

    const nextField = { ...field };
    let fieldChanged = false;

    for (const [packageName, nextVersion] of renamedPackages) {
      const currentRange = nextField[packageName];
      if (!currentRange) {
        continue;
      }
      if (currentRange.startsWith("workspace:")) {
        continue;
      }

      let nextRange = nextVersion;
      if (currentRange.startsWith("^")) {
        nextRange = `^${nextVersion}`;
      } else if (currentRange.startsWith("~")) {
        nextRange = `~${nextVersion}`;
      }

      if (nextRange !== currentRange) {
        nextField[packageName] = nextRange;
        fieldChanged = true;
      }
    }

    if (fieldChanged) {
      changed = true;
      nextManifest[fieldName] = nextField;
    }
  }

  return { changed, nextManifest };
}

function printUsageAndExit(message: string): never {
  console.error(message);
  console.error(
    [
      "Usage:",
      "  bun ./scripts/version-packages.ts --package=all --bump=patch",
      "  bun ./scripts/version-packages.ts --package=waker-web --bump=preminor --preid=rc",
      "  bun ./scripts/version-packages.ts --package=all --set=0.2.0",
    ].join("\n"),
  );
  process.exit(1);
}

const packageSelector = normalizePackageSelector(readValue("--package") ?? "all");
const bumpKindValue = readValue("--bump");
const setVersion = readValue("--set");
const dryRun = readFlag("--dry-run");
const preid = readValue("--preid") ?? DEFAULT_PREID;

if (Boolean(bumpKindValue) === Boolean(setVersion)) {
  printUsageAndExit("Choose exactly one of --bump or --set.");
}

if (bumpKindValue && !BUMP_KINDS.has(bumpKindValue as BumpKind)) {
  printUsageAndExit(`Unsupported bump kind: ${bumpKindValue}`);
}

if (setVersion) {
  parseVersion(setVersion);
}

const workspacePackages = loadWorkspacePackages();
const selectedPackages = packageSelector === "all"
  ? workspacePackages
  : workspacePackages.filter((workspacePackage) => workspacePackage.manifest.name === packageSelector);

if (selectedPackages.length === 0) {
  printUsageAndExit(`Unknown package selector: ${packageSelector}`);
}

const currentVersions = [...new Set(selectedPackages.map((workspacePackage) => workspacePackage.manifest.version))];
if (packageSelector === "all" && currentVersions.length !== 1) {
  printUsageAndExit(
    `Refusing lockstep bump because package versions diverge: ${currentVersions.join(", ")}`,
  );
}

const nextVersion = setVersion ?? bumpVersion(currentVersions[0], bumpKindValue as BumpKind, preid);
const versionChanges = new Map<string, string>();

for (const workspacePackage of selectedPackages) {
  versionChanges.set(workspacePackage.manifest.name, nextVersion);
}

const updatedPackages = workspacePackages.map((workspacePackage) => {
  let nextManifest = { ...workspacePackage.manifest };
  let changed = false;

  const explicitNextVersion = versionChanges.get(workspacePackage.manifest.name);
  if (explicitNextVersion && explicitNextVersion !== workspacePackage.manifest.version) {
    nextManifest.version = explicitNextVersion;
    changed = true;
  }

  const dependencyRewrite = rewriteDependencyVersions(nextManifest, versionChanges);
  if (dependencyRewrite.changed) {
    nextManifest = dependencyRewrite.nextManifest;
    changed = true;
  }

  return {
    changed,
    currentVersion: workspacePackage.manifest.version,
    dir: workspacePackage.dir,
    manifestPath: workspacePackage.manifestPath,
    name: workspacePackage.manifest.name,
    nextManifest,
  };
});

for (const updatedPackage of updatedPackages) {
  if (!updatedPackage.changed || dryRun) {
    continue;
  }
  writePackageJson(updatedPackage.manifestPath, updatedPackage.nextManifest);
}

console.log(`version mode: ${dryRun ? "dry-run" : "apply"}`);
console.log(`selection: ${packageSelector}`);
console.log(`next version: ${nextVersion}`);
for (const updatedPackage of updatedPackages) {
  if (!updatedPackage.changed) {
    continue;
  }
  console.log(`${updatedPackage.name}: ${updatedPackage.currentVersion} -> ${updatedPackage.nextManifest.version}`);
}

console.log("");
console.log("Suggested next commands:");
console.log("corepack pnpm run build");
console.log("corepack pnpm run typecheck");
console.log("git diff -- package.json packages/*/package.json");

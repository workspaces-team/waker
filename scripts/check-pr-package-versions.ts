import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

type PackageRule = {
  extraWatchedPrefixes: string[];
  manifestPath: string;
  name: string;
  packageDir: string;
};

type PackageResult = {
  affectedFiles: string[];
  baseVersion: string | null;
  headVersion: string;
  name: string;
  versionChanged: boolean;
};

const repoRoot = resolve(import.meta.dirname, "..");
const DOC_ONLY_PACKAGE_FILES = new Set([
  ".gitignore",
  "CONTRIBUTING.md",
  "LICENSE",
  "README.md",
  "RELEASING.md",
]);

const PACKAGE_RULES: PackageRule[] = [
  {
    extraWatchedPrefixes: ["rust/sdk-wasm/"],
    manifestPath: "packages/waker-config/package.json",
    name: "@workspaces-team/waker-config",
    packageDir: "packages/waker-config",
  },
  {
    extraWatchedPrefixes: ["rust/vad-wasm/"],
    manifestPath: "packages/waker-vad/package.json",
    name: "@workspaces-team/waker-vad",
    packageDir: "packages/waker-vad",
  },
  {
    extraWatchedPrefixes: ["rust/sdk-wasm/"],
    manifestPath: "packages/waker-web/package.json",
    name: "@workspaces-team/waker-web",
    packageDir: "packages/waker-web",
  },
];

function runGit(args: string[]): string {
  return execFileSync("git", args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function readFlagValue(flagName: string): string | null {
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

function readVersionFromFile(manifestPath: string): string {
  const manifest = JSON.parse(
    readFileSync(resolve(repoRoot, manifestPath), "utf8"),
  ) as { version?: string };

  if (!manifest.version) {
    throw new Error(`Missing version in ${manifestPath}`);
  }

  return manifest.version;
}

function readVersionFromRef(ref: string, manifestPath: string): string | null {
  try {
    const manifest = JSON.parse(runGit(["show", `${ref}:${manifestPath}`])) as { version?: string };
    if (!manifest.version) {
      throw new Error(`Missing version in ${ref}:${manifestPath}`);
    }
    return manifest.version;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (message.includes(`exists on disk, but not in '${ref}'`) || message.includes(`path '${manifestPath}' does not exist in '${ref}'`)) {
      return null;
    }
    throw error;
  }
}

function renderBaseVersion(baseVersion: string | null): string {
  return baseVersion ?? "<new package>";
}

function didVersionChange(baseVersion: string | null, headVersion: string): boolean {
  if (baseVersion === null) {
    return true;
  }

  return baseVersion !== headVersion;
}

function isPackageAffectingFile(filePath: string, rule: PackageRule): boolean {
  if (rule.extraWatchedPrefixes.some((prefix) => filePath.startsWith(prefix))) {
    return true;
  }

  if (!filePath.startsWith(`${rule.packageDir}/`)) {
    return false;
  }

  const packageRelativePath = filePath.slice(rule.packageDir.length + 1);
  return !DOC_ONLY_PACKAGE_FILES.has(packageRelativePath);
}

function printFailure(baseRef: string, failures: PackageResult[]) {
  console.error(`Package version policy failed against ${baseRef}.`);
  console.error("");
  console.error("Each publishable package change in a pull request must bump that package version.");
  console.error("Docs-only changes under package directories are ignored.");
  console.error("");

  for (const failure of failures) {
    console.error(`${failure.name}: version did not change (${failure.headVersion})`);
    console.error(`base version: ${renderBaseVersion(failure.baseVersion)}`);
    console.error("changed files:");
    for (const filePath of failure.affectedFiles) {
      console.error(`- ${filePath}`);
    }
    console.error("");
  }

  console.error("Suggested fix:");
  console.error("pnpm run version:packages -- --package=all --bump=patch");
}

const baseRef = readFlagValue("--base-ref") ??
  (process.env.GITHUB_BASE_REF ? `origin/${process.env.GITHUB_BASE_REF}` : null);

if (!baseRef) {
  console.error("Missing --base-ref. Example: bun ./scripts/check-pr-package-versions.ts --base-ref origin/main");
  process.exit(1);
}

runGit(["rev-parse", "--verify", baseRef]);
const mergeBase = runGit(["merge-base", baseRef, "HEAD"]);

const changedFiles = runGit([
  "diff",
  "--name-only",
  "--diff-filter=ACMR",
  mergeBase,
]).split("\n").filter(Boolean);

if (changedFiles.length === 0) {
  console.log(`No file changes detected compared to ${baseRef}.`);
  process.exit(0);
}

const results: PackageResult[] = PACKAGE_RULES.map((rule) => {
  const affectedFiles = changedFiles.filter((filePath) => isPackageAffectingFile(filePath, rule));
  const baseVersion = readVersionFromRef(mergeBase, rule.manifestPath);
  const headVersion = readVersionFromFile(rule.manifestPath);

  return {
    affectedFiles,
    baseVersion,
    headVersion,
    name: rule.name,
    versionChanged: didVersionChange(baseVersion, headVersion),
  };
});

const failures = results.filter((result) => result.affectedFiles.length > 0 && !result.versionChanged);
if (failures.length > 0) {
  printFailure(baseRef, failures);
  process.exit(1);
}

const affectedPackages = results.filter((result) => result.affectedFiles.length > 0);
if (affectedPackages.length === 0) {
  console.log(`No publishable package changes detected compared to ${baseRef}.`);
  process.exit(0);
}

console.log(`Package version policy passed against ${baseRef}.`);
for (const result of affectedPackages) {
  console.log(`${result.name}: ${renderBaseVersion(result.baseVersion)} -> ${result.headVersion}`);
}

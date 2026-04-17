import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";

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

const sourceExtension = readValue("--source-extension");
const destination = readValue("--destination");

if (!sourceExtension || !destination) {
  console.error(
    "Usage: bun ./scripts/sync-rust-source.ts --source-extension=<name> --destination=<relative-path>",
  );
  process.exit(1);
}

const repoRoot = resolve(import.meta.dirname, "..");
const sourceRepoCandidates = [
  process.env.WAKER_SOURCE_REPO ? resolve(process.env.WAKER_SOURCE_REPO) : null,
  resolve(repoRoot, "../.."),
  resolve(repoRoot, "../../../waker"),
].filter((value): value is string => Boolean(value));
const sourceRepoRoot = sourceRepoCandidates.find((candidateRoot) =>
  existsSync(resolve(candidateRoot, "lib/extensions", sourceExtension)),
);

if (!sourceRepoRoot) {
  console.error(
    [
      `Could not locate source repo for ${sourceExtension}.`,
      "Set WAKER_SOURCE_REPO to a compatible private workspace or run this repo from the managed monorepo checkout.",
      `Checked roots: ${sourceRepoCandidates.join(", ")}`,
    ].join("\n"),
  );
  process.exit(1);
}

const sourceRoot = resolve(sourceRepoRoot, "lib/extensions", sourceExtension);
const destinationRoot = resolve(repoRoot, destination);

rmSync(destinationRoot, { force: true, recursive: true });
mkdirSync(dirname(destinationRoot), { recursive: true });

cpSync(sourceRoot, destinationRoot, {
  filter: (entryPath) => {
    const relativePath = entryPath.startsWith(sourceRoot)
      ? entryPath.slice(sourceRoot.length).replace(/^\/+/, "")
      : "";
    return relativePath === "" || (!relativePath.startsWith("pkg/") && !relativePath.startsWith("target/"));
  },
  recursive: true,
});

console.log(
  [
    `Synced Rust source tree`,
    `extension: ${sourceExtension}`,
    `source: ${sourceRoot}`,
    `destination: ${destinationRoot}`,
  ].join("\n"),
);

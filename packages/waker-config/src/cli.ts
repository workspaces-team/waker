#!/usr/bin/env node

import { existsSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

import {
  createDefaultWakerHeadTrainingConfig,
  serializeWakerHeadTrainingConfig,
} from "./config-template";
import type { WakerBundledRegistrationPolicy } from "./types";

type ParsedArgs = {
  acceptedWakeForms: string[];
  command: "help" | "init";
  force: boolean;
  keyword: string | null;
  outPath: string;
  registrationPolicy: WakerBundledRegistrationPolicy;
  siblingNegativeForms: string[];
  stdout: boolean;
  structuralConfusables: string[];
};

const DEFAULT_OUT_PATH = "waker-head.config.json";

function printHelp(): void {
  console.log(`waker-config

Generate a starter Waker tiny-head config file.

Usage:
  npx @workspaces-team/waker-config --keyword "Operator"
  npx @workspaces-team/waker-config init --keyword "Operator" --out ./waker-head.config.json

Options:
  --keyword <value>                Required single wake word.
  --out <path>                     Output path. Default: ${DEFAULT_OUT_PATH}
  --policy <policy>                Registration policy. Default: single_word_only
  --accepted-form <value>          Add an accepted wake form. Repeatable.
  --sibling-negative <value>       Add a sibling negative phrase. Repeatable.
  --structural-confusable <value>  Add a structural confusable phrase. Repeatable.
  --stdout                         Print config to stdout instead of writing a file.
  --force                          Overwrite the output file if it already exists.
  -h, --help                       Show this help text.
`);
}

function expectValue(args: string[], flag: string, index: number): string {
  const value = args[index + 1];
  if (!value || value.startsWith("-")) {
    throw new Error(`Missing value for ${flag}.`);
  }
  return value;
}

function parsePolicy(value: string): WakerBundledRegistrationPolicy {
  if (value === "single_word_only") {
    return value;
  }
  throw new Error(
    `Unsupported policy "${value}". Expected: single_word_only.`,
  );
}

function parseArgs(argv: string[]): ParsedArgs {
  const args = [...argv];
  let command: "help" | "init" = "init";
  if (args[0] && !args[0].startsWith("-")) {
    const candidate = args.shift();
    if (candidate === "help") {
      command = "help";
    } else if (candidate === "init") {
      command = "init";
    } else {
      throw new Error(`Unknown command "${candidate}".`);
    }
  }

  const parsed: ParsedArgs = {
    acceptedWakeForms: [],
    command,
    force: false,
    keyword: null,
    outPath: DEFAULT_OUT_PATH,
    registrationPolicy: "single_word_only",
    siblingNegativeForms: [],
    stdout: false,
    structuralConfusables: [],
  };

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    switch (arg) {
      case "-h":
      case "--help":
        parsed.command = "help";
        break;
      case "--force":
        parsed.force = true;
        break;
      case "--stdout":
        parsed.stdout = true;
        break;
      case "--keyword":
        parsed.keyword = expectValue(args, arg, index);
        index += 1;
        break;
      case "--out":
        parsed.outPath = expectValue(args, arg, index);
        index += 1;
        break;
      case "--policy":
        parsed.registrationPolicy = parsePolicy(expectValue(args, arg, index));
        index += 1;
        break;
      case "--accepted-form":
        parsed.acceptedWakeForms.push(expectValue(args, arg, index));
        index += 1;
        break;
      case "--sibling-negative":
        parsed.siblingNegativeForms.push(expectValue(args, arg, index));
        index += 1;
        break;
      case "--structural-confusable":
        parsed.structuralConfusables.push(expectValue(args, arg, index));
        index += 1;
        break;
      default:
        throw new Error(`Unknown argument "${arg}".`);
    }
  }

  return parsed;
}

function run(): number {
  try {
    const parsed = parseArgs(process.argv.slice(2));
    if (parsed.command === "help") {
      printHelp();
      return 0;
    }
    if (!parsed.keyword) {
      throw new Error("Missing required --keyword value.");
    }

    const config = createDefaultWakerHeadTrainingConfig({
      keyword: parsed.keyword,
      registrationPolicy: parsed.registrationPolicy,
      acceptedWakeForms:
        parsed.acceptedWakeForms.length > 0 ? parsed.acceptedWakeForms : undefined,
      siblingNegativeForms:
        parsed.siblingNegativeForms.length > 0 ? parsed.siblingNegativeForms : undefined,
      structuralConfusables:
        parsed.structuralConfusables.length > 0 ? parsed.structuralConfusables : undefined,
    });
    const serialized = serializeWakerHeadTrainingConfig(config);

    if (parsed.stdout) {
      process.stdout.write(serialized);
      return 0;
    }

    const resolvedOutPath = resolve(process.cwd(), parsed.outPath);
    if (existsSync(resolvedOutPath) && !parsed.force) {
      throw new Error(
        `Refusing to overwrite ${parsed.outPath}. Re-run with --force or choose a different --out path.`,
      );
    }
    writeFileSync(resolvedOutPath, serialized, "utf8");
    process.stdout.write(
      `Wrote ${parsed.outPath}\nNext: import this JSON into your app and pass it to trainFromClips(...).\n`,
    );
    return 0;
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.stderr.write("Run with --help for usage.\n");
    return 1;
  }
}

process.exitCode = run();

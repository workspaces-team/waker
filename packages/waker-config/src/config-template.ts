import type {
  WakerBrowserHeadTrainingConfig,
  WakerBundledRegistrationPolicy,
} from "./types";

export type WakerHeadTrainingConfigTemplateOptions = {
  acceptedWakeForms?: string[];
  keyword: string;
  registrationPolicy?: WakerBundledRegistrationPolicy;
  siblingNegativeForms?: string[];
  structuralConfusables?: string[];
};

function normalizeKeyword(keyword: string): string {
  return keyword
    .split(/\s+/)
    .filter((segment) => segment.length > 0)
    .join(" ")
    .toLowerCase();
}

function defaultAcceptedWakeForms(
  normalizedKeyword: string,
  registrationPolicy: WakerBundledRegistrationPolicy,
): string[] {
  switch (registrationPolicy) {
    case "bare_plus_prefix":
    case "single_word_plus_prefix": {
      const prefixed = normalizedKeyword.startsWith("hey ")
        ? normalizedKeyword
        : `hey ${normalizedKeyword}`;
      return prefixed === normalizedKeyword
        ? [normalizedKeyword]
        : [normalizedKeyword, prefixed];
    }
    default:
      return [normalizedKeyword];
  }
}

function defaultSiblingNegativeForms(normalizedKeyword: string): string[] {
  if (normalizedKeyword.startsWith("hey ")) {
    return [];
  }
  return [
    `hi ${normalizedKeyword}`,
    `hello ${normalizedKeyword}`,
    `say ${normalizedKeyword}`,
    `hey ${normalizedKeyword} please`,
  ];
}

export function createDefaultWakerHeadTrainingConfig(
  options: WakerHeadTrainingConfigTemplateOptions,
): WakerBrowserHeadTrainingConfig {
  const registrationPolicy = options.registrationPolicy ?? "single_word_only";
  const normalizedKeyword = normalizeKeyword(options.keyword);
  return {
    keyword: options.keyword,
    registrationPolicy,
    acceptedWakeForms:
      options.acceptedWakeForms ?? defaultAcceptedWakeForms(normalizedKeyword, registrationPolicy),
    siblingNegativeForms:
      options.siblingNegativeForms ?? defaultSiblingNegativeForms(normalizedKeyword),
    structuralConfusables: options.structuralConfusables ?? [],
    epochs: 32,
    learningRate: 0.08,
    negativeWeight: 1.5,
    l2Reg: 1e-4,
    validationSplit: 0.2,
    detector: {
      hiddenWidth: 128,
      dilations: [1, 2, 4],
      smoothScale: 0.6,
      edgeScale: 0.25,
      accelScale: 0.1,
    },
  };
}

export function serializeWakerHeadTrainingConfig(
  config: WakerBrowserHeadTrainingConfig,
): string {
  return `${JSON.stringify(config, null, 2)}\n`;
}

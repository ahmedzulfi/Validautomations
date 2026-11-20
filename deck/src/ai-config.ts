export const MODELS = [
  { id: "claude-opus-5-5", label: "Claude Opus 5.5 (best)" },
  { id: "claude-sonnet-5-5", label: "Claude Sonnet 5.5 (faster)" },
  { id: "claude-haiku-5-5", label: "Claude Haiku 5.5 (cheapest)" },
] as const;

export const DEFAULT_MODEL = "claude-opus-5-5";

export interface AISettings {
  apiKey: string;
  model: string;
}

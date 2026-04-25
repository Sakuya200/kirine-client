export const QWEN3_TTS_DEFAULT_PARAMS = {
  voicePrompt: ''
} as const;

export const createQwen3TtsModelParams = (): Record<string, unknown> => ({
  ...QWEN3_TTS_DEFAULT_PARAMS
});

export const normalizeQwen3TtsModelParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...QWEN3_TTS_DEFAULT_PARAMS,
  ...modelParams
});

export const buildQwen3TtsGenerationSummaryLines = (modelParams: Record<string, unknown>): string[] => {
  const voicePrompt = String(modelParams.voicePrompt ?? '').trim();
  return [voicePrompt ? `声音 Prompt：${voicePrompt}` : '未填写声音 Prompt，将使用默认声音风格。'];
};

export const buildQwen3TtsResultSummaryLines = (modelParams: Record<string, unknown>): string[] => {
  const voicePrompt = String(modelParams.voicePrompt ?? '').trim();
  return voicePrompt ? [`声音 Prompt：${voicePrompt}`] : [];
};

export const VOX_CPM2_TTS_DEFAULT_PARAMS = {
  cfgValue: 2.0,
  inferenceTimesteps: 10
} as const;

export const createVoxCpm2TtsModelParams = (): Record<string, unknown> => ({
  ...VOX_CPM2_TTS_DEFAULT_PARAMS
});

export const normalizeVoxCpm2TtsModelParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...VOX_CPM2_TTS_DEFAULT_PARAMS,
  ...modelParams,
  cfgValue: Number(modelParams.cfgValue ?? VOX_CPM2_TTS_DEFAULT_PARAMS.cfgValue),
  inferenceTimesteps: Number(modelParams.inferenceTimesteps ?? VOX_CPM2_TTS_DEFAULT_PARAMS.inferenceTimesteps)
});

export const buildVoxCpm2TtsGenerationSummaryLines = (modelParams: Record<string, unknown>): string[] => {
  const normalized = normalizeVoxCpm2TtsModelParams(modelParams);
  return [
    `当前 CFG=${normalized.cfgValue ?? VOX_CPM2_TTS_DEFAULT_PARAMS.cfgValue}，推理步数=${normalized.inferenceTimesteps ?? VOX_CPM2_TTS_DEFAULT_PARAMS.inferenceTimesteps}。`
  ];
};

export const buildVoxCpm2TtsResultSummaryLines = (modelParams: Record<string, unknown>): string[] => {
  const normalized = normalizeVoxCpm2TtsModelParams(modelParams);
  return [
    `CFG ${normalized.cfgValue ?? VOX_CPM2_TTS_DEFAULT_PARAMS.cfgValue} · 步数 ${normalized.inferenceTimesteps ?? VOX_CPM2_TTS_DEFAULT_PARAMS.inferenceTimesteps}`
  ];
};

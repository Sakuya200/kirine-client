export const MOSS_TTS_LOCAL_TTS_DEFAULT_PARAMS = {
  nVqForInference: 32
} as const;

export const createMossTtsLocalTtsModelParams = (): Record<string, unknown> => ({
  ...MOSS_TTS_LOCAL_TTS_DEFAULT_PARAMS
});

export const normalizeMossTtsLocalTtsModelParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...MOSS_TTS_LOCAL_TTS_DEFAULT_PARAMS,
  ...modelParams,
  nVqForInference: Number(modelParams.nVqForInference ?? MOSS_TTS_LOCAL_TTS_DEFAULT_PARAMS.nVqForInference)
});

export const buildMossTtsLocalTtsGenerationSummaryLines = (modelParams: Record<string, unknown>): string[] => {
  const normalized = normalizeMossTtsLocalTtsModelParams(modelParams);
  return [`当前并行码本数 nVQ=${normalized.nVqForInference ?? MOSS_TTS_LOCAL_TTS_DEFAULT_PARAMS.nVqForInference}。`];
};

export const buildMossTtsLocalTtsResultSummaryLines = (modelParams: Record<string, unknown>): string[] => {
  const normalized = normalizeMossTtsLocalTtsModelParams(modelParams);
  return [`并行码本数 nVQ：${normalized.nVqForInference ?? MOSS_TTS_LOCAL_TTS_DEFAULT_PARAMS.nVqForInference}`];
};

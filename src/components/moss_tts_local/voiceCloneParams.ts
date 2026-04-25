export const MOSS_TTS_LOCAL_VOICE_CLONE_DEFAULT_PARAMS = {
  nVqForInference: 32
} as const;

export const createMossVoiceCloneParams = (): Record<string, unknown> => ({
  ...MOSS_TTS_LOCAL_VOICE_CLONE_DEFAULT_PARAMS
});

export const normalizeMossVoiceCloneParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...MOSS_TTS_LOCAL_VOICE_CLONE_DEFAULT_PARAMS,
  ...modelParams,
  nVqForInference: Number(modelParams.nVqForInference ?? MOSS_TTS_LOCAL_VOICE_CLONE_DEFAULT_PARAMS.nVqForInference)
});

export const buildMossVoiceCloneSummaryLines = (modelParams: Record<string, unknown>): string[] => {
  const normalized = normalizeMossVoiceCloneParams(modelParams);
  return [`当前并行码本数 nVQ=${normalized.nVqForInference ?? MOSS_TTS_LOCAL_VOICE_CLONE_DEFAULT_PARAMS.nVqForInference}。`];
};

export const buildMossVoiceCloneResultSummaryLines = (modelParams: Record<string, unknown>): string[] => {
  const normalized = normalizeMossVoiceCloneParams(modelParams);
  return [`并行码本数 nVQ：${normalized.nVqForInference ?? MOSS_TTS_LOCAL_VOICE_CLONE_DEFAULT_PARAMS.nVqForInference}`];
};

export const VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS = {
  mode: 'reference',
  stylePrompt: '',
  cfgValue: 2.0,
  inferenceTimesteps: 10
} as const;

export const createVoxCpm2VoiceCloneParams = (): Record<string, unknown> => ({
  ...VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS
});

export const normalizeVoxCpm2VoiceCloneParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS,
  ...modelParams,
  mode: String(modelParams.mode ?? VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS.mode),
  stylePrompt: String(modelParams.stylePrompt ?? VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS.stylePrompt),
  cfgValue: Number(modelParams.cfgValue ?? VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS.cfgValue),
  inferenceTimesteps: Number(modelParams.inferenceTimesteps ?? VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS.inferenceTimesteps)
});

export const buildVoxCpm2VoiceCloneSummaryLines = (modelParams: Record<string, unknown>): string[] => {
  const normalized = normalizeVoxCpm2VoiceCloneParams(modelParams);
  const lines = [
    `当前 CFG=${normalized.cfgValue ?? VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS.cfgValue}，推理步数=${normalized.inferenceTimesteps ?? VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS.inferenceTimesteps}。`
  ];
  const stylePrompt = String(normalized.stylePrompt ?? '').trim();

  if (stylePrompt) {
    lines.push(`风格提示词：${stylePrompt}`);
  }

  return lines;
};

export const buildVoxCpm2VoiceCloneResultSummaryLines = (modelParams: Record<string, unknown>): string[] => {
  const normalized = normalizeVoxCpm2VoiceCloneParams(modelParams);
  const lines = [`克隆模式：${normalized.mode === 'ultimate' ? 'Ultimate' : 'Reference'}`];
  const stylePrompt = String(normalized.stylePrompt ?? '').trim();

  if (stylePrompt) {
    lines.push(`风格提示词：${stylePrompt}`);
  }

  return lines;
};

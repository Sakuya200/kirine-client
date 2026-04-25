export const VOX_CPM2_TRAINING_DEFAULT_PARAMS = {
  trainingMode: 'lora',
  useLora: true,
  loraRank: 32,
  loraAlpha: 32,
  loraDropout: '0.0',
  epochCount: 2,
  batchSize: 4,
  gradientAccumulationSteps: 1,
  enableGradientCheckpointing: false
} as const;

export const createVoxCpm2TrainingParams = (): Record<string, unknown> => ({
  ...VOX_CPM2_TRAINING_DEFAULT_PARAMS
});

export const normalizeVoxCpm2TrainingParams = (modelParams: Record<string, unknown>): Record<string, unknown> => {
  const legacyTrainingMode = String(modelParams.trainingMode ?? '').trim();
  const useLora = typeof modelParams.useLora === 'boolean' ? modelParams.useLora : legacyTrainingMode !== 'full';

  return {
    ...VOX_CPM2_TRAINING_DEFAULT_PARAMS,
    ...modelParams,
    useLora,
    trainingMode: useLora ? 'lora' : 'full',
    loraRank: Number(modelParams.loraRank ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraRank),
    loraAlpha: Number(modelParams.loraAlpha ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraAlpha),
    loraDropout:
      String(modelParams.loraDropout ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraDropout).trim() || VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraDropout
  };
};

export const buildVoxCpm2TrainingSummaryLines = (modelParams: Record<string, unknown>): string[] => {
  const normalized = normalizeVoxCpm2TrainingParams(modelParams);

  if (!normalized.useLora) {
    return ['当前微调模式 全量微调。'];
  }

  return [
    '当前微调模式 LoRA 微调。',
    `LoRA 参数 rank ${normalized.loraRank ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraRank}，alpha ${normalized.loraAlpha ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraAlpha}，dropout ${normalized.loraDropout ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraDropout}。`
  ];
};

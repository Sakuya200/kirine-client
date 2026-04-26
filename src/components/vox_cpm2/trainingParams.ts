export const VOX_CPM2_TRAINING_DEFAULT_PARAMS = {
  trainingMode: 'lora',
  useLora: true,
  loraRank: 32,
  loraAlpha: 32,
  loraDropout: '0.0',
  epochCount: 2,
  batchSize: 4,
  gradientAccumulationSteps: 1,
  enableGradientCheckpointing: false,
  learningRate: '1e-4',
  weightDecay: '0.01',
  warmupSteps: null
} as const;

export const createVoxCpm2TrainingParams = (): Record<string, unknown> => ({
  ...VOX_CPM2_TRAINING_DEFAULT_PARAMS
});

export const normalizeVoxCpm2TrainingParams = (modelParams: Record<string, unknown>): Record<string, unknown> => {
  const legacyTrainingMode = String(modelParams.trainingMode ?? '').trim();
  const useLora = typeof modelParams.useLora === 'boolean' ? modelParams.useLora : legacyTrainingMode !== 'full';
  const rawWarmupSteps = modelParams.warmupSteps;

  return {
    ...VOX_CPM2_TRAINING_DEFAULT_PARAMS,
    ...modelParams,
    useLora,
    trainingMode: useLora ? 'lora' : 'full',
    loraRank: Number(modelParams.loraRank ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraRank),
    loraAlpha: Number(modelParams.loraAlpha ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraAlpha),
    loraDropout:
      String(modelParams.loraDropout ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraDropout).trim() || VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraDropout,
    learningRate:
      String(modelParams.learningRate ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.learningRate).trim() || VOX_CPM2_TRAINING_DEFAULT_PARAMS.learningRate,
    weightDecay:
      String(modelParams.weightDecay ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.weightDecay).trim() || VOX_CPM2_TRAINING_DEFAULT_PARAMS.weightDecay,
    warmupSteps: rawWarmupSteps == null || String(rawWarmupSteps).trim().length === 0 ? null : Math.max(0, Number(rawWarmupSteps))
  };
};

export const buildVoxCpm2TrainingSummaryLines = (modelParams: Record<string, unknown>): string[] => {
  const normalized = normalizeVoxCpm2TrainingParams(modelParams);
  const modeLine = `当前微调模式 ${normalized.useLora ? 'LoRA 微调' : '全量微调'}。`;
  const optimizerLine = `学习率 ${normalized.learningRate ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.learningRate}，权重衰减 ${normalized.weightDecay ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.weightDecay}，Warmup Steps ${normalized.warmupSteps ?? '自动'}。`;

  if (!normalized.useLora) {
    return [modeLine, optimizerLine];
  }

  return [
    modeLine,
    optimizerLine,
    `LoRA 参数 rank ${normalized.loraRank ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraRank}，alpha ${normalized.loraAlpha ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraAlpha}，dropout ${normalized.loraDropout ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraDropout}。`
  ];
};

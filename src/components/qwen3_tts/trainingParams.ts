export const QWEN3_TRAINING_DEFAULT_PARAMS = {
  epochCount: 30,
  batchSize: 8,
  gradientAccumulationSteps: 4,
  enableGradientCheckpointing: false,
  learningRate: '2e-5'
} as const;

export const createQwen3TrainingParams = (): Record<string, unknown> => ({
  ...QWEN3_TRAINING_DEFAULT_PARAMS
});

export const normalizeQwen3TrainingParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...QWEN3_TRAINING_DEFAULT_PARAMS,
  ...modelParams,
  learningRate: String(modelParams.learningRate ?? QWEN3_TRAINING_DEFAULT_PARAMS.learningRate).trim() || QWEN3_TRAINING_DEFAULT_PARAMS.learningRate
});

export const buildQwen3TrainingSummaryLines = (modelParams: Record<string, unknown>): string[] => {
  const normalized = normalizeQwen3TrainingParams(modelParams);

  return [
    `学习率 ${normalized.learningRate ?? QWEN3_TRAINING_DEFAULT_PARAMS.learningRate}，梯度检查点 ${normalized.enableGradientCheckpointing ? '启用' : '禁用'}。`
  ];
};

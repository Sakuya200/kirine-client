export const QWEN3_TRAINING_DEFAULT_PARAMS = {
  epochCount: 30,
  batchSize: 8,
  gradientAccumulationSteps: 4,
  enableGradientCheckpointing: false
} as const;

export const createQwen3TrainingParams = (): Record<string, unknown> => ({
  ...QWEN3_TRAINING_DEFAULT_PARAMS
});

export const normalizeQwen3TrainingParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...QWEN3_TRAINING_DEFAULT_PARAMS,
  ...modelParams
});

export const buildQwen3TrainingSummaryLines = (): string[] => [];

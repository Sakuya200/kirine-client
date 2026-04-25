export const MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS = {
  epochCount: 3,
  batchSize: 1,
  gradientAccumulationSteps: 8,
  enableGradientCheckpointing: true,
  learningRate: 1e-5,
  weightDecay: 0.1,
  warmupRatio: 0.03,
  warmupSteps: 0,
  maxGradNorm: 1.0,
  mixedPrecision: 'bf16',
  channelwiseLossWeight: '1,32',
  skipReferenceAudioCodes: true,
  prepBatchSize: 16,
  prepNVq: null
} as const;

export const createMossTtsLocalTrainingParams = (): Record<string, unknown> => ({
  ...MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS
});

export const normalizeMossTtsLocalTrainingParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS,
  ...modelParams
});

export const buildMossTtsLocalTrainingSummaryLines = (modelParams: Record<string, unknown>): string[] => {
  const normalized = normalizeMossTtsLocalTrainingParams(modelParams);

  return [
    `学习率 ${normalized.learningRate ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.learningRate}，权重衰减 ${normalized.weightDecay ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.weightDecay}，混合精度 ${normalized.mixedPrecision ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.mixedPrecision}。`,
    `预处理批次 ${normalized.prepBatchSize ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.prepBatchSize}，预处理 nVQ ${normalized.prepNVq ?? '默认'}，参考音频编码 ${normalized.skipReferenceAudioCodes ? '跳过' : '保留'}。`
  ];
};

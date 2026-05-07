interface TrainingModelRegistryEntry {
  normalizeParams: (modelParams: Record<string, unknown>) => Record<string, unknown>;
}

const QWEN3_TRAINING_DEFAULT_PARAMS = {
  epochCount: 30,
  batchSize: 8,
  gradientAccumulationSteps: 4,
  enableGradientCheckpointing: false,
  learningRate: '2e-5'
} as const;

const VOX_CPM2_TRAINING_DEFAULT_PARAMS = {
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

const MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS = {
  epochCount: 3,
  batchSize: 1,
  gradientAccumulationSteps: 8,
  enableGradientCheckpointing: true,
  learningRate: '1e-5',
  weightDecay: '0.1',
  warmupRatio: '0.03',
  warmupSteps: 0,
  maxGradNorm: '1.0',
  mixedPrecision: 'bf16',
  channelwiseLossWeight: '1,32',
  skipReferenceAudioCodes: true,
  prepBatchSize: 16,
  prepNVq: null
} as const;

const normalizeQwen3TrainingParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...QWEN3_TRAINING_DEFAULT_PARAMS,
  ...modelParams,
  learningRate: String(modelParams.learningRate ?? QWEN3_TRAINING_DEFAULT_PARAMS.learningRate).trim() || QWEN3_TRAINING_DEFAULT_PARAMS.learningRate
});

const normalizeVoxCpm2TrainingParams = (modelParams: Record<string, unknown>): Record<string, unknown> => {
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

const normalizeMossTtsLocalTrainingParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS,
  ...modelParams,
  learningRate:
    String(modelParams.learningRate ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.learningRate).trim() ||
    MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.learningRate,
  weightDecay:
    String(modelParams.weightDecay ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.weightDecay).trim() ||
    MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.weightDecay,
  warmupRatio:
    String(modelParams.warmupRatio ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.warmupRatio).trim() ||
    MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.warmupRatio,
  maxGradNorm:
    String(modelParams.maxGradNorm ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.maxGradNorm).trim() || MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.maxGradNorm
});

const defaultEntry: TrainingModelRegistryEntry = {
  normalizeParams: normalizeQwen3TrainingParams
};

const MODEL_TRAINING_REGISTRY: Record<string, TrainingModelRegistryEntry> = {
  qwen3_tts: defaultEntry,
  vox_cpm2: {
    normalizeParams: normalizeVoxCpm2TrainingParams
  },
  moss_tts_local: {
    normalizeParams: normalizeMossTtsLocalTrainingParams
  }
};

export const getTrainingModelRegistryEntry = (baseModel: string): TrainingModelRegistryEntry => MODEL_TRAINING_REGISTRY[baseModel] ?? defaultEntry;

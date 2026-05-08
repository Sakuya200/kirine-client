interface TrainingModelRegistryEntry {
  normalizeParams: (modelParams: Record<string, unknown>) => Record<string, unknown>;
}

const normalizeQwen3TrainingParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...modelParams,
  learningRate: String(modelParams.learningRate ?? '').trim()
});

const normalizeVoxCpm2TrainingParams = (modelParams: Record<string, unknown>): Record<string, unknown> => {
  const legacyTrainingMode = String(modelParams.trainingMode ?? '').trim();
  const useLora = typeof modelParams.useLora === 'boolean' ? modelParams.useLora : legacyTrainingMode !== 'full';
  const rawWarmupSteps = modelParams.warmupSteps;

  return {
    ...modelParams,
    useLora,
    trainingMode: useLora ? 'lora' : 'full',
    loraRank: Number(modelParams.loraRank ?? 0),
    loraAlpha: Number(modelParams.loraAlpha ?? 0),
    loraDropout: String(modelParams.loraDropout ?? '').trim(),
    learningRate: String(modelParams.learningRate ?? '').trim(),
    weightDecay: String(modelParams.weightDecay ?? '').trim(),
    warmupSteps: rawWarmupSteps == null || String(rawWarmupSteps).trim().length === 0 ? null : Math.max(0, Number(rawWarmupSteps))
  };
};

const normalizeMossTtsLocalTrainingParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...modelParams,
  learningRate: String(modelParams.learningRate ?? '').trim(),
  weightDecay: String(modelParams.weightDecay ?? '').trim(),
  warmupRatio: String(modelParams.warmupRatio ?? '').trim(),
  maxGradNorm: String(modelParams.maxGradNorm ?? '').trim()
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

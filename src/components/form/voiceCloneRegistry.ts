interface VoiceCloneModelRegistryEntry {
  normalizeParams: (modelParams: Record<string, unknown>) => Record<string, unknown>;
  requiresReferenceText: (modelParams: Record<string, unknown>) => boolean;
}

const normalizeQwen3VoiceCloneParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...modelParams
});

const normalizeVoxCpm2VoiceCloneParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...modelParams,
  mode: String(modelParams.mode ?? ''),
  stylePrompt: String(modelParams.stylePrompt ?? ''),
  cfgValue: String(modelParams.cfgValue ?? '').trim(),
  inferenceTimesteps: Number(modelParams.inferenceTimesteps ?? 0)
});

const normalizeMossVoiceCloneParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...modelParams,
  nVqForInference: Number(modelParams.nVqForInference ?? 0)
});

const normalizeGptSovitsV2ppVoiceCloneParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...modelParams,
  refAudioPath: String(modelParams.refAudioPath ?? ''),
  refTextPath: String(modelParams.refTextPath ?? ''),
  promptLang: String(modelParams.promptLang ?? ''),
  topK: Number(modelParams.topK ?? 0),
  topP: String(modelParams.topP ?? '').trim(),
  temperature: String(modelParams.temperature ?? '').trim(),
  speedFactor: String(modelParams.speedFactor ?? '').trim(),
  textSplitMethod: String(modelParams.textSplitMethod ?? ''),
  batchSize: Number(modelParams.batchSize ?? 0),
  splitBucket: Boolean(modelParams.splitBucket),
  fragmentInterval: String(modelParams.fragmentInterval ?? '').trim(),
  parallelInfer: Boolean(modelParams.parallelInfer),
  seed: Number(modelParams.seed ?? 0)
});

const defaultEntry: VoiceCloneModelRegistryEntry = {
  normalizeParams: normalizeQwen3VoiceCloneParams,
  requiresReferenceText: () => true
};

const VOICE_CLONE_MODEL_REGISTRY: Record<string, VoiceCloneModelRegistryEntry> = {
  qwen3_tts: defaultEntry,
  vox_cpm2: {
    normalizeParams: normalizeVoxCpm2VoiceCloneParams,
    requiresReferenceText: modelParams => String(normalizeVoxCpm2VoiceCloneParams(modelParams).mode) === 'ultimate'
  },
  moss_tts_local: {
    normalizeParams: normalizeMossVoiceCloneParams,
    requiresReferenceText: () => false
  },
  gpt_sovits_v2pp: {
    normalizeParams: normalizeGptSovitsV2ppVoiceCloneParams,
    requiresReferenceText: () => true
  },
  gpt_sovits_cpufast: {
    normalizeParams: normalizeGptSovitsV2ppVoiceCloneParams,
    requiresReferenceText: () => true
  }
};

export const getVoiceCloneModelRegistryEntry = (baseModel: string): VoiceCloneModelRegistryEntry =>
  VOICE_CLONE_MODEL_REGISTRY[baseModel] ?? defaultEntry;

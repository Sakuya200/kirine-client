interface VoiceCloneModelRegistryEntry {
  normalizeParams: (modelParams: Record<string, unknown>) => Record<string, unknown>;
  requiresReferenceText: (modelParams: Record<string, unknown>) => boolean;
}

const VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS = {
  mode: 'reference',
  stylePrompt: '',
  cfgValue: '2.0',
  inferenceTimesteps: 10
} as const;

const MOSS_TTS_LOCAL_VOICE_CLONE_DEFAULT_PARAMS = {
  nVqForInference: 32
} as const;

const normalizeQwen3VoiceCloneParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...modelParams
});

const normalizeVoxCpm2VoiceCloneParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS,
  ...modelParams,
  mode: String(modelParams.mode ?? VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS.mode),
  stylePrompt: String(modelParams.stylePrompt ?? VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS.stylePrompt),
  cfgValue: String(modelParams.cfgValue ?? VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS.cfgValue).trim() || VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS.cfgValue,
  inferenceTimesteps: Number(modelParams.inferenceTimesteps ?? VOX_CPM2_VOICE_CLONE_DEFAULT_PARAMS.inferenceTimesteps)
});

const normalizeMossVoiceCloneParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...MOSS_TTS_LOCAL_VOICE_CLONE_DEFAULT_PARAMS,
  ...modelParams,
  nVqForInference: Number(modelParams.nVqForInference ?? MOSS_TTS_LOCAL_VOICE_CLONE_DEFAULT_PARAMS.nVqForInference)
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
  }
};

export const getVoiceCloneModelRegistryEntry = (baseModel: string): VoiceCloneModelRegistryEntry =>
  VOICE_CLONE_MODEL_REGISTRY[baseModel] ?? defaultEntry;

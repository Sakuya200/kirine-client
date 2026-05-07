interface TextToSpeechModelRegistryEntry {
  normalizeParams: (modelParams: Record<string, unknown>) => Record<string, unknown>;
}

const QWEN3_TTS_DEFAULT_PARAMS = {
  voicePrompt: ''
} as const;

const VOX_CPM2_TTS_DEFAULT_PARAMS = {
  cfgValue: '2.0',
  inferenceTimesteps: 10
} as const;

const MOSS_TTS_LOCAL_TTS_DEFAULT_PARAMS = {
  nVqForInference: 32
} as const;

const normalizeQwen3TtsModelParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...QWEN3_TTS_DEFAULT_PARAMS,
  ...modelParams
});

const normalizeVoxCpm2TtsModelParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...VOX_CPM2_TTS_DEFAULT_PARAMS,
  ...modelParams,
  cfgValue: String(modelParams.cfgValue ?? VOX_CPM2_TTS_DEFAULT_PARAMS.cfgValue).trim() || VOX_CPM2_TTS_DEFAULT_PARAMS.cfgValue,
  inferenceTimesteps: Number(modelParams.inferenceTimesteps ?? VOX_CPM2_TTS_DEFAULT_PARAMS.inferenceTimesteps)
});

const normalizeMossTtsLocalTtsModelParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...MOSS_TTS_LOCAL_TTS_DEFAULT_PARAMS,
  ...modelParams,
  nVqForInference: Number(modelParams.nVqForInference ?? MOSS_TTS_LOCAL_TTS_DEFAULT_PARAMS.nVqForInference)
});

const defaultEntry: TextToSpeechModelRegistryEntry = {
  normalizeParams: normalizeQwen3TtsModelParams
};

const TEXT_TO_SPEECH_MODEL_REGISTRY: Record<string, TextToSpeechModelRegistryEntry> = {
  qwen3_tts: defaultEntry,
  vox_cpm2: {
    normalizeParams: normalizeVoxCpm2TtsModelParams
  },
  moss_tts_local: {
    normalizeParams: normalizeMossTtsLocalTtsModelParams
  }
};

export const getTextToSpeechModelRegistryEntry = (baseModel: string): TextToSpeechModelRegistryEntry =>
  TEXT_TO_SPEECH_MODEL_REGISTRY[baseModel] ?? defaultEntry;

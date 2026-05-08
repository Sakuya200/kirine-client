interface TextToSpeechModelRegistryEntry {
  normalizeParams: (modelParams: Record<string, unknown>) => Record<string, unknown>;
}

const normalizeQwen3TtsModelParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...modelParams,
  voicePrompt: String(modelParams.voicePrompt ?? '')
});

const normalizeVoxCpm2TtsModelParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...modelParams,
  cfgValue: String(modelParams.cfgValue ?? '').trim(),
  inferenceTimesteps: Number(modelParams.inferenceTimesteps ?? 0)
});

const normalizeMossTtsLocalTtsModelParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
  ...modelParams,
  nVqForInference: Number(modelParams.nVqForInference ?? 0)
});

const normalizeGptSovitsV2ppTtsModelParams = (modelParams: Record<string, unknown>): Record<string, unknown> => ({
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
  },
  gpt_sovits_v2pp: {
    normalizeParams: normalizeGptSovitsV2ppTtsModelParams
  },
  gpt_sovits_cpufast: {
    normalizeParams: normalizeGptSovitsV2ppTtsModelParams
  }
};

export const getTextToSpeechModelRegistryEntry = (baseModel: string): TextToSpeechModelRegistryEntry =>
  TEXT_TO_SPEECH_MODEL_REGISTRY[baseModel] ?? defaultEntry;

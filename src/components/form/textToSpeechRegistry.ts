import type { Component } from 'vue';

import MossTtsLocalTextToSpeechParamsForm from '@/components/moss_tts_local/MossTtsLocalTextToSpeechParamsForm.vue';
import {
  buildMossTtsLocalTtsGenerationSummaryLines,
  buildMossTtsLocalTtsResultSummaryLines,
  createMossTtsLocalTtsModelParams,
  normalizeMossTtsLocalTtsModelParams
} from '@/components/moss_tts_local/textToSpeechParams';
import Qwen3TtsTextToSpeechParamsForm from '@/components/qwen3_tts/Qwen3TtsTextToSpeechParamsForm.vue';
import {
  buildQwen3TtsGenerationSummaryLines,
  buildQwen3TtsResultSummaryLines,
  createQwen3TtsModelParams,
  normalizeQwen3TtsModelParams
} from '@/components/qwen3_tts/textToSpeechParams';
import VoxCpm2TextToSpeechParamsForm from '@/components/vox_cpm2/VoxCpm2TextToSpeechParamsForm.vue';
import {
  buildVoxCpm2TtsGenerationSummaryLines,
  buildVoxCpm2TtsResultSummaryLines,
  createVoxCpm2TtsModelParams,
  normalizeVoxCpm2TtsModelParams
} from '@/components/vox_cpm2/textToSpeechParams';

export interface TextToSpeechModelRegistryEntry {
  createDefaultParams: () => Record<string, unknown>;
  normalizeParams: (modelParams: Record<string, unknown>) => Record<string, unknown>;
  paramsComponent: Component;
  buildGenerationSummaryLines: (modelParams: Record<string, unknown>) => string[];
  buildResultSummaryLines: (modelParams: Record<string, unknown>) => string[];
}

const defaultEntry: TextToSpeechModelRegistryEntry = {
  createDefaultParams: createQwen3TtsModelParams,
  normalizeParams: normalizeQwen3TtsModelParams,
  paramsComponent: Qwen3TtsTextToSpeechParamsForm,
  buildGenerationSummaryLines: buildQwen3TtsGenerationSummaryLines,
  buildResultSummaryLines: buildQwen3TtsResultSummaryLines
};

export const TEXT_TO_SPEECH_MODEL_REGISTRY: Record<string, TextToSpeechModelRegistryEntry> = {
  qwen3_tts: defaultEntry,
  vox_cpm2: {
    createDefaultParams: createVoxCpm2TtsModelParams,
    normalizeParams: normalizeVoxCpm2TtsModelParams,
    paramsComponent: VoxCpm2TextToSpeechParamsForm,
    buildGenerationSummaryLines: buildVoxCpm2TtsGenerationSummaryLines,
    buildResultSummaryLines: buildVoxCpm2TtsResultSummaryLines
  },
  moss_tts_local: {
    createDefaultParams: createMossTtsLocalTtsModelParams,
    normalizeParams: normalizeMossTtsLocalTtsModelParams,
    paramsComponent: MossTtsLocalTextToSpeechParamsForm,
    buildGenerationSummaryLines: buildMossTtsLocalTtsGenerationSummaryLines,
    buildResultSummaryLines: buildMossTtsLocalTtsResultSummaryLines
  }
};

export const getTextToSpeechModelRegistryEntry = (baseModel: string): TextToSpeechModelRegistryEntry =>
  TEXT_TO_SPEECH_MODEL_REGISTRY[baseModel] ?? defaultEntry;

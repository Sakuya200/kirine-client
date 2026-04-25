import type { Component } from 'vue';

import MossTtsLocalVoiceCloneParamsForm from '@/components/moss_tts_local/MossTtsLocalVoiceCloneParamsForm.vue';
import {
  buildMossVoiceCloneResultSummaryLines,
  buildMossVoiceCloneSummaryLines,
  createMossVoiceCloneParams,
  normalizeMossVoiceCloneParams
} from '@/components/moss_tts_local/voiceCloneParams';
import Qwen3TtsVoiceCloneParamsForm from '@/components/qwen3_tts/Qwen3TtsVoiceCloneParamsForm.vue';
import {
  buildQwen3VoiceCloneResultSummaryLines,
  buildQwen3VoiceCloneSummaryLines,
  createQwen3VoiceCloneParams,
  normalizeQwen3VoiceCloneParams
} from '@/components/qwen3_tts/voiceCloneParams';
import VoxCpm2VoiceCloneParamsForm from '@/components/vox_cpm2/VoxCpm2VoiceCloneParamsForm.vue';
import {
  buildVoxCpm2VoiceCloneResultSummaryLines,
  buildVoxCpm2VoiceCloneSummaryLines,
  createVoxCpm2VoiceCloneParams,
  normalizeVoxCpm2VoiceCloneParams
} from '@/components/vox_cpm2/voiceCloneParams';

export interface VoiceCloneModelRegistryEntry {
  createDefaultParams: () => Record<string, unknown>;
  normalizeParams: (modelParams: Record<string, unknown>) => Record<string, unknown>;
  paramsComponent: Component;
  requiresReferenceText: (modelParams: Record<string, unknown>) => boolean;
  buildModeSummary: (modelParams: Record<string, unknown>) => string;
  buildCloneSummaryLines: (modelParams: Record<string, unknown>) => string[];
  buildResultSummaryLines: (modelParams: Record<string, unknown>) => string[];
}

const defaultEntry: VoiceCloneModelRegistryEntry = {
  createDefaultParams: createQwen3VoiceCloneParams,
  normalizeParams: normalizeQwen3VoiceCloneParams,
  paramsComponent: Qwen3TtsVoiceCloneParamsForm,
  requiresReferenceText: () => true,
  buildModeSummary: () => '当前克隆模式为参考音频 + 参考台词克隆。',
  buildCloneSummaryLines: buildQwen3VoiceCloneSummaryLines,
  buildResultSummaryLines: buildQwen3VoiceCloneResultSummaryLines
};

export const VOICE_CLONE_MODEL_REGISTRY: Record<string, VoiceCloneModelRegistryEntry> = {
  qwen3_tts: defaultEntry,
  vox_cpm2: {
    createDefaultParams: createVoxCpm2VoiceCloneParams,
    normalizeParams: normalizeVoxCpm2VoiceCloneParams,
    paramsComponent: VoxCpm2VoiceCloneParamsForm,
    requiresReferenceText: modelParams => String(normalizeVoxCpm2VoiceCloneParams(modelParams).mode) === 'ultimate',
    buildModeSummary: modelParams =>
      String(normalizeVoxCpm2VoiceCloneParams(modelParams).mode) === 'ultimate' ? '当前克隆模式为 Ultimate 克隆。' : '当前克隆模式为参考音频克隆。',
    buildCloneSummaryLines: buildVoxCpm2VoiceCloneSummaryLines,
    buildResultSummaryLines: buildVoxCpm2VoiceCloneResultSummaryLines
  },
  moss_tts_local: {
    createDefaultParams: createMossVoiceCloneParams,
    normalizeParams: normalizeMossVoiceCloneParams,
    paramsComponent: MossTtsLocalVoiceCloneParamsForm,
    requiresReferenceText: () => false,
    buildModeSummary: () => '当前克隆模式为参考音频条件克隆，参考台词可选。',
    buildCloneSummaryLines: buildMossVoiceCloneSummaryLines,
    buildResultSummaryLines: buildMossVoiceCloneResultSummaryLines
  }
};

export const getVoiceCloneModelRegistryEntry = (baseModel: string): VoiceCloneModelRegistryEntry =>
  VOICE_CLONE_MODEL_REGISTRY[baseModel] ?? defaultEntry;

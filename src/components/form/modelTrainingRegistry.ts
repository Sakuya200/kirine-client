import type { Component } from 'vue';

import MossTtsLocalTrainingParamsForm from '@/components/moss_tts_local/MossTtsLocalTrainingParamsForm.vue';
import {
  buildMossTtsLocalTrainingSummaryLines,
  createMossTtsLocalTrainingParams,
  normalizeMossTtsLocalTrainingParams
} from '@/components/moss_tts_local/trainingParams';
import Qwen3TtsTrainingParamsForm from '@/components/qwen3_tts/Qwen3TtsTrainingParamsForm.vue';
import { buildQwen3TrainingSummaryLines, createQwen3TrainingParams, normalizeQwen3TrainingParams } from '@/components/qwen3_tts/trainingParams';
import VoxCpm2TrainingParamsForm from '@/components/vox_cpm2/VoxCpm2TrainingParamsForm.vue';
import { buildVoxCpm2TrainingSummaryLines, createVoxCpm2TrainingParams, normalizeVoxCpm2TrainingParams } from '@/components/vox_cpm2/trainingParams';

export interface TrainingModelRegistryEntry {
  createDefaultParams: () => Record<string, unknown>;
  normalizeParams: (modelParams: Record<string, unknown>) => Record<string, unknown>;
  paramsComponent: Component;
  baseSummary: string;
  buildSummaryLines: (modelParams: Record<string, unknown>) => string[];
}

const defaultEntry: TrainingModelRegistryEntry = {
  createDefaultParams: createQwen3TrainingParams,
  normalizeParams: normalizeQwen3TrainingParams,
  paramsComponent: Qwen3TtsTrainingParamsForm,
  baseSummary: '微调任务会使用设置页中的全局硬件类型；若切换硬件，请先前往设置页保存。',
  buildSummaryLines: buildQwen3TrainingSummaryLines
};

export const MODEL_TRAINING_REGISTRY: Record<string, TrainingModelRegistryEntry> = {
  qwen3_tts: defaultEntry,
  vox_cpm2: {
    createDefaultParams: createVoxCpm2TrainingParams,
    normalizeParams: normalizeVoxCpm2TrainingParams,
    paramsComponent: VoxCpm2TrainingParamsForm,
    baseSummary: '微调任务会使用设置页中的全局硬件类型；若切换硬件，请先前往设置页保存。',
    buildSummaryLines: buildVoxCpm2TrainingSummaryLines
  },
  moss_tts_local: {
    createDefaultParams: createMossTtsLocalTrainingParams,
    normalizeParams: normalizeMossTtsLocalTrainingParams,
    paramsComponent: MossTtsLocalTrainingParamsForm,
    baseSummary: 'MOSS-TTS Local 使用单卡训练封装，不启用 FSDP 或 DeepSpeed；若切换硬件，请先前往设置页保存。',
    buildSummaryLines: buildMossTtsLocalTrainingSummaryLines
  }
};

export const getTrainingModelRegistryEntry = (baseModel: string): TrainingModelRegistryEntry => MODEL_TRAINING_REGISTRY[baseModel] ?? defaultEntry;

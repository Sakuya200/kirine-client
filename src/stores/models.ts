import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import { BaseModel } from '@/enums/settings';
import { HistoryTaskType } from '@/enums/task';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useUiStore } from '@/stores/ui';
import type { ModelInfo } from '@/types/domain';

const normalizeModelInfo = (item: Partial<ModelInfo>): ModelInfo => ({
  id: typeof item.id === 'number' ? item.id : 0,
  baseModel: item.baseModel === BaseModel.Qwen3Tts ? item.baseModel : BaseModel.Qwen3Tts,
  modelName: item.modelName?.trim() || 'Unknown Model',
  modelScaleList: Array.isArray(item.modelScaleList) ? item.modelScaleList.filter(scale => typeof scale === 'string') : [],
  requiredModelNameList: Array.isArray(item.requiredModelNameList) ? item.requiredModelNameList.filter(name => typeof name === 'string') : [],
  requiredModelRepoIdList: Array.isArray(item.requiredModelRepoIdList) ? item.requiredModelRepoIdList.filter(name => typeof name === 'string') : [],
  supportedFeatureList: Array.isArray(item.supportedFeatureList)
    ? item.supportedFeatureList.filter(
        feature => feature === HistoryTaskType.TextToSpeech || feature === HistoryTaskType.VoiceClone || feature === HistoryTaskType.ModelTraining
      )
    : [],
  createTime: item.createTime ?? '',
  modifyTime: item.modifyTime ?? ''
});

export const useModelStore = defineStore('models', () => {
  const items = ref<ModelInfo[]>([]);
  const isLoading = ref(false);
  const initialized = ref(false);
  const uiStore = useUiStore();

  const byBaseModel = computed(() => new Map(items.value.map(item => [item.baseModel, item])));

  const loadModels = async () => {
    isLoading.value = true;

    try {
      const result = await invoke<ModelInfo[]>('list_model_infos');
      items.value = Array.isArray(result) ? result.map(normalizeModelInfo) : [];
    } catch (error) {
      items.value = [];
      uiStore.notifyError(formatErrorMessage('加载模型列表失败', error));
    } finally {
      initialized.value = true;
      isLoading.value = false;
    }
  };

  const ensureLoaded = async () => {
    if (!initialized.value && !isLoading.value) {
      await loadModels();
    }
  };

  const getModelsByFeature = (feature: HistoryTaskType) => items.value.filter(item => item.supportedFeatureList.includes(feature));

  const getModelLabel = (baseModel: BaseModel) => byBaseModel.value.get(baseModel)?.modelName ?? baseModel;

  const getModelScaleOptions = (baseModel: BaseModel) =>
    (byBaseModel.value.get(baseModel)?.modelScaleList ?? []).map(scale => ({
      label: scale,
      value: scale
    }));

  return {
    items,
    isLoading,
    initialized,
    byBaseModel,
    loadModels,
    ensureLoaded,
    getModelsByFeature,
    getModelLabel,
    getModelScaleOptions
  };
});

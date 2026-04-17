import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import { HistoryTaskType } from '@/enums/task';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useUiStore } from '@/stores/ui';
import type { BaseModel, ModelInfo } from '@/types/domain';

const normalizeModelInfo = (item: Partial<ModelInfo>): ModelInfo => ({
  id: typeof item.id === 'number' ? item.id : 0,
  baseModel: typeof item.baseModel === 'string' ? item.baseModel.trim() : '',
  modelName: item.modelName?.trim() || 'Unknown Model',
  modelScale: typeof item.modelScale === 'string' ? item.modelScale.trim() : '',
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

  const byBaseModel = computed(() => {
    const grouped = new Map<BaseModel, ModelInfo[]>();

    for (const item of items.value) {
      if (!item.baseModel) {
        continue;
      }

      const next = grouped.get(item.baseModel) ?? [];
      next.push(item);
      grouped.set(item.baseModel, next);
    }

    return grouped;
  });

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

  const getModelsByFeature = (feature: HistoryTaskType) =>
    Array.from(byBaseModel.value.values())
      .map(variants => variants.find(item => item.supportedFeatureList.includes(feature)))
      .filter((item): item is ModelInfo => Boolean(item));

  const getModelLabel = (baseModel: BaseModel) => byBaseModel.value.get(baseModel)?.[0]?.modelName ?? baseModel;

  const getModelScaleOptions = (baseModel: BaseModel) =>
    (byBaseModel.value.get(baseModel) ?? []).map(item => ({
      label: item.modelScale,
      value: item.modelScale
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

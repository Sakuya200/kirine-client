import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import { HardwareType } from '@/enums/settings';
import { HistoryTaskType } from '@/enums/task';
import { AppLanguage } from '@/enums/language';
import { ModelInstallStatus } from '@/enums/status';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useUiStore } from '@/stores/ui';
import type { BaseModel, ModelInfo, ModelMutationResult, Page } from '@/types/domain';

const normalizeModelInfo = (item: Partial<ModelInfo>): ModelInfo => ({
  id: typeof item.id === 'number' ? item.id : 0,
  baseModel: typeof item.baseModel === 'string' ? item.baseModel.trim() : '',
  modelName: item.modelName?.trim() || 'Unknown Model',
  modelVersion: typeof item.modelVersion === 'string' ? item.modelVersion.trim() : '',
  requiredModelNameList: Array.isArray(item.requiredModelNameList) ? item.requiredModelNameList.filter(name => typeof name === 'string') : [],
  requiredModelRepoIdList: Array.isArray(item.requiredModelRepoIdList) ? item.requiredModelRepoIdList.filter(name => typeof name === 'string') : [],
  supportedFeatureList: Array.isArray(item.supportedFeatureList)
    ? item.supportedFeatureList
        .filter((feature): feature is string => typeof feature === 'string')
        .map(feature => feature.trim())
        .filter(Boolean)
    : [],
  supportedDevices: Array.isArray(item.supportedDevices)
    ? item.supportedDevices.map(device => (device === HardwareType.Cuda ? HardwareType.Cuda : HardwareType.Cpu))
    : [],
  supportedLanguages: Array.isArray(item.supportedLanguages)
    ? item.supportedLanguages.filter((lang): lang is AppLanguage => Object.values(AppLanguage).includes(lang as AppLanguage))
    : [],
  downloaded: item.downloaded === true,
  createTime: item.createTime ?? '',
  modifyTime: item.modifyTime ?? ''
});

export const useModelStore = defineStore('models', () => {
  const items = ref<ModelInfo[]>([]);
  const isLoading = ref(false);
  const initialized = ref(false);
  // 安装失败的模型 id（Rust 端 install_model 报错时记录）。这是会话级前端状态：
  // 后端 downloaded 仅反映权重是否就绪，无法表达“最近一次安装失败”，故在此单独追踪，
  // 供模型管理页展示“安装失败”状态。安装/重装成功或卸载成功后清除。
  const failedModelIds = ref<Set<number>>(new Set());
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
      const result = await invoke<Page<ModelInfo>>('list_model_infos', {
        request: { page: 1, pageSize: 500, filter: null }
      });
      const rawItems = Array.isArray(result?.items) ? result.items : [];
      items.value = rawItems.map(normalizeModelInfo);
    } catch (error) {
      items.value = [];
      uiStore.notifyError(formatErrorMessage('加载模型列表失败', error));
    } finally {
      initialized.value = true;
      isLoading.value = false;
    }
  };

  const replaceModel = (nextModel: ModelInfo) => {
    items.value = items.value.map(item => (item.id === nextModel.id ? nextModel : item));
  };

  const installStatusOf = (item: ModelInfo): ModelInstallStatus => {
    if (failedModelIds.value.has(item.id)) {
      return ModelInstallStatus.Failed;
    }
    return item.downloaded ? ModelInstallStatus.Installed : ModelInstallStatus.NotInstalled;
  };

  const clearInstallFailed = (modelId: number) => {
    failedModelIds.value.delete(modelId);
  };

  const installModel = async (modelId: number, device: HardwareType = HardwareType.Cpu) => {
    try {
      const result = await invoke<ModelMutationResult>('install_model', { modelId, device });
      const normalized = {
        ...result,
        model: normalizeModelInfo(result.model)
      };
      replaceModel(normalized.model);
      failedModelIds.value.delete(modelId);
      uiStore.notifySuccess(`模型 ${normalized.model.modelName} ${normalized.model.modelVersion} 已安装。`, 3200);
      return normalized;
    } catch (error) {
      failedModelIds.value.add(modelId);
      uiStore.notifyError(formatErrorMessage('安装模型失败', error));
      return null;
    }
  };

  const uninstallModel = async (modelId: number) => {
    try {
      const result = await invoke<ModelMutationResult>('uninstall_model', { modelId });
      const normalized = {
        ...result,
        model: normalizeModelInfo(result.model)
      };
      replaceModel(normalized.model);
      failedModelIds.value.delete(modelId);
      uiStore.notifySuccess(`模型 ${normalized.model.modelName} ${normalized.model.modelVersion} 已卸载。`, 3200);
      return normalized;
    } catch (error) {
      uiStore.notifyError(formatErrorMessage('卸载模型失败', error));
      return null;
    }
  };

  const reinstallModel = async (modelId: number, device: HardwareType = HardwareType.Cpu) => {
    const uninstalled = await uninstallModel(modelId);
    if (!uninstalled) {
      uiStore.notifyWarning('模型卸载失败，无法继续重装，请重试卸载。', 4200);
      return null;
    }

    const installed = await installModel(modelId, device);
    if (!installed) {
      uiStore.notifyWarning('模型已卸载，但重装失败，请重试安装。', 4200);
      return null;
    }

    return installed;
  };

  const getDeviceType = async (baseModel: BaseModel, modelVersion: string) => {
    const result = await invoke<HardwareType>('get_device_type', { baseModel, modelVersion });
    return result === HardwareType.Cuda ? HardwareType.Cuda : HardwareType.Cpu;
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

  const supportsModelFeature = (baseModel: BaseModel, modelVersion: string, feature: string) =>
    (byBaseModel.value.get(baseModel) ?? []).some(item => item.modelVersion === modelVersion && item.supportedFeatureList.includes(feature));

  const getModelLabel = (baseModel: BaseModel) => byBaseModel.value.get(baseModel)?.[0]?.modelName ?? baseModel;

  const getModelVersionOptions = (baseModel: BaseModel) =>
    (byBaseModel.value.get(baseModel) ?? []).map(item => ({
      label: item.modelVersion,
      value: item.modelVersion
    }));

  const getSupportedDevices = (baseModel: BaseModel, modelVersion: string) =>
    (byBaseModel.value.get(baseModel) ?? []).find(item => item.modelVersion === modelVersion)?.supportedDevices ?? [];

  // 模型声明的支持语言；若模型未声明则回退到全部语言，避免下拉为空
  const getSupportedLanguages = (baseModel: BaseModel, modelVersion: string): AppLanguage[] => {
    const declared = (byBaseModel.value.get(baseModel) ?? []).find(item => item.modelVersion === modelVersion)?.supportedLanguages ?? [];
    return declared.length > 0 ? declared : Object.values(AppLanguage);
  };

  return {
    items,
    isLoading,
    initialized,
    failedModelIds,
    byBaseModel,
    loadModels,
    ensureLoaded,
    installModel,
    reinstallModel,
    getDeviceType,
    getModelsByFeature,
    getModelLabel,
    getModelVersionOptions,
    getSupportedDevices,
    getSupportedLanguages,
    uninstallModel,
    supportsModelFeature,
    installStatusOf,
    clearInstallFailed
  };
});

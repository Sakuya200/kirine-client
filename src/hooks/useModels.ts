import { invoke } from '@tauri-apps/api/core';
import { computed, ref } from 'vue';
import { i18n } from '@/locales';

import { HardwareType } from '@/enums/settings';
import { HistoryTaskType } from '@/enums/task';
import { AppLanguage } from '@/enums/language';
import { ModelInstallStatus } from '@/enums/status';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useModelStore } from '@/stores/models';
import { useUiStore } from '@/stores/ui';
import type { BaseModel, ModelInfo, Page } from '@/types/domain';

export const normalizeModelInfo = (item: Partial<ModelInfo>): ModelInfo => {
  const supportedDevices = Array.isArray(item.supportedDevices)
    ? item.supportedDevices.map(device => (device === HardwareType.Cuda ? HardwareType.Cuda : HardwareType.Cpu))
    : [];
  // currentDevice 仅当为合法 HardwareType 且属于该模型支持设备时保留；缺省/非法/失效统一为 null，
  // 对应「多设备要求先选」；单设备模型的回填由后端 sync 完成。
  const currentDevice =
    item.currentDevice === HardwareType.Cpu || item.currentDevice === HardwareType.Cuda
      ? supportedDevices.includes(item.currentDevice)
        ? item.currentDevice
        : null
      : null;

  return {
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
    supportedDevices,
    currentDevice,
    supportedLanguages: Array.isArray(item.supportedLanguages)
      ? item.supportedLanguages.filter((lang): lang is AppLanguage => Object.values(AppLanguage).includes(lang as AppLanguage))
      : [],
    downloaded: item.downloaded === true,
    createTime: item.createTime ?? '',
    modifyTime: item.modifyTime ?? ''
  };
};

export interface UseModelsOptions {
  /** 挂载后立即拉取模型列表（默认 true）；传 false 时仅靠查询函数的惰性加载 */
  immediate?: boolean;
}

/**
 * 模型列表直连取数（页面每次挂载拉取，不做跨页缓存）。
 * 查询函数在数据未就绪时会自动触发一次静默加载，保证回显场景（如历史详情标签）
 * 无需手动调用 load。
 */
export const useModels = (options: UseModelsOptions = {}) => {
  const { immediate = true } = options;

  const items = ref<ModelInfo[]>([]);
  const isLoading = ref(false);
  const uiStore = useUiStore();
  const modelStore = useModelStore();
  let hasRequested = false;

  const load = async ({ silent = false }: { silent?: boolean } = {}) => {
    hasRequested = true;
    isLoading.value = true;

    try {
      const result = await invoke<Page<ModelInfo>>('list_model_infos', {
        request: { page: 1, pageSize: 500, filter: null }
      });
      const rawItems = Array.isArray(result?.items) ? result.items : [];
      items.value = rawItems.map(normalizeModelInfo);
    } catch (error) {
      items.value = [];
      if (!silent) {
        uiStore.notifyError(formatErrorMessage(i18n.global.t('common.store.models.loadFailed'), error));
      }
    } finally {
      isLoading.value = false;
    }
  };

  // 惰性加载：查询函数被调用时若尚未拉取过数据，自动静默加载一次
  const ensureLoaded = () => {
    if (!hasRequested && !isLoading.value) {
      void load({ silent: true });
    }
  };

  if (immediate) {
    ensureLoaded();
  }

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

  const getModelsByFeature = (feature: HistoryTaskType) => {
    ensureLoaded();
    return Array.from(byBaseModel.value.values())
      .map(variants => variants.find(item => item.supportedFeatureList.includes(feature)))
      .filter((item): item is ModelInfo => Boolean(item));
  };

  const supportsModelFeature = (baseModel: BaseModel, modelVersion: string, feature: string) => {
    ensureLoaded();
    return (byBaseModel.value.get(baseModel) ?? []).some(
      item => item.modelVersion === modelVersion && item.supportedFeatureList.includes(feature)
    );
  };

  const getModelLabel = (baseModel: BaseModel) => {
    ensureLoaded();
    return byBaseModel.value.get(baseModel)?.[0]?.modelName ?? baseModel;
  };

  const getModelVersionOptions = (baseModel: BaseModel) => {
    ensureLoaded();
    return (byBaseModel.value.get(baseModel) ?? []).map(item => ({
      label: item.modelVersion,
      value: item.modelVersion
    }));
  };

  const getSupportedDevices = (baseModel: BaseModel, modelVersion: string) => {
    ensureLoaded();
    return (byBaseModel.value.get(baseModel) ?? []).find(item => item.modelVersion === modelVersion)?.supportedDevices ?? [];
  };

  // 模型声明的支持语言；若模型未声明则回退到全部语言，避免下拉为空
  const getSupportedLanguages = (baseModel: BaseModel, modelVersion: string): AppLanguage[] => {
    ensureLoaded();
    const declared =
      (byBaseModel.value.get(baseModel) ?? []).find(item => item.modelVersion === modelVersion)?.supportedLanguages ?? [];
    return declared.length > 0 ? declared : Object.values(AppLanguage);
  };

  const installStatusOf = (item: ModelInfo): ModelInstallStatus => {
    if (modelStore.failedModelIds.has(item.id)) {
      return ModelInstallStatus.Failed;
    }
    return item.downloaded ? ModelInstallStatus.Installed : ModelInstallStatus.NotInstalled;
  };

  return {
    items,
    isLoading,
    load,
    byBaseModel,
    getModelsByFeature,
    supportsModelFeature,
    getModelLabel,
    getModelVersionOptions,
    getSupportedDevices,
    getSupportedLanguages,
    installStatusOf
  };
};

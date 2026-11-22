import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { ref } from 'vue';
import { i18n } from '@/locales';

import { HardwareType } from '@/enums/settings';
import { ModelInstallStatus } from '@/enums/status';
import { normalizeModelInfo } from '@/hooks/useModels';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useUiStore } from '@/stores/ui';
import type { BaseModel, ModelInfo, ModelMutationResult, Page } from '@/types/domain';

/** 模型列表跨页缓存 TTL：窗口内切页/查询共享同一份数据，不重复发 list_model_infos。 */
const MODEL_CACHE_TTL_MS = 60_000;

export const useModelStore = defineStore('models', () => {
  // 安装失败的模型 id（Rust 端 install_model 报错时记录）。这是会话级前端状态：
  // 后端 downloaded 仅反映权重是否就绪，无法表达“最近一次安装失败”，故在此单独追踪，
  // 供模型管理页展示“安装失败”状态。安装/重装成功或卸载成功后清除。
  const failedModelIds = ref<Set<number>>(new Set());
  const uiStore = useUiStore();

  // —— 模型列表跨页缓存 ——
  // 每个视图/表单挂载都经 useModels 拉一次 pageSize:500 的全量列表；TTL 缓存 +
  // 在途去重把同窗口内的 N 次切页请求合并为 1 次。安装/卸载/换设备成功后失效，
  // 保证 mutation 之后的刷新仍走真实请求。
  const cachedModels = ref<ModelInfo[] | null>(null);
  const cachedModelsAt = ref(0);
  let modelsRequest: Promise<ModelInfo[]> | null = null;

  const invalidateModelCache = () => {
    cachedModels.value = null;
    cachedModelsAt.value = 0;
  };

  const getModelCache = async ({ force = false } = {}): Promise<ModelInfo[]> => {
    if (!force && cachedModels.value && Date.now() - cachedModelsAt.value < MODEL_CACHE_TTL_MS) {
      return cachedModels.value;
    }
    if (!modelsRequest) {
      modelsRequest = invoke<Page<ModelInfo>>('list_model_infos', { request: { page: 1, pageSize: 500, filter: null } })
        .then(result => {
          const rawItems = Array.isArray(result?.items) ? result.items : [];
          return rawItems.map(normalizeModelInfo);
        })
        .finally(() => {
          modelsRequest = null;
        });
    }
    const items = await modelsRequest;
    cachedModels.value = items;
    cachedModelsAt.value = Date.now();
    return items;
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

  const installModel = async (modelId: number, device: HardwareType) => {
    try {
      const result = await invoke<ModelMutationResult>('install_model', { modelId, device });
      const normalized = {
        ...result,
        model: normalizeModelInfo(result.model)
      };
      failedModelIds.value.delete(modelId);
      invalidateModelCache();
      uiStore.notifySuccess(i18n.global.t('common.store.models.installed', { name: normalized.model.modelName, version: normalized.model.modelVersion }), 3200);
      return normalized;
    } catch (error) {
      failedModelIds.value.add(modelId);
      uiStore.notifyError(formatErrorMessage(i18n.global.t('common.store.models.installFailed'), error));
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
      failedModelIds.value.delete(modelId);
      invalidateModelCache();
      uiStore.notifySuccess(i18n.global.t('common.store.models.uninstalled', { name: normalized.model.modelName, version: normalized.model.modelVersion }), 3200);
      return normalized;
    } catch (error) {
      uiStore.notifyError(formatErrorMessage(i18n.global.t('common.store.models.uninstallFailed'), error));
      return null;
    }
  };

  const reinstallModel = async (modelId: number, device: HardwareType) => {
    const uninstalled = await uninstallModel(modelId);
    if (!uninstalled) {
      uiStore.notifyWarning(i18n.global.t('common.store.models.reinstallAfterUninstallFailed'), 4200);
      return null;
    }

    const installed = await installModel(modelId, device);
    if (!installed) {
      uiStore.notifyWarning(i18n.global.t('common.store.models.reinstallAfterUninstall'), 4200);
      return null;
    }

    return installed;
  };

  const getDeviceType = async (baseModel: BaseModel, modelVersion: string) => {
    const result = await invoke<HardwareType>('get_device_type', { baseModel, modelVersion });
    return result === HardwareType.Cuda ? HardwareType.Cuda : HardwareType.Cpu;
  };

  // 持久化用户在模型管理页选择的当前设备，跨会话保留。
  // 仅允许写入该模型 supportedDevices 内的设备（后端会再次校验）。
  // 注意：调用方负责在成功后重新拉取模型列表（本 store 不再缓存列表）。
  const setCurrentDevice = async (modelId: number, device: HardwareType): Promise<ModelInfo | null> => {
    try {
      const next = await invoke<ModelInfo>('set_model_current_device', { modelId, device });
      invalidateModelCache();
      return normalizeModelInfo(next);
    } catch (error) {
      uiStore.notifyError(formatErrorMessage(i18n.global.t('common.store.models.setCurrentDeviceFailed'), error));
      return null;
    }
  };

  return {
    failedModelIds,
    installModel,
    reinstallModel,
    setCurrentDevice,
    getDeviceType,
    uninstallModel,
    installStatusOf,
    clearInstallFailed,
    getModelCache,
    invalidateModelCache
  };
});

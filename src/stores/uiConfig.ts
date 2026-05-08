import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useUiStore } from '@/stores/ui';
import type { UiConfigCatalog, TaskParamConfig, ComponentProps, ParamDefinition, SelectOption, UiTaskKind, VisibleWhenRule } from '@/types/uiConfig';

const EMPTY_CATALOG: UiConfigCatalog = {
  taskConfigs: []
};

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value);

const normalizeVisibleWhen = (value: unknown): VisibleWhenRule | undefined => {
  if (!isRecord(value) || typeof value.field !== 'string') {
    return undefined;
  }

  return {
    field: value.field,
    equals: value.equals
  };
};

const normalizeSelectOption = (value: unknown): SelectOption | null => {
  if (!isRecord(value) || typeof value.label !== 'string') {
    return null;
  }

  return {
    label: value.label,
    value: value.value
  };
};

const normalizeComponentProps = (value: unknown): ComponentProps => {
  if (!isRecord(value)) {
    return {
      options: [],
      extra: {}
    };
  }

  const { label, text, textOn, textOff, rows, placeholder, helpText, min, max, step, nullable, inputMode, visibleWhen, options, ...extra } = value;

  return {
    label: typeof label === 'string' ? label : undefined,
    text: typeof text === 'string' ? text : undefined,
    textOn: typeof textOn === 'string' ? textOn : undefined,
    textOff: typeof textOff === 'string' ? textOff : undefined,
    rows: typeof rows === 'number' ? rows : undefined,
    placeholder: typeof placeholder === 'string' ? placeholder : undefined,
    helpText: typeof helpText === 'string' ? helpText : undefined,
    min,
    max,
    step,
    nullable: typeof nullable === 'boolean' ? nullable : undefined,
    inputMode: typeof inputMode === 'string' ? inputMode : undefined,
    visibleWhen: normalizeVisibleWhen(visibleWhen),
    options: Array.isArray(options) ? options.map(normalizeSelectOption).filter((item): item is SelectOption => item !== null) : [],
    extra
  };
};

const normalizeParamDefinition = (value: unknown): ParamDefinition | null => {
  if (
    !isRecord(value) ||
    typeof value.name !== 'string' ||
    typeof (value.paramType ?? value.type) !== 'string' ||
    typeof value.componentType !== 'string'
  ) {
    return null;
  }

  return {
    name: value.name,
    paramType: (value.paramType ?? value.type) as ParamDefinition['paramType'],
    componentType: value.componentType as ParamDefinition['componentType'],
    componentProps: normalizeComponentProps(value.componentProps),
    required: value.required === true,
    defaultValue: value.defaultValue,
    description: typeof value.description === 'string' ? value.description : ''
  };
};

const normalizeTaskParamConfig = (value: unknown): TaskParamConfig | null => {
  if (!isRecord(value) || typeof value.task !== 'string' || typeof (value.baseModel ?? value['base-model']) !== 'string') {
    return null;
  }

  return {
    task: value.task as UiTaskKind,
    baseModel: String(value.baseModel ?? value['base-model']).trim(),
    params: Array.isArray(value.params) ? value.params.map(normalizeParamDefinition).filter((item): item is ParamDefinition => item !== null) : []
  };
};

const normalizeUiConfigCatalog = (value: unknown): UiConfigCatalog => {
  if (!isRecord(value)) {
    return EMPTY_CATALOG;
  }

  const rawTaskConfigs = Array.isArray(value.taskConfigs) ? value.taskConfigs : Array.isArray(value.task_configs) ? value.task_configs : [];

  return {
    taskConfigs: rawTaskConfigs.map(normalizeTaskParamConfig).filter((item): item is TaskParamConfig => item !== null)
  };
};

export const useUiConfigStore = defineStore('ui-config', () => {
  const catalog = ref<UiConfigCatalog>(EMPTY_CATALOG);
  const initialized = ref(false);
  const isLoading = ref(false);
  const uiStore = useUiStore();

  const taskConfigMap = computed(() => {
    const next = new Map<string, TaskParamConfig>();

    for (const item of catalog.value.taskConfigs) {
      if (!item.baseModel) {
        continue;
      }

      next.set(`${item.baseModel}:${item.task}`, item);
    }

    return next;
  });

  const loadCatalog = async () => {
    isLoading.value = true;

    try {
      const payload = await invoke<UiConfigCatalog>('get_ui_config');
      catalog.value = normalizeUiConfigCatalog(payload);
    } catch (error) {
      catalog.value = EMPTY_CATALOG;
      uiStore.notifyError(formatErrorMessage('加载界面参数配置失败', error));
    } finally {
      initialized.value = true;
      isLoading.value = false;
    }
  };

  const ensureLoaded = async () => {
    if (!initialized.value && !isLoading.value) {
      await loadCatalog();
    }
  };

  const getTaskConfig = (baseModel: string, task: UiTaskKind) => taskConfigMap.value.get(`${baseModel}:${task}`) ?? null;

  const validateModelParams = (baseModel: string, task: UiTaskKind, modelParams: Record<string, unknown>): boolean => {
    const taskConfig = getTaskConfig(baseModel, task);
    if (!taskConfig) {
      return false;
    }
    const paramDefinitionsMap = new Map<string, ParamDefinition>(taskConfig.params.map(param => [param.name, param]));
    for (const [key, value] of Object.entries(modelParams)) {
      const paramDefinition = paramDefinitionsMap.get(key);
      if (!paramDefinition || !paramDefinition.required) {
        continue;
      }

      switch (paramDefinition.paramType) {
        case 'number':
          if (typeof value !== 'number') {
            return false;
          }
          break;
        case 'string':
          if (typeof value !== 'string' || value.trim() === '') {
            return false;
          }
          break;
        case 'boolean':
          if (typeof value !== 'boolean') {
            return false;
          }
          break;
        default:
          return false;
      }
    }
    return true;
  };

  return {
    catalog,
    initialized,
    isLoading,
    loadCatalog,
    ensureLoaded,
    getTaskConfig,
    validateModelParams
  };
});

import type { TaskParamConfig } from '@/types/uiConfig';

const cloneValue = (value: unknown): unknown => {
  if (Array.isArray(value)) {
    return value.map(item => cloneValue(item));
  }

  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value as Record<string, unknown>).map(([key, item]) => [key, cloneValue(item)]));
  }

  return value;
};

export const buildDefaultModelParamsFromUiConfig = (taskConfig: TaskParamConfig | null): Record<string, unknown> => {
  if (!taskConfig) {
    return {};
  }

  return Object.fromEntries(taskConfig.params.map(param => [param.name, cloneValue(param.defaultValue)]));
};

export const mergeModelParamsWithUiConfigDefaults = (
  taskConfig: TaskParamConfig | null,
  modelParams: Record<string, unknown>
): Record<string, unknown> => ({
  ...buildDefaultModelParamsFromUiConfig(taskConfig),
  ...modelParams
});

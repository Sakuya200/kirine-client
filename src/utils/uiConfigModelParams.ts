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

const normalizeBooleanValue = (value: unknown): boolean => {
  if (typeof value === 'boolean') {
    return value;
  }

  if (typeof value === 'string') {
    const normalized = value.trim().toLowerCase();
    if (normalized === 'true') {
      return true;
    }
    if (normalized === 'false') {
      return false;
    }
  }

  return Boolean(value);
};

const normalizeValueByParamType = (value: unknown, paramType: 'number' | 'string' | 'boolean'): unknown => {
  if (value == null) {
    return value;
  }

  if (paramType === 'string') {
    return String(value);
  }

  if (paramType === 'number') {
    return typeof value === 'number' ? value : Number(value);
  }

  return normalizeBooleanValue(value);
};

const shouldUseDefaultValue = (value: unknown, paramType: 'number' | 'string' | 'boolean'): boolean => {
  if (value == null) {
    return true;
  }

  if (paramType === 'string') {
    return String(value).trim().length === 0;
  }

  if (paramType === 'number') {
    const numeric = typeof value === 'number' ? value : Number(value);
    return Number.isNaN(numeric);
  }

  return false;
};

export const normalizeModelParamsWithUiConfig = (
  taskConfig: TaskParamConfig | null,
  modelParams: Record<string, unknown>
): Record<string, unknown> => {
  if (!taskConfig) {
    return { ...modelParams };
  }

  const normalizedParams: Record<string, unknown> = { ...modelParams };

  for (const param of taskConfig.params) {
    const rawValue = modelParams[param.name];

    if (shouldUseDefaultValue(rawValue, param.paramType)) {
      normalizedParams[param.name] = cloneValue(param.defaultValue);
      continue;
    }

    normalizedParams[param.name] = normalizeValueByParamType(rawValue, param.paramType);
  }

  return normalizedParams;
};

export const mergeModelParamsWithUiConfigDefaults = (
  taskConfig: TaskParamConfig | null,
  modelParams: Record<string, unknown>
): Record<string, unknown> => ({
  ...buildDefaultModelParamsFromUiConfig(taskConfig),
  ...normalizeModelParamsWithUiConfig(taskConfig, modelParams)
});

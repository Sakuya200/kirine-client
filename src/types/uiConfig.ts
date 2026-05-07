export type UiTaskKind = 'training' | 'tts' | 'voice-clone';

export type UiParamType = 'number' | 'string' | 'boolean';

export type UiComponentType = 'input-number' | 'input-text' | 'textarea' | 'select' | 'switch';

export interface VisibleWhenRule {
  field: string;
  equals: unknown;
}

export interface SelectOption {
  label: string;
  value: unknown;
}

export interface ComponentProps {
  label?: string;
  text?: string;
  textOn?: string;
  textOff?: string;
  rows?: number;
  placeholder?: string;
  helpText?: string;
  min?: unknown;
  max?: unknown;
  step?: unknown;
  nullable?: boolean;
  inputMode?: string;
  visibleWhen?: VisibleWhenRule;
  options: SelectOption[];
  extra: Record<string, unknown>;
}

export interface ParamDefinition {
  name: string;
  paramType: UiParamType;
  componentType: UiComponentType;
  componentProps: ComponentProps;
  required: boolean;
  defaultValue: unknown;
  description: string;
}

export interface TaskParamConfig {
  task: UiTaskKind;
  baseModel: string;
  params: ParamDefinition[];
}

export interface UiConfigCatalog {
  taskConfigs: TaskParamConfig[];
}

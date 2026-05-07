<script setup lang="ts">
import { computed } from 'vue';

import UiParamInputField from '@/components/ui/UiParamInputField.vue';
import UiParamSelectField from '@/components/ui/UiParamSelectField.vue';
import UiParamSwitchField from '@/components/ui/UiParamSwitchField.vue';
import UiParamTextareaField from '@/components/ui/UiParamTextareaField.vue';
import type { ParamDefinition, SelectOption, TaskParamConfig } from '@/types/uiConfig';

interface Props {
  modelValue: Record<string, unknown>;
  taskConfig: TaskParamConfig | null;
  supportsLora?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  supportsLora: true
});

const emit = defineEmits<{
  'update:modelValue': [value: Record<string, unknown>];
}>();

const isVisibleWhenMatched = (param: ParamDefinition) => {
  const rule = param.componentProps.visibleWhen;

  if (!rule) {
    return true;
  }

  const currentValue = rule.field === 'useLora' && !props.supportsLora ? false : props.modelValue[rule.field];
  return JSON.stringify(currentValue) === JSON.stringify(rule.equals);
};

const shouldRenderParam = (param: ParamDefinition) => {
  if (param.name === 'useLora' && !props.supportsLora) {
    return false;
  }

  return isVisibleWhenMatched(param);
};

const visibleParams = computed(() => (props.taskConfig?.params ?? []).filter(param => shouldRenderParam(param)));

const updateModelValue = (name: string, value: unknown) => {
  const nextValue: Record<string, unknown> = {
    ...props.modelValue,
    [name]: value
  };

  if (name === 'useLora') {
    nextValue.trainingMode = value === true ? 'lora' : 'full';
  }

  emit('update:modelValue', nextValue);
};

const getFieldValue = (param: ParamDefinition) => {
  if (param.name === 'useLora' && !props.supportsLora) {
    return false;
  }

  return props.modelValue[param.name];
};

const toInputString = (param: ParamDefinition) => {
  const value = getFieldValue(param);
  return value == null ? '' : String(value);
};
const handleNumberInput = (param: ParamDefinition, nextRawValue: string) => {
  if (nextRawValue.trim().length === 0 && param.componentProps.nullable) {
    updateModelValue(param.name, null);
    return;
  }

  if (param.paramType === 'number') {
    updateModelValue(param.name, nextRawValue.trim().length === 0 ? 0 : Number(nextRawValue));
    return;
  }

  updateModelValue(param.name, nextRawValue);
};

const handleTextInput = (param: ParamDefinition, value: string) => {
  updateModelValue(param.name, value);
};

const handleSwitchChange = (param: ParamDefinition, checked: boolean) => {
  updateModelValue(param.name, checked);
};

const handleSelectChange = (param: ParamDefinition, value: string | number | boolean | null | undefined) => {
  updateModelValue(param.name, value);
};

const mapSelectOptions = (options: SelectOption[]) =>
  options.map(option => ({
    label: option.label,
    value: option.value as string | number | boolean | null | undefined
  }));
</script>

<template>
  <div class="space-y-4 text-sm text-slate-700">
    <div class="grid gap-3 md:grid-cols-2">
      <template v-for="param in visibleParams" :key="param.name">
        <UiParamInputField
          v-if="param.componentType === 'input-number' || param.componentType === 'input-text'"
          :model-value="toInputString(param)"
          :label="param.componentProps.label || param.name"
          :description="param.componentProps.helpText || param.description"
          :type="param.componentType === 'input-number' ? 'number' : 'text'"
          :min="param.componentType === 'input-number' ? (param.componentProps.min as string | number | undefined) : undefined"
          :max="param.componentType === 'input-number' ? (param.componentProps.max as string | number | undefined) : undefined"
          :step="param.componentType === 'input-number' ? (param.componentProps.step as string | number | undefined) : undefined"
          :input-mode="param.componentProps.inputMode"
          :placeholder="param.componentProps.placeholder"
          @update:model-value="param.componentType === 'input-number' ? handleNumberInput(param, $event) : handleTextInput(param, $event)"
        />

        <UiParamTextareaField
          v-else-if="param.componentType === 'textarea'"
          :model-value="toInputString(param)"
          :label="param.componentProps.label || param.name"
          :description="param.componentProps.helpText || param.description"
          :rows="param.componentProps.rows ?? 3"
          :placeholder="param.componentProps.placeholder"
          @update:model-value="handleTextInput(param, $event)"
        />

        <UiParamSelectField
          v-else-if="param.componentType === 'select'"
          :model-value="getFieldValue(param) as string | number | boolean | null | undefined"
          :label="param.componentProps.label || param.name"
          :description="param.componentProps.helpText || param.description"
          :options="mapSelectOptions(param.componentProps.options)"
          :placeholder="param.componentProps.placeholder || '请选择'"
          @update:model-value="handleSelectChange(param, $event)"
        />

        <UiParamSwitchField
          v-else-if="param.componentType === 'switch'"
          :model-value="Boolean(getFieldValue(param))"
          :label="param.componentProps.label || param.name"
          :description="param.componentProps.helpText || param.description"
          :text="param.componentProps.text"
          :text-on="param.componentProps.textOn"
          :text-off="param.componentProps.textOff"
          @update:model-value="handleSwitchChange(param, $event)"
        />
      </template>
    </div>
  </div>
</template>

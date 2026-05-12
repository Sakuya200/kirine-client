<script setup lang="ts">
type SupportedInputMode = 'none' | 'text' | 'tel' | 'url' | 'email' | 'numeric' | 'decimal' | 'search';

interface Props {
  modelValue: string;
  label: string;
  description?: string;
  type?: 'text' | 'number';
  min?: string | number;
  max?: string | number;
  step?: string | number;
  inputMode?: string;
  placeholder?: string;
}

const props = withDefaults(defineProps<Props>(), {
  description: '',
  type: 'text',
  min: undefined,
  max: undefined,
  step: undefined,
  inputMode: undefined,
  placeholder: undefined
});

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const getInputMode = (inputMode?: string): SupportedInputMode | undefined => {
  if (
    inputMode === 'none' ||
    inputMode === 'text' ||
    inputMode === 'tel' ||
    inputMode === 'url' ||
    inputMode === 'email' ||
    inputMode === 'numeric' ||
    inputMode === 'decimal' ||
    inputMode === 'search'
  ) {
    return inputMode;
  }

  return undefined;
};

const onInput = (event: Event) => {
  emit('update:modelValue', (event.target as HTMLInputElement).value);
};
</script>

<template>
  <label class="block">
    <span class="mb-1 block text-xs text-stone-500">{{ label }}</span>
    <input
      :value="modelValue"
      :type="type"
      :min="type === 'number' ? min : undefined"
      :max="type === 'number' ? max : undefined"
      :step="type === 'number' ? step : undefined"
      :inputmode="getInputMode(props.inputMode)"
      :placeholder="placeholder"
      class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
      @input="onInput"
    />
    <p v-if="description" class="mt-1 text-xs leading-5 text-stone-500">{{ description }}</p>
  </label>
</template>

<script setup lang="ts">
import BaseListbox from '@/components/common/BaseListbox.vue';

type SelectValue = string | number | boolean | null | undefined;

interface SelectOption {
  label: string;
  value: SelectValue;
}

interface Props {
  modelValue: SelectValue;
  label: string;
  description?: string;
  placeholder?: string;
  options: SelectOption[];
}

withDefaults(defineProps<Props>(), {
  description: '',
  placeholder: '请选择'
});

const emit = defineEmits<{
  'update:modelValue': [value: SelectValue];
}>();
</script>

<template>
  <div class="block">
    <BaseListbox
      :model-value="modelValue"
      :label="label"
      :options="options"
      :placeholder="placeholder"
      @update:model-value="emit('update:modelValue', $event)"
    />
    <p v-if="description" class="mt-1 text-xs leading-5 text-stone-500">{{ description }}</p>
  </div>
</template>

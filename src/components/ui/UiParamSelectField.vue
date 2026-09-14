<script setup lang="ts">
import { useI18n } from 'vue-i18n';
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

const { t } = useI18n();

withDefaults(defineProps<Props>(), {
  description: '',
  placeholder: undefined as unknown as string
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
      :placeholder="placeholder ?? t('common.ui.selectPlaceholder')"
      @update:model-value="emit('update:modelValue', $event)"
    />
    <p v-if="description" class="mt-1 text-xs leading-5 text-stone-500">{{ description }}</p>
  </div>
</template>

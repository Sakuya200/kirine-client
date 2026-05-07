<script setup lang="ts">
interface Props {
  modelValue: boolean;
  label: string;
  description?: string;
  text?: string;
  textOn?: string;
  textOff?: string;
}

const props = withDefaults(defineProps<Props>(), {
  description: '',
  text: undefined,
  textOn: undefined,
  textOff: undefined
});

const emit = defineEmits<{
  'update:modelValue': [value: boolean];
}>();

const currentText = () => (props.modelValue ? props.textOn || props.text || '已启用' : props.textOff || props.text || '未启用');

const onChange = (event: Event) => {
  emit('update:modelValue', (event.target as HTMLInputElement).checked);
};
</script>

<template>
  <label class="block">
    <span class="mb-1 block text-xs text-stone-500">{{ label }}</span>
    <span class="flex min-h-10 w-full items-center justify-between rounded-xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700">
      <span>{{ currentText() }}</span>
      <span class="relative inline-flex h-6 w-11 items-center">
        <input :checked="modelValue" type="checkbox" class="peer sr-only" @change="onChange" />
        <span class="absolute inset-0 rounded-full bg-stone-300 transition peer-checked:bg-brand-500" />
        <span class="absolute left-0.5 h-5 w-5 rounded-full bg-white shadow-sm transition peer-checked:translate-x-5" />
      </span>
    </span>
    <p v-if="description" class="mt-1 text-xs leading-5 text-stone-500">{{ description }}</p>
  </label>
</template>

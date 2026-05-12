<script setup lang="ts">
import { open } from '@tauri-apps/plugin-dialog';

interface Props {
  modelValue: string;
  label: string;
  description?: string;
  placeholder?: string;
  dialogTitle?: string;
  buttonText?: string;
  clearButtonText?: string;
  extensions?: string[];
}

const props = withDefaults(defineProps<Props>(), {
  description: '',
  placeholder: '尚未选择文本文件',
  dialogTitle: '选择文本文件',
  buttonText: '上传文本',
  clearButtonText: '清空',
  extensions: () => []
});

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const chooseTextFile = async () => {
  const selected = await open({
    title: props.dialogTitle,
    multiple: false,
    directory: false,
    filters: [
      {
        name: '文本文件',
        extensions: props.extensions.length > 0 ? props.extensions : ['txt', 'md', 'text']
      }
    ]
  });

  if (typeof selected === 'string' && selected.trim().length > 0) {
    emit('update:modelValue', selected);
  }
};

const clearSelection = () => {
  emit('update:modelValue', '');
};
</script>

<template>
  <div class="block md:col-span-2">
    <span class="mb-1 block text-xs text-stone-500">{{ label }}</span>
    <div class="rounded-xl border border-brand-200 bg-white/90 p-3">
      <p class="break-all text-sm text-slate-700">
        {{ modelValue || placeholder }}
      </p>

      <div class="mt-3 flex flex-wrap gap-2">
        <button
          type="button"
          class="inline-flex h-9 items-center justify-center rounded-lg bg-brand-500 px-3 text-sm font-medium text-white transition hover:bg-brand-600"
          @click="void chooseTextFile()"
        >
          {{ buttonText }}
        </button>

        <button
          v-if="modelValue"
          type="button"
          class="inline-flex h-9 items-center justify-center rounded-lg border border-brand-200 px-3 text-sm font-medium text-stone-600 transition hover:bg-brand-50"
          @click="clearSelection"
        >
          {{ clearButtonText }}
        </button>
      </div>
    </div>
    <p v-if="description" class="mt-1 text-xs leading-5 text-stone-500">{{ description }}</p>
  </div>
</template>

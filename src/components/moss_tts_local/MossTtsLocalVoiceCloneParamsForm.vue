<script setup lang="ts">
import { computed } from 'vue';

interface Props {
  modelValue: Record<string, unknown>;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  'update:modelValue': [value: Record<string, unknown>];
}>();

const nVqForInference = computed({
  get: () => Number(props.modelValue.nVqForInference ?? 32),
  set: value => {
    emit('update:modelValue', {
      ...props.modelValue,
      nVqForInference: value
    });
  }
});
</script>

<template>
  <div class="space-y-3 text-sm text-slate-700">
    <label class="block">
      <span class="mb-1 block text-xs text-stone-500">并行码本数</span>
      <input
        v-model.number="nVqForInference"
        type="number"
        min="1"
        step="1"
        class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
      />
    </label>

    <div class="rounded-2xl border border-brand-200 bg-white/80 p-3 text-xs leading-5 text-stone-500">
      MOSS 声音克隆当前仅暴露 `n_vq_for_inference`。
    </div>
  </div>
</template>

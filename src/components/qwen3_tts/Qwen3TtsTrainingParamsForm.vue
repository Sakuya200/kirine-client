<script setup lang="ts">
import { computed } from 'vue';

import { QWEN3_TRAINING_DEFAULT_PARAMS } from '@/components/qwen3_tts/trainingParams';

interface Props {
  modelValue: Record<string, unknown>;
  supportsLora?: boolean;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  'update:modelValue': [value: Record<string, unknown>];
}>();

const updateValue = (key: string, value: unknown) => {
  emit('update:modelValue', {
    ...props.modelValue,
    [key]: value
  });
};

const epochCount = computed({
  get: () => Number(props.modelValue.epochCount ?? QWEN3_TRAINING_DEFAULT_PARAMS.epochCount),
  set: value => updateValue('epochCount', value)
});

const batchSize = computed({
  get: () => Number(props.modelValue.batchSize ?? QWEN3_TRAINING_DEFAULT_PARAMS.batchSize),
  set: value => updateValue('batchSize', value)
});

const gradientAccumulationSteps = computed({
  get: () => Number(props.modelValue.gradientAccumulationSteps ?? QWEN3_TRAINING_DEFAULT_PARAMS.gradientAccumulationSteps),
  set: value => updateValue('gradientAccumulationSteps', value)
});

const enableGradientCheckpointing = computed({
  get: () => Boolean(props.modelValue.enableGradientCheckpointing ?? QWEN3_TRAINING_DEFAULT_PARAMS.enableGradientCheckpointing),
  set: value => updateValue('enableGradientCheckpointing', value)
});
</script>

<template>
  <div class="space-y-3 text-sm text-slate-700">
    <div class="grid gap-3 md:grid-cols-2">
      <label class="block">
        <span class="mb-1 block text-xs text-stone-500">训练轮次</span>
        <input v-model.number="epochCount" type="number" min="1" class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" />
      </label>
      <label class="block">
        <span class="mb-1 block text-xs text-stone-500">批次大小</span>
        <input v-model.number="batchSize" type="number" min="1" class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" />
      </label>
    </div>

    <div class="grid gap-3 md:grid-cols-2">
      <label class="block">
        <span class="mb-1 block text-xs text-stone-500">梯度累积步数</span>
        <input
          v-model.number="gradientAccumulationSteps"
          type="number"
          min="1"
          class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
        />
      </label>
      <label class="block">
        <span class="mb-1 block text-xs text-stone-500">梯度检查点</span>
        <span class="flex h-10 w-full items-center justify-between rounded-xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700">
          <span>启用梯度检查点</span>
          <span class="relative inline-flex h-6 w-11 items-center">
            <input v-model="enableGradientCheckpointing" type="checkbox" class="peer sr-only" />
            <span class="absolute inset-0 rounded-full bg-stone-300 transition peer-checked:bg-brand-500" />
            <span class="absolute left-0.5 h-5 w-5 rounded-full bg-white shadow-sm transition peer-checked:translate-x-5" />
          </span>
        </span>
      </label>
    </div>
  </div>
</template>

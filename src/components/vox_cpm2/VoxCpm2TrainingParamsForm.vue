<script setup lang="ts">
import { computed } from 'vue';

import BaseListbox from '../common/BaseListbox.vue';

interface Props {
  modelValue: Record<string, unknown>;
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

const trainingMode = computed({
  get: () => String(props.modelValue.trainingMode ?? 'lora'),
  set: value => updateValue('trainingMode', value)
});

const trainingModeOptions = [
  { label: 'LoRA 微调', value: 'lora' },
  { label: '全量微调', value: 'full' }
];

const epochCount = computed({
  get: () => Number(props.modelValue.epochCount ?? 2),
  set: value => updateValue('epochCount', value)
});

const batchSize = computed({
  get: () => Number(props.modelValue.batchSize ?? 4),
  set: value => updateValue('batchSize', value)
});

const gradientAccumulationSteps = computed({
  get: () => Number(props.modelValue.gradientAccumulationSteps ?? 1),
  set: value => updateValue('gradientAccumulationSteps', value)
});

const enableGradientCheckpointing = computed({
  get: () => Boolean(props.modelValue.enableGradientCheckpointing ?? false),
  set: value => updateValue('enableGradientCheckpointing', value)
});
</script>

<template>
  <div class="space-y-3 text-sm text-slate-700">
    <label class="block">
      <span class="mb-1 block text-xs text-stone-500">微调模式</span>
      <BaseListbox v-model="trainingMode" :options="trainingModeOptions" />
    </label>

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

    <div class="rounded-2xl border border-brand-200 bg-white/80 p-3 text-xs leading-5 text-stone-500">
      <p v-if="trainingMode === 'lora'">LoRA 模式会使用固定默认超参：LM/DiT 开启，Projection 关闭，rank 32，alpha 32，dropout 0.0。</p>
      <p v-else>全量微调会直接更新完整模型权重，显存需求明显更高，建议优先确认本机硬件资源充足。</p>
    </div>
  </div>
</template>

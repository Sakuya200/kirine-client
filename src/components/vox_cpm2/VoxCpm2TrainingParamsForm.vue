<script setup lang="ts">
import { computed } from 'vue';

import { VOX_CPM2_TRAINING_DEFAULT_PARAMS, normalizeVoxCpm2TrainingParams } from '@/components/vox_cpm2/trainingParams';

interface Props {
  modelValue: Record<string, unknown>;
  supportsLora?: boolean;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  'update:modelValue': [value: Record<string, unknown>];
}>();

const normalizedModelValue = computed(() => normalizeVoxCpm2TrainingParams(props.modelValue));

const updateValue = (key: string, value: unknown) => {
  emit('update:modelValue', {
    ...props.modelValue,
    [key]: value
  });
};

const updateLoraState = (value: boolean) => {
  emit('update:modelValue', {
    ...props.modelValue,
    useLora: value,
    trainingMode: value ? 'lora' : 'full'
  });
};

const useLora = computed({
  get: () => {
    return Boolean(normalizedModelValue.value.useLora);
  },
  set: value => updateLoraState(value)
});

const loraRank = computed({
  get: () => Number(normalizedModelValue.value.loraRank ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraRank),
  set: value => updateValue('loraRank', value)
});

const loraAlpha = computed({
  get: () => Number(normalizedModelValue.value.loraAlpha ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraAlpha),
  set: value => updateValue('loraAlpha', value)
});

const loraDropout = computed({
  get: () => String(normalizedModelValue.value.loraDropout ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.loraDropout),
  set: value => updateValue('loraDropout', value)
});

const epochCount = computed({
  get: () => Number(normalizedModelValue.value.epochCount ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.epochCount),
  set: value => updateValue('epochCount', value)
});

const batchSize = computed({
  get: () => Number(normalizedModelValue.value.batchSize ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.batchSize),
  set: value => updateValue('batchSize', value)
});

const gradientAccumulationSteps = computed({
  get: () => Number(normalizedModelValue.value.gradientAccumulationSteps ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.gradientAccumulationSteps),
  set: value => updateValue('gradientAccumulationSteps', value)
});

const enableGradientCheckpointing = computed({
  get: () => Boolean(normalizedModelValue.value.enableGradientCheckpointing ?? VOX_CPM2_TRAINING_DEFAULT_PARAMS.enableGradientCheckpointing),
  set: value => updateValue('enableGradientCheckpointing', value)
});
</script>

<template>
  <div class="space-y-3 text-sm text-slate-700">
    <section v-if="supportsLora" class="space-y-3 rounded-2xl border border-brand-200 bg-brand-50/40 p-4">
      <header class="space-y-1">
        <h3 class="text-sm font-semibold text-slate-800">LoRA 微调配置</h3>
        <p class="text-xs leading-5 text-stone-500">当前模型支持 LoRA。关闭后将退回为全量微调。</p>
      </header>
      <label class="block">
        <span class="mb-1 block text-xs text-stone-500">LoRA 开关</span>
        <span class="flex h-10 w-full items-center justify-between rounded-xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700">
          <span>{{ useLora ? '启用 LoRA 微调' : '使用全量微调' }}</span>
          <span class="relative inline-flex h-6 w-11 items-center">
            <input v-model="useLora" type="checkbox" class="peer sr-only" />
            <span class="absolute inset-0 rounded-full bg-stone-300 transition peer-checked:bg-brand-500" />
            <span class="absolute left-0.5 h-5 w-5 rounded-full bg-white shadow-sm transition peer-checked:translate-x-5" />
          </span>
        </span>
      </label>

      <div v-if="useLora" class="grid gap-3 md:grid-cols-3">
        <label class="block">
          <span class="mb-1 block text-xs text-stone-500">Rank</span>
          <input v-model.number="loraRank" type="number" min="1" class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" />
        </label>
        <label class="block">
          <span class="mb-1 block text-xs text-stone-500">Alpha</span>
          <input v-model.number="loraAlpha" type="number" min="1" class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" />
        </label>
        <label class="block">
          <span class="mb-1 block text-xs text-stone-500">Dropout</span>
          <input v-model="loraDropout" type="text" inputmode="decimal" class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" />
        </label>
      </div>
    </section>

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
      <p v-if="supportsLora && useLora">LoRA 模式仅训练适配器参数，默认更省显存；当前可直接在表单中调整 rank、alpha 和 dropout。</p>
      <p v-else-if="supportsLora">全量微调会直接更新完整模型权重，显存需求明显更高，建议优先确认本机硬件资源充足。</p>
      <p v-else>当前模型未声明 LoRA 能力，本次训练将使用全量微调参数。</p>
    </div>
  </div>
</template>

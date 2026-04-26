<script setup lang="ts">
import { computed } from 'vue';

import { MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS } from '@/components/moss_tts_local/trainingParams';

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
  get: () => Number(props.modelValue.epochCount ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.epochCount),
  set: value => updateValue('epochCount', value)
});

const batchSize = computed({
  get: () => Number(props.modelValue.batchSize ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.batchSize),
  set: value => updateValue('batchSize', value)
});

const gradientAccumulationSteps = computed({
  get: () => Number(props.modelValue.gradientAccumulationSteps ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.gradientAccumulationSteps),
  set: value => updateValue('gradientAccumulationSteps', value)
});

const enableGradientCheckpointing = computed({
  get: () => Boolean(props.modelValue.enableGradientCheckpointing ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.enableGradientCheckpointing),
  set: value => updateValue('enableGradientCheckpointing', value)
});

const learningRate = computed({
  get: () => String(props.modelValue.learningRate ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.learningRate),
  set: value => updateValue('learningRate', value)
});

const weightDecay = computed({
  get: () => String(props.modelValue.weightDecay ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.weightDecay),
  set: value => updateValue('weightDecay', value)
});

const warmupRatio = computed({
  get: () => String(props.modelValue.warmupRatio ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.warmupRatio),
  set: value => updateValue('warmupRatio', value)
});

const warmupSteps = computed({
  get: () => Number(props.modelValue.warmupSteps ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.warmupSteps),
  set: value => updateValue('warmupSteps', value)
});

const maxGradNorm = computed({
  get: () => String(props.modelValue.maxGradNorm ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.maxGradNorm),
  set: value => updateValue('maxGradNorm', value)
});

const mixedPrecision = computed({
  get: () => String(props.modelValue.mixedPrecision ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.mixedPrecision),
  set: value => updateValue('mixedPrecision', value)
});

const channelwiseLossWeight = computed({
  get: () => String(props.modelValue.channelwiseLossWeight ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.channelwiseLossWeight),
  set: value => updateValue('channelwiseLossWeight', value)
});

const skipReferenceAudioCodes = computed({
  get: () => Boolean(props.modelValue.skipReferenceAudioCodes ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.skipReferenceAudioCodes),
  set: value => updateValue('skipReferenceAudioCodes', value)
});

const prepBatchSize = computed({
  get: () => Number(props.modelValue.prepBatchSize ?? MOSS_TTS_LOCAL_TRAINING_DEFAULT_PARAMS.prepBatchSize),
  set: value => updateValue('prepBatchSize', value)
});

const prepNVq = computed({
  get: () => (props.modelValue.prepNVq == null ? '' : String(props.modelValue.prepNVq)),
  set: value => updateValue('prepNVq', value.trim().length === 0 ? null : Number(value))
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
        <span class="mb-1 block text-xs text-stone-500">预处理批次大小</span>
        <input v-model.number="prepBatchSize" type="number" min="1" class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" />
      </label>
    </div>

    <div class="grid gap-3 md:grid-cols-2">
      <label class="block">
        <span class="mb-1 block text-xs text-stone-500">学习率</span>
        <input
          v-model="learningRate"
          type="number"
          min="0"
          step="0.000001"
          class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
        />
      </label>
      <label class="block">
        <span class="mb-1 block text-xs text-stone-500">权重衰减</span>
        <input v-model="weightDecay" type="number" min="0" step="0.01" class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" />
      </label>
    </div>

    <div class="grid gap-3 md:grid-cols-2 lg:grid-cols-4">
      <label class="block lg:col-span-1">
        <span class="mb-1 block text-xs text-stone-500">Warmup Ratio</span>
        <input v-model="warmupRatio" type="number" min="0" step="0.01" class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" />
      </label>
      <label class="block lg:col-span-1">
        <span class="mb-1 block text-xs text-stone-500">Warmup Steps</span>
        <input
          v-model.number="warmupSteps"
          type="number"
          min="0"
          step="1"
          class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
        />
      </label>
      <label class="block lg:col-span-1">
        <span class="mb-1 block text-xs text-stone-500">Max Grad Norm</span>
        <input v-model="maxGradNorm" type="number" min="0" step="0.1" class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" />
      </label>
      <label class="block lg:col-span-1">
        <span class="mb-1 block text-xs text-stone-500">预处理 nVQ</span>
        <input
          v-model="prepNVq"
          type="number"
          min="1"
          step="1"
          class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
          placeholder="留空则使用脚本默认值"
        />
      </label>
    </div>

    <div class="grid gap-3 md:grid-cols-2">
      <label class="block">
        <span class="mb-1 block text-xs text-stone-500">Mixed Precision</span>
        <input
          v-model="mixedPrecision"
          type="text"
          class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
          placeholder="例如 bf16"
        />
      </label>
      <label class="block">
        <span class="mb-1 block text-xs text-stone-500">Channelwise Loss Weight</span>
        <input
          v-model="channelwiseLossWeight"
          type="text"
          class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
          placeholder="例如 1,32"
        />
      </label>
    </div>

    <div class="grid gap-3 md:grid-cols-2">
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
      <label class="block">
        <span class="mb-1 block text-xs text-stone-500">跳过参考音频编码</span>
        <span class="flex h-10 w-full items-center justify-between rounded-xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700">
          <span>{{ skipReferenceAudioCodes ? '跳过 reference audio codes' : '保留 reference audio codes' }}</span>
          <span class="relative inline-flex h-6 w-11 items-center">
            <input v-model="skipReferenceAudioCodes" type="checkbox" class="peer sr-only" />
            <span class="absolute inset-0 rounded-full bg-stone-300 transition peer-checked:bg-brand-500" />
            <span class="absolute left-0.5 h-5 w-5 rounded-full bg-white shadow-sm transition peer-checked:translate-x-5" />
          </span>
        </span>
      </label>
    </div>

    <div class="rounded-2xl border border-brand-200 bg-white/80 p-3 text-xs leading-5 text-stone-500">
      MOSS-TTS Local 当前使用单卡训练流水线。页面参数会直接透传到本地 `training.py` 包装脚本，不启用 FSDP 或 DeepSpeed。
    </div>
  </div>
</template>

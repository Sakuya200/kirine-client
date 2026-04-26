<script setup lang="ts">
import { computed } from 'vue';

import { VOX_CPM2_TTS_DEFAULT_PARAMS } from '@/components/vox_cpm2/textToSpeechParams';

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

const cfgValue = computed({
  get: () => String(props.modelValue.cfgValue ?? VOX_CPM2_TTS_DEFAULT_PARAMS.cfgValue),
  set: value => updateValue('cfgValue', value)
});

const inferenceTimesteps = computed({
  get: () => Number(props.modelValue.inferenceTimesteps ?? VOX_CPM2_TTS_DEFAULT_PARAMS.inferenceTimesteps),
  set: value => updateValue('inferenceTimesteps', value)
});
</script>

<template>
  <div class="space-y-3 text-sm text-slate-700">
    <div class="grid gap-3 md:grid-cols-2">
      <label class="block">
        <span class="mb-1 block text-xs text-stone-500">CFG 值</span>
        <input v-model="cfgValue" type="number" min="0.1" step="0.1" class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" />
      </label>
      <label class="block">
        <span class="mb-1 block text-xs text-stone-500">推理步数</span>
        <input
          v-model.number="inferenceTimesteps"
          type="number"
          min="1"
          step="1"
          class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
        />
      </label>
    </div>

    <div class="rounded-2xl border border-brand-200 bg-white/80 p-3 text-xs leading-5 text-stone-500">
      VoxCPM2 会直接基于基础模型生成语音，不依赖现有说话人。CFG 值越高通常更偏向文本条件，推理步数越高通常更稳定但更慢。
    </div>
  </div>
</template>

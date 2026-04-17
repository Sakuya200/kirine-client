<script setup lang="ts">
import { computed } from 'vue';

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

const mode = computed({
  get: () => String(props.modelValue.mode ?? 'reference'),
  set: value => updateValue('mode', value)
});

const stylePrompt = computed({
  get: () => String(props.modelValue.stylePrompt ?? ''),
  set: value => updateValue('stylePrompt', value)
});

const cfgValue = computed({
  get: () => Number(props.modelValue.cfgValue ?? 2.0),
  set: value => updateValue('cfgValue', value)
});

const inferenceTimesteps = computed({
  get: () => Number(props.modelValue.inferenceTimesteps ?? 10),
  set: value => updateValue('inferenceTimesteps', value)
});
</script>

<template>
  <div class="space-y-3 text-sm text-slate-700">
    <label class="block">
      <span class="mb-1 block text-xs text-stone-500">克隆模式</span>
      <select v-model="mode" class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2">
        <option value="reference">参考音频克隆</option>
        <option value="ultimate">Ultimate 克隆</option>
      </select>
    </label>

    <label class="block">
      <span class="mb-1 block text-xs text-stone-500">风格提示词</span>
      <textarea
        v-model="stylePrompt"
        rows="3"
        class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2"
        placeholder="例如：稍快一点、情绪更轻快、语气更坚定。"
      />
    </label>

    <div class="grid gap-3 md:grid-cols-2">
      <label class="block">
        <span class="mb-1 block text-xs text-stone-500">CFG 值</span>
        <input
          v-model.number="cfgValue"
          type="number"
          min="0.1"
          step="0.1"
          class="h-10 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
        />
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
      <p v-if="mode === 'ultimate'">Ultimate 克隆会把当前参考音频同时作为 prompt audio 和 reference audio，并要求页面上的参考台词与音频逐字对应。</p>
      <p v-else>参考音频克隆只要求参考音频本身，风格提示词会以前缀文本的形式注入到生成请求。</p>
    </div>
  </div>
</template>

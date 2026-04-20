<script setup lang="ts">
import { computed } from 'vue';

interface Props {
  modelValue: Record<string, unknown>;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  'update:modelValue': [value: Record<string, unknown>];
}>();

const voicePrompt = computed({
  get: () => String(props.modelValue.voicePrompt ?? ''),
  set: value => {
    emit('update:modelValue', {
      ...props.modelValue,
      voicePrompt: value
    });
  }
});
</script>

<template>
  <label class="block text-sm text-slate-700">
    <span class="mb-1 block text-xs text-stone-500">声音 Prompt</span>
    <textarea
      v-model="voicePrompt"
      rows="4"
      class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2"
      placeholder="例如：温柔、自然、轻微微笑感，适合长句播报。"
    />
    <p class="mt-2 text-xs leading-5 text-stone-500">用于描述目标语气、情绪、节奏或播报风格。</p>
  </label>
</template>

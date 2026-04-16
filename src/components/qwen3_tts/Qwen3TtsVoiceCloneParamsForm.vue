<script setup lang="ts">
interface Props {
  modelValue: Record<string, unknown>;
}

defineProps<Props>();
</script>

<template>
  <div class="rounded-2xl border border-brand-200 bg-white/80 p-4 text-sm text-stone-500">
    当前 Qwen3-TTS 声音克隆没有额外的模型专属参数，参考音频与文本输入即为主要控制项。
  </div>
</template>

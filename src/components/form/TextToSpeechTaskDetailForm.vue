<script setup lang="ts">
import { computed } from 'vue';

import { APP_LANGUAGE_LABELS } from '@/enums/language';
import { TEXT_TO_SPEECH_FORMATS } from '@/enums/textToSpeech';
import { useModelStore } from '@/stores/models';
import type { TextToSpeechHistoryRecord } from '@/types/domain';

interface Props {
  record: TextToSpeechHistoryRecord;
}

const props = defineProps<Props>();
const modelStore = useModelStore();

const formatLabel = computed(
  () => TEXT_TO_SPEECH_FORMATS.find(option => option.value === props.record.detail.format)?.label ?? props.record.detail.format
);
const baseModelLabel = computed(() => modelStore.getModelLabel(props.record.detail.baseModel));
const voicePrompt = computed(() => String(props.record.detail.modelParams.voicePrompt ?? ''));
</script>

<template>
  <div class="space-y-4">
    <div class="grid gap-3 md:grid-cols-2">
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">说话人</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ record.speaker }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">语言</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ APP_LANGUAGE_LABELS[record.detail.language] }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">基础模型</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ baseModelLabel }} {{ record.detail.modelScale }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">输出格式</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ formatLabel }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4 md:col-span-2">
        <p class="text-xs text-stone-500">导出音频名称</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ record.detail.exportAudioName }}</p>
      </article>
    </div>

    <div class="grid gap-3 md:grid-cols-2">
      <article class="rounded-2xl border border-brand-200 bg-brand-50/55 p-4">
        <p class="text-xs text-brand-700">字符数</p>
        <p class="mt-1 text-lg font-semibold text-brand-900">{{ record.detail.charCount }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">输出文件</p>
        <p class="mt-1 break-all text-sm font-semibold text-slate-800">{{ record.detail.fileName }}</p>
      </article>
    </div>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">输出路径</p>
      <p class="mt-3 break-all rounded-xl bg-brand-50/45 px-3 py-3 text-sm leading-6 text-slate-700">
        {{ record.detail.outputFilePath || '当前任务尚未生成可用输出文件。' }}
      </p>
    </section>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">输入文本</p>
      <p class="mt-3 whitespace-pre-wrap rounded-xl bg-brand-50/45 px-3 py-3 text-sm leading-6 text-slate-700">{{ record.detail.text }}</p>
    </section>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">模型特定参数</p>
      <p class="mt-3 whitespace-pre-wrap rounded-xl bg-brand-50/45 px-3 py-3 text-sm leading-6 text-slate-700">
        {{ voicePrompt || '未填写声音 Prompt，使用默认风格。' }}
      </p>
    </section>
  </div>
</template>

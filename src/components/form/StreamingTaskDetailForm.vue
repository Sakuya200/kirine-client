<script setup lang="ts">
import { computed } from 'vue';

import { APP_LANGUAGE_LABELS } from '@/enums/language';
import { HARDWARE_TYPE_TEXT, HardwareType } from '@/enums/settings';
import { useModelStore } from '@/stores/models';
import type { StreamingSpeechHistoryRecord } from '@/types/domain';

interface Props {
  record: StreamingSpeechHistoryRecord;
}

const props = defineProps<Props>();
const modelStore = useModelStore();

const baseModelLabel = computed(() => modelStore.getModelLabel(props.record.detail.baseModel));
const modelParamsText = computed(() => JSON.stringify(props.record.detail.modelParams ?? {}, null, 2));
</script>

<template>
  <div class="space-y-4">
    <div class="grid gap-3 md:grid-cols-2">
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">基础模型</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ baseModelLabel }} {{ record.detail.modelVersion }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">语言</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ APP_LANGUAGE_LABELS[record.detail.language] }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">设备类型</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">
          {{ HARDWARE_TYPE_TEXT[record.detail.device as HardwareType] ?? record.detail.device.toUpperCase() }}
        </p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-brand-50/55 p-4">
        <p class="text-xs text-brand-700">已生成消息数</p>
        <p class="mt-1 text-lg font-semibold text-brand-900">{{ record.detail.messageCount }}</p>
      </article>
    </div>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">模型参数</p>
      <pre class="mt-3 overflow-x-auto rounded-xl bg-brand-50/45 px-3 py-3 text-xs leading-6 text-slate-700">{{ modelParamsText }}</pre>
    </section>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">会话上下文文件</p>
      <p class="mt-3 break-all rounded-xl bg-brand-50/45 px-3 py-3 text-sm leading-6 text-slate-700">{{ record.detail.contextFilePath }}</p>
    </section>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">输入缓存文件</p>
      <p class="mt-3 break-all rounded-xl bg-brand-50/45 px-3 py-3 text-sm leading-6 text-slate-700">{{ record.detail.inputCacheFilePath }}</p>
    </section>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">输出音频目录</p>
      <p class="mt-3 break-all rounded-xl bg-brand-50/45 px-3 py-3 text-sm leading-6 text-slate-700">{{ record.detail.outputAudioDir }}</p>
    </section>
  </div>
</template>

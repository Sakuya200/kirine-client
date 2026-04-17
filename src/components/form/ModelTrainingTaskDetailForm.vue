<script setup lang="ts">
import { computed } from 'vue';

import { APP_LANGUAGE_LABELS } from '@/enums/language';
import { useModelStore } from '@/stores/models';
import type { ModelTrainingHistoryRecord } from '@/types/domain';

interface Props {
  record: ModelTrainingHistoryRecord;
}

const props = defineProps<Props>();
const modelStore = useModelStore();
const modelLabel = computed(() => modelStore.getModelLabel(props.record.detail.baseModel));
</script>

<template>
  <div class="space-y-4">
    <div class="grid gap-3 md:grid-cols-2">
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">模型名称</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ record.detail.modelName }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">训练语言</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ APP_LANGUAGE_LABELS[record.detail.language] }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">基础模型</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ modelLabel }} {{ record.detail.modelScale }}</p>
      </article>
    </div>

    <div class="grid gap-3 md:grid-cols-1">
      <article class="rounded-2xl border border-brand-200 bg-brand-50/55 p-4">
        <p class="text-xs text-brand-700">样本数</p>
        <p class="mt-1 text-lg font-semibold text-brand-900">{{ record.detail.sampleCount }}</p>
      </article>
    </div>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">模型特定参数</p>
      <p class="mt-3 break-all rounded-xl bg-brand-50/45 px-3 py-3 text-sm leading-6 text-slate-700">
        {{ Object.keys(record.detail.modelParams).length > 0 ? JSON.stringify(record.detail.modelParams, null, 2) : '当前模型没有额外参数。' }}
      </p>
    </section>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">训练说明</p>
      <ul class="mt-3 space-y-2 text-sm text-slate-600">
        <li v-for="note in record.detail.notes" :key="note" class="rounded-xl bg-brand-50/45 px-3 py-2">{{ note }}</li>
      </ul>
    </section>
  </div>
</template>

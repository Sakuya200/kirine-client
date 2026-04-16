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
const modelParamEntries = computed(() =>
  Object.entries(props.record.detail.modelParams ?? {}).map(([key, value]) => ({
    key,
    label:
      key === 'epochCount'
        ? '训练轮次'
        : key === 'batchSize'
          ? '批次大小'
          : key === 'gradientAccumulationSteps'
            ? '梯度累积步数'
            : key === 'enableGradientCheckpointing'
              ? '梯度检查点'
              : key,
    value: typeof value === 'boolean' ? (value ? '启用' : '禁用') : String(value ?? '')
  }))
);
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
      <div v-if="modelParamEntries.length > 0" class="mt-3 grid gap-3 md:grid-cols-2">
        <article v-for="item in modelParamEntries" :key="item.key" class="rounded-2xl border border-brand-200 bg-brand-50/45 p-4">
          <p class="text-xs text-stone-500">{{ item.label }}</p>
          <p class="mt-1 text-sm font-semibold text-slate-800">{{ item.value }}</p>
        </article>
      </div>
      <p v-else class="mt-3 rounded-xl bg-brand-50/45 px-3 py-3 text-sm leading-6 text-slate-700">当前模型没有额外参数。</p>
    </section>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">训练说明</p>
      <ul class="mt-3 space-y-2 text-sm text-slate-600">
        <li v-for="note in record.detail.notes" :key="note" class="rounded-xl bg-brand-50/45 px-3 py-2">{{ note }}</li>
      </ul>
    </section>
  </div>
</template>

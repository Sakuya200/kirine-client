<script setup lang="ts">
import { BASE_MODEL_TEXT } from '@/enums/settings';
import { APP_LANGUAGE_LABELS } from '@/enums/language';
import type { ModelTrainingHistoryRecord } from '@/types/domain';

interface Props {
  record: ModelTrainingHistoryRecord;
}

defineProps<Props>();
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
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ BASE_MODEL_TEXT[record.detail.baseModel] }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">训练轮次</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ record.detail.epochCount }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">批次大小</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ record.detail.batchSize }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">梯度累积步数</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ record.detail.gradientAccumulationSteps }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">梯度检查点</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ record.detail.enableGradientCheckpointing ? '启用' : '禁用' }}</p>
      </article>
    </div>

    <div class="grid gap-3 md:grid-cols-1">
      <article class="rounded-2xl border border-brand-200 bg-brand-50/55 p-4">
        <p class="text-xs text-brand-700">样本数</p>
        <p class="mt-1 text-lg font-semibold text-brand-900">{{ record.detail.sampleCount }}</p>
      </article>
    </div>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">训练说明</p>
      <ul class="mt-3 space-y-2 text-sm text-slate-600">
        <li v-for="note in record.detail.notes" :key="note" class="rounded-xl bg-brand-50/45 px-3 py-2">{{ note }}</li>
      </ul>
    </section>
  </div>
</template>

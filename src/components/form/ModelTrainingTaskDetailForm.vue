<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';

import { APP_LANGUAGE_LABELS } from '@/enums/language';
import { HARDWARE_TYPE_TEXT, HardwareType } from '@/enums/settings';
import { useModels } from '@/hooks/useModels';
import type { ModelTrainingHistoryRecord } from '@/types/domain';

interface Props {
  record: ModelTrainingHistoryRecord;
}

const { t } = useI18n();

const props = defineProps<Props>();
const { getModelLabel } = useModels();
const modelLabel = computed(() => getModelLabel(props.record.detail.baseModel));
const trainingParamsSummary = computed(() => {
  const params = props.record.detail.modelParams ?? {};
  const batchSize = Number(params.batchSize ?? 0);
  const gradientAccumulationSteps = Number(params.gradientAccumulationSteps ?? 0);
  const epochCount = Number(params.epochCount ?? 0);

  return {
    epochCount,
    batchSize,
    gradientAccumulationSteps,
    effectiveBatchSize: Math.max(0, batchSize) * Math.max(0, gradientAccumulationSteps),
    useLora: Boolean(params.useLora ?? (params.trainingMode ?? 'lora') !== 'full')
  };
});
</script>

<template>
  <div class="space-y-4">
    <div class="grid gap-3 md:grid-cols-2">
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">{{ t('history.detailForm.speakerName') }}</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ record.detail.speakerName }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">{{ t('history.detailForm.trainingLanguage') }}</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ APP_LANGUAGE_LABELS[record.detail.language] }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">{{ t('history.detailForm.baseModel') }}</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ modelLabel }} {{ record.detail.modelVersion }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">{{ t('history.detailForm.deviceType') }}</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">
          {{ HARDWARE_TYPE_TEXT[record.device as HardwareType] ?? record.device.toUpperCase() }}
        </p>
      </article>
    </div>

    <div class="grid gap-3 md:grid-cols-1">
      <article class="rounded-2xl border border-brand-200 bg-brand-50/55 p-4">
        <p class="text-xs text-brand-700">{{ t('history.detailForm.sampleCount') }}</p>
        <p class="mt-1 text-lg font-semibold text-brand-900">{{ record.detail.sampleCount }}</p>
      </article>
    </div>

    <div class="grid gap-3 md:grid-cols-4">
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">{{ t('history.detailForm.epochCount') }}</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ trainingParamsSummary.epochCount || '-' }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">{{ t('history.detailForm.batchSize') }}</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ trainingParamsSummary.batchSize || '-' }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">{{ t('history.detailForm.gradAccum') }}</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ trainingParamsSummary.gradientAccumulationSteps || '-' }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">{{ t('history.detailForm.perStepSamples') }}</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ trainingParamsSummary.effectiveBatchSize || '-' }}</p>
      </article>
    </div>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">{{ t('history.detailForm.modelParams') }}</p>
      <p class="mt-3 break-all rounded-xl bg-brand-50/45 px-3 py-3 text-sm leading-6 text-slate-700">
        {{ Object.keys(record.detail.modelParams).length > 0 ? JSON.stringify(record.detail.modelParams, null, 2) : t('history.detailForm.noExtraParams') }}
      </p>
      <p class="mt-3 text-xs text-stone-500">
        {{ trainingParamsSummary.useLora ? t('history.detailForm.loraEnabled') : t('history.detailForm.loraDisabled') }}
      </p>
    </section>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">{{ t('history.detailForm.trainingNotes') }}</p>
      <ul class="mt-3 space-y-2 text-sm text-slate-600">
        <li v-for="note in record.detail.notes" :key="note" class="rounded-xl bg-brand-50/45 px-3 py-2">{{ note }}</li>
      </ul>
    </section>
  </div>
</template>

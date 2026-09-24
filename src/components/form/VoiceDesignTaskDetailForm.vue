<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';

import { APP_LANGUAGE_LABELS } from '@/enums/language';
import { HARDWARE_TYPE_TEXT, HardwareType } from '@/enums/settings';
import { TEXT_TO_SPEECH_FORMATS } from '@/enums/textToSpeech';
import { useModels } from '@/hooks/useModels';
import type { VoiceDesignHistoryRecord } from '@/types/domain';

interface Props {
  record: VoiceDesignHistoryRecord;
}

const props = defineProps<Props>();
const { getModelLabel } = useModels();
const { t } = useI18n();

const baseModelLabel = computed(() => getModelLabel(props.record.detail.baseModel));
const formatLabel = computed(() => {
  const found = TEXT_TO_SPEECH_FORMATS.find(option => option.value === props.record.detail.format);
  return found ? t(found.label) : props.record.detail.format;
});
</script>

<template>
  <div class="space-y-4">
    <div class="grid gap-3 md:grid-cols-2">
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">{{ t('history.detailForm.language') }}</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ APP_LANGUAGE_LABELS[record.detail.language] }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">{{ t('history.detailForm.outputFormat') }}</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ formatLabel }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">{{ t('history.detailForm.baseModel') }}</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ baseModelLabel }} {{ record.detail.modelVersion }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">{{ t('history.detailForm.deviceType') }}</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">
          {{ HARDWARE_TYPE_TEXT[record.device as HardwareType] ?? record.device.toUpperCase() }}
        </p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4 md:col-span-2">
        <p class="text-xs text-stone-500">{{ t('history.detailForm.exportAudioName') }}</p>
        <p class="mt-1 text-sm font-semibold text-slate-800">{{ record.detail.exportAudioName }}</p>
      </article>
    </div>

    <div class="grid gap-3 md:grid-cols-2">
      <article class="rounded-2xl border border-brand-200 bg-brand-50/55 p-4">
        <p class="text-xs text-brand-700">{{ t('history.detailForm.charCount') }}</p>
        <p class="mt-1 text-lg font-semibold text-brand-900">{{ record.detail.charCount }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
        <p class="text-xs text-stone-500">{{ t('history.detailForm.outputFile') }}</p>
        <p class="mt-1 break-all text-sm font-semibold text-slate-800">{{ record.detail.fileName }}</p>
      </article>
    </div>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">{{ t('history.detailForm.voicePrompt') }}</p>
      <p class="mt-3 whitespace-pre-wrap rounded-xl bg-brand-50/45 px-3 py-3 text-sm leading-6 text-slate-700">{{ record.detail.prompt }}</p>
    </section>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">{{ t('history.detailForm.targetText') }}</p>
      <p class="mt-3 whitespace-pre-wrap rounded-xl bg-brand-50/45 px-3 py-3 text-sm leading-6 text-slate-700">{{ record.detail.text }}</p>
    </section>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">{{ t('history.detailForm.modelParams') }}</p>
      <p class="mt-3 break-all rounded-xl bg-brand-50/45 px-3 py-3 text-sm leading-6 text-slate-700">
        {{ Object.keys(record.detail.modelParams).length > 0 ? JSON.stringify(record.detail.modelParams, null, 2) : t('history.detailForm.noExtraParams') }}
      </p>
    </section>

    <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
      <p class="text-sm font-semibold text-slate-800">{{ t('history.detailForm.outputPath') }}</p>
      <p class="mt-3 break-all rounded-xl bg-brand-50/45 px-3 py-3 text-sm leading-6 text-slate-700">
        {{ record.detail.outputFilePath || t('history.detailForm.noOutputFile') }}
      </p>
    </section>
  </div>
</template>

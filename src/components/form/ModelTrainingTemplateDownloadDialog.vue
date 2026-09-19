<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { DocumentArrowDownIcon, DocumentTextIcon, TableCellsIcon } from '@heroicons/vue/24/outline';
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseDialog from '@/components/common/BaseDialog.vue';
import { MODEL_TRAINING_ANNOTATION_FORMAT_TEXT, ModelTrainingAnnotationFormat } from '@/enums/modelTraining';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useUiStore } from '@/stores/ui';

interface Props {
  open: boolean;
}

defineProps<Props>();

const emit = defineEmits<{
  close: [];
}>();

const { t } = useI18n();

const uiStore = useUiStore();
const downloadingFormat = ref<ModelTrainingAnnotationFormat | null>(null);

const downloadTemplate = async (format: ModelTrainingAnnotationFormat) => {
  downloadingFormat.value = format;
  try {
    const saved = await invoke<boolean>('save_model_training_template_as', {
      templateFormat: format
    });

    if (saved) {
      uiStore.notifySuccess(format === ModelTrainingAnnotationFormat.Xlsx ? t('training.template.xlsxSaved') : t('training.template.jsonlSaved'), 2200);
    }
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('training.template.saveFailed'), error));
  } finally {
    downloadingFormat.value = null;
  }
};

const templateCards = [
  {
    format: ModelTrainingAnnotationFormat.Jsonl,
    title: MODEL_TRAINING_ANNOTATION_FORMAT_TEXT[ModelTrainingAnnotationFormat.Jsonl],
    description: t('training.template.jsonlDesc'),
    hint: t('training.template.jsonlHint'),
    icon: DocumentTextIcon
  },
  {
    format: ModelTrainingAnnotationFormat.Xlsx,
    title: MODEL_TRAINING_ANNOTATION_FORMAT_TEXT[ModelTrainingAnnotationFormat.Xlsx],
    description: t('training.template.xlsxDesc'),
    hint: t('training.template.xlsxHint'),
    icon: TableCellsIcon
  }
] as const;
</script>

<template>
  <BaseDialog
    :open="open"
    :title="t('training.template.dialogTitle')"
    panel-class="max-w-2xl sm:max-w-3xl"
    content-class="overflow-visible"
    z-class="z-[160]"
    @close="emit('close')"
  >
    <div class="space-y-4">
      <p class="text-sm leading-6 text-stone-600">
        {{ t('training.template.dialogIntro') }}
      </p>

      <div class="grid items-stretch gap-3 md:grid-cols-2">
        <article v-for="card in templateCards" :key="card.format" class="flex h-full flex-col rounded-2xl border border-brand-200 bg-brand-50/40 p-4">
          <div class="flex flex-1 items-start justify-between gap-3">
            <div class="flex-1">
              <div class="flex items-center gap-2">
                <component :is="card.icon" class="h-5 w-5 text-brand-600" aria-hidden="true" />
                <p class="text-sm font-semibold text-slate-800">{{ card.title }}</p>
              </div>
              <p class="mt-2 text-xs leading-5 text-stone-500">{{ card.description }}</p>
              <p class="mt-3 rounded-xl bg-white/90 px-3 py-2 text-[11px] text-stone-500">{{ card.hint }}</p>
            </div>
          </div>

          <BaseButton
            class="mt-4 min-h-[42px]"
            block
            :loading="downloadingFormat === card.format"
            :disabled="downloadingFormat !== null && downloadingFormat !== card.format"
            @click="downloadTemplate(card.format)"
          >
            <DocumentArrowDownIcon class="h-4 w-4" aria-hidden="true" />
            <span>{{
              downloadingFormat === card.format
                ? t('training.template.downloading')
                : card.format === ModelTrainingAnnotationFormat.Xlsx
                  ? t('training.template.downloadXlsx')
                  : t('training.template.downloadNamed', { title: card.title })
            }}</span>
          </BaseButton>
        </article>
      </div>

      <div class="rounded-2xl border border-brand-200 bg-white/90 p-4 text-xs leading-5 text-stone-500">
        <p>{{ t('training.template.xlsxSupport') }}</p>
        <p class="mt-1">{{ t('training.template.oggHint') }}</p>
      </div>
    </div>

    <template #footer>
      <BaseButton tone="ghost" @click="emit('close')">{{ t('training.template.close') }}</BaseButton>
    </template>
  </BaseDialog>
</template>

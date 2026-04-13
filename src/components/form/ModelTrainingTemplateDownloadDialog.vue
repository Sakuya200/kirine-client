<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { DocumentArrowDownIcon, DocumentTextIcon, TableCellsIcon } from '@heroicons/vue/24/outline';

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

const uiStore = useUiStore();

const downloadTemplate = async (format: ModelTrainingAnnotationFormat) => {
  try {
    const saved = await invoke<boolean>('save_model_training_template_as', {
      templateFormat: format
    });

    if (saved) {
      uiStore.notifySuccess(format === ModelTrainingAnnotationFormat.Xlsx ? 'Excel 模板已保存。' : 'JSONL 模板已保存。', 2200);
    }
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('保存模板文件失败', error));
  }
};

const templateCards = [
  {
    format: ModelTrainingAnnotationFormat.Jsonl,
    title: MODEL_TRAINING_ANNOTATION_FORMAT_TEXT[ModelTrainingAnnotationFormat.Jsonl],
    description: '每行一个 JSON 对象，适合脚本批量处理或版本管理。',
    hint: '{"audio": "speaker_001.wav", "text": "这里填写台词"}',
    icon: DocumentTextIcon
  },
  {
    format: ModelTrainingAnnotationFormat.Xlsx,
    title: MODEL_TRAINING_ANNOTATION_FORMAT_TEXT[ModelTrainingAnnotationFormat.Xlsx],
    description: '首列填写文件名，第二列填写台词，适合直接用 Excel 编辑。',
    hint: '第一列 文件名 / 第二列 台词',
    icon: TableCellsIcon
  }
] as const;
</script>

<template>
  <BaseDialog
    :open="open"
    title="下载数据标注模板"
    panel-class="max-w-2xl sm:max-w-3xl"
    content-class="overflow-visible"
    z-class="z-[160]"
    @close="emit('close')"
  >
    <div class="space-y-4">
      <p class="text-sm leading-6 text-stone-600">
        选择一种模板格式下载。JSONL 与 Excel 模板都使用相同的数据结构，Excel 导入时会读取第一列文件名与第二列台词。
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

          <BaseButton class="mt-4 min-h-[42px]" block @click="downloadTemplate(card.format)">
            <DocumentArrowDownIcon class="h-4 w-4" aria-hidden="true" />
            <span>{{ card.format === ModelTrainingAnnotationFormat.Xlsx ? '下载Excel模板' : `下载 ${card.title} 模板` }}</span>
          </BaseButton>
        </article>
      </div>

      <div class="rounded-2xl border border-brand-200 bg-white/90 p-4 text-xs leading-5 text-stone-500">
        <p>Excel 导入支持 .xlsx 与 .xls。</p>
        <p class="mt-1">如果压缩包中的音频是 OGG，后端会在训练前自动转成 WAV 后再交给模型处理。</p>
      </div>
    </div>

    <template #footer>
      <BaseButton tone="ghost" @click="emit('close')">关闭</BaseButton>
    </template>
  </BaseDialog>
</template>

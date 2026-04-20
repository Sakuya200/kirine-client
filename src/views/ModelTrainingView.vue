<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue';
import {
  ArrowDownTrayIcon,
  ArrowPathIcon,
  ArrowUpTrayIcon,
  CheckCircleIcon,
  CpuChipIcon,
  StopCircleIcon,
  TrashIcon,
  ArchiveBoxArrowDownIcon
} from '@heroicons/vue/24/outline';
import { useRoute, useRouter } from 'vue-router';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseLoadingBanner from '@/components/common/BaseLoadingBanner.vue';
import BaseLoadingIndicator from '@/components/common/BaseLoadingIndicator.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import RecentTaskList, { type RecentTaskListItem } from '@/components/common/RecentTaskList.vue';
import ModelTrainingTemplateDownloadDialog from '@/components/form/ModelTrainingTemplateDownloadDialog.vue';
import Qwen3TtsTrainingParamsForm from '@/components/qwen3_tts/Qwen3TtsTrainingParamsForm.vue';
import VoxCpm2TrainingParamsForm from '@/components/vox_cpm2/VoxCpm2TrainingParamsForm.vue';
import { AppLanguage } from '@/enums/language';
import {
  MODEL_TRAINING_ANNOTATION_FILE_EXTENSIONS,
  MODEL_TRAINING_ANNOTATION_FORMAT_TEXT,
  MODEL_TRAINING_AUDIO_FILE_EXTENSIONS,
  MODEL_TRAINING_LANGUAGE_OPTIONS,
  MODEL_TRAINING_SAMPLE_TYPE_TEXT,
  ModelTrainingAnnotationFormat,
  ModelTrainingSampleType,
  type ModelTrainingOption
} from '@/enums/modelTraining';
import { TaskStatus } from '@/enums/status';
import { getHistoryTaskReplayId, HISTORY_TASK_REPLAY_QUERY_KEY, HistoryTaskType } from '@/enums/task';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useModelStore } from '@/stores/models';
import { useUiStore } from '@/stores/ui';
import type { HistoryRecord, ModelTrainingHistoryRecord, ModelTrainingSampleDetail } from '@/types/domain';

type LocalFileKind = 'audio' | 'archive' | 'annotation';

interface SelectedLocalFile {
  fileName: string;
  filePath: string;
  fileKind: LocalFileKind;
}

interface ImportedSampleItem {
  id: number;
  type: ModelTrainingSampleType;
  title: string;
  detail: string;
  transcriptPreview?: string;
  primaryFile: SelectedLocalFile;
  secondaryFile?: SelectedLocalFile;
}

interface ModelTrainingTaskResultPayload {
  taskId: number;
  baseModel: string;
  modelScale: string;
  modelName: string;
  modelParams: Record<string, unknown>;
  sampleCount: number;
  createTime: string;
  status: TaskStatus;
}

const VOX_CPM2_BASE_MODEL = 'vox_cpm2';
const LORA_FEATURE = 'lora';

const createQwen3TrainingParams = () => ({
  epochCount: 30,
  batchSize: 8,
  gradientAccumulationSteps: 4,
  enableGradientCheckpointing: false
});

const createVoxCpm2TrainingParams = () => ({
  trainingMode: 'lora',
  useLora: true,
  loraRank: 32,
  loraAlpha: 32,
  loraDropout: '0.0',
  epochCount: 2,
  batchSize: 4,
  gradientAccumulationSteps: 1,
  enableGradientCheckpointing: false
});

const normalizeVoxCpm2TrainingParams = (modelParams: Record<string, unknown>) => {
  const defaults = createVoxCpm2TrainingParams();
  const legacyTrainingMode = String(modelParams.trainingMode ?? '').trim();
  const useLora = typeof modelParams.useLora === 'boolean' ? modelParams.useLora : legacyTrainingMode !== 'full';

  return {
    ...defaults,
    ...modelParams,
    useLora,
    trainingMode: useLora ? 'lora' : 'full',
    loraRank: Number(modelParams.loraRank ?? defaults.loraRank),
    loraAlpha: Number(modelParams.loraAlpha ?? defaults.loraAlpha),
    loraDropout: String(modelParams.loraDropout ?? defaults.loraDropout).trim() || defaults.loraDropout
  };
};

const normalizeTrainingModelParams = (baseModel: string, modelParams: Record<string, unknown>) =>
  baseModel === VOX_CPM2_BASE_MODEL ? normalizeVoxCpm2TrainingParams(modelParams) : { ...createQwen3TrainingParams(), ...modelParams };

const form = reactive({
  language: AppLanguage.Chinese,
  baseModel: '',
  modelScale: '',
  modelName: 'speaker_a_custom',
  modelParams: createQwen3TrainingParams() as Record<string, unknown>,
  singleAudioFile: null as SelectedLocalFile | null,
  singleTranscript: '',
  datasetArchiveFile: null as SelectedLocalFile | null,
  datasetAnnotationFile: null as SelectedLocalFile | null
});
const selectedLanguageOption = ref<ModelTrainingOption | null>(MODEL_TRAINING_LANGUAGE_OPTIONS[0]);
const isStarting = ref(false);
const isCancelling = ref(false);
const isRefreshingHistory = ref(false);
const activeTrainingTask = ref<ModelTrainingTaskResultPayload | null>(null);
const recentTrainingHistory = ref<ModelTrainingHistoryRecord[]>([]);
const isTemplateDialogOpen = ref(false);
const modelStore = useModelStore();
const uiStore = useUiStore();
const route = useRoute();
const router = useRouter();
let importedSampleIdSeed = Date.now();
let activeTaskStatusTimer: ReturnType<typeof setInterval> | null = null;
let isActiveTaskRefreshInFlight = false;
let isHistoryRefreshInFlight = false;

const trainingChecklist = [
  '准备最少 1 条音频样本与对应文本稿，建议 24kHz 以上质量。',
  '确保样本语言一致，避免中途切换语言导致风格漂移。',
  '上传前先裁剪静音段，控制单条样本长度在 15 秒以内。'
];

const importedSamples = ref<ImportedSampleItem[]>([]);
const modelOptions = computed(() =>
  modelStore.getModelsByFeature(HistoryTaskType.ModelTraining).map(item => ({
    label: item.modelName,
    value: item.baseModel
  }))
);
const modelScaleOptions = computed(() => modelStore.getModelScaleOptions(form.baseModel));
const isVoxCpm2Model = computed(() => form.baseModel === VOX_CPM2_BASE_MODEL);
const supportsSelectedModelLora = computed(() => modelStore.supportsModelFeature(form.baseModel, form.modelScale, LORA_FEATURE));
const activeTrainingParamsComponent = computed(() => (isVoxCpm2Model.value ? VoxCpm2TrainingParamsForm : Qwen3TtsTrainingParamsForm));

const singleImportReady = computed(() => Boolean(form.singleAudioFile) && form.singleTranscript.trim().length > 0);
const batchImportReady = computed(() => Boolean(form.datasetArchiveFile) && Boolean(form.datasetAnnotationFile));
const canStartTraining = computed(() => {
  const epochCount = Number(form.modelParams.epochCount ?? 0);
  const batchSize = Number(form.modelParams.batchSize ?? 0);
  const gradientAccumulationSteps = Number(form.modelParams.gradientAccumulationSteps ?? 0);

  return (
    form.modelName.trim().length > 0 &&
    importedSamples.value.length > 0 &&
    epochCount > 0 &&
    batchSize > 0 &&
    gradientAccumulationSteps > 0 &&
    !isStarting.value &&
    !!form.modelScale
  );
});

const sampleSummary = computed(() => ({
  total: importedSamples.value.length
}));
const recentTaskItems = computed<RecentTaskListItem[]>(() =>
  recentTrainingHistory.value.map(item => ({
    taskId: item.id,
    title: item.detail.modelName,
    subtitle: `任务 ${item.id} · ${modelStore.getModelLabel(item.detail.baseModel)} ${item.detail.modelScale}`,
    status: item.status
  }))
);

const baseModelSummary = computed(() => {
  return '训练任务会使用设置页中的全局硬件类型；若切换硬件，请先前往设置页保存。';
});
const trainingBusyLabel = computed(() => {
  if (isStarting.value) {
    return '正在创建模型训练任务，请稍候';
  }

  if (isCancelling.value) {
    return '正在请求终止模型训练任务，请稍候';
  }

  if (activeTrainingTask.value?.status === TaskStatus.Pending || activeTrainingTask.value?.status === TaskStatus.Running) {
    return '任务执行中，页面会持续刷新状态';
  }

  return '';
});

watch(
  modelOptions,
  options => {
    if (options.length === 0) {
      return;
    }

    if (!options.some(option => option.value === form.baseModel)) {
      form.baseModel = String(options[0]?.value ?? '');
    }
  },
  { immediate: true }
);

watch(
  modelScaleOptions,
  options => {
    if (options.length === 0) {
      form.modelScale = '';
      return;
    }

    if (!options.some(option => option.value === form.modelScale)) {
      form.modelScale = String(options[0]?.value ?? '');
    }
  },
  { immediate: true }
);

watch(
  () => form.baseModel,
  nextBaseModel => {
    form.modelParams = normalizeTrainingModelParams(nextBaseModel, form.modelParams);
  },
  { immediate: true }
);

const extractFileName = (filePath: string) => {
  const parts = filePath.split(/[/\\]/);
  return parts[parts.length - 1] ?? filePath;
};

const nextImportedSampleId = () => {
  importedSampleIdSeed += 1;
  return importedSampleIdSeed;
};

const stopActiveTaskStatusRefresh = () => {
  if (activeTaskStatusTimer) {
    clearInterval(activeTaskStatusTimer);
    activeTaskStatusTimer = null;
  }
};

const syncActiveTaskStatusRefresh = () => {
  stopActiveTaskStatusRefresh();

  if (
    !activeTrainingTask.value ||
    activeTrainingTask.value.status === TaskStatus.Completed ||
    activeTrainingTask.value.status === TaskStatus.Cancelled ||
    activeTrainingTask.value.status === TaskStatus.Failed
  ) {
    return;
  }

  activeTaskStatusTimer = setInterval(() => {
    void refreshActiveTaskStatus();
  }, 3000);
};

const mapHistoryRecordToTrainingTask = (record: HistoryRecord): ModelTrainingTaskResultPayload | null => {
  if (record.taskType !== HistoryTaskType.ModelTraining) {
    return null;
  }

  const trainingRecord = record as ModelTrainingHistoryRecord;
  return {
    taskId: trainingRecord.id,
    baseModel: trainingRecord.detail.baseModel,
    modelScale: trainingRecord.detail.modelScale,
    modelName: trainingRecord.detail.modelName,
    modelParams: trainingRecord.detail.modelParams,
    sampleCount: trainingRecord.detail.sampleCount,
    createTime: trainingRecord.createTime,
    status: trainingRecord.status
  };
};

const isModelTrainingHistoryRecord = (record: HistoryRecord): record is ModelTrainingHistoryRecord =>
  record.taskType === HistoryTaskType.ModelTraining;

const clearReplayTaskId = async () => {
  if (!(HISTORY_TASK_REPLAY_QUERY_KEY in route.query)) {
    return;
  }

  const nextQuery = { ...route.query };
  delete nextQuery[HISTORY_TASK_REPLAY_QUERY_KEY];
  await router.replace({ path: route.path, query: nextQuery });
};

const selectLocalFile = async (title: string, extensions: string[], fileKind: LocalFileKind) => {
  try {
    const selected = await open({
      title,
      multiple: false,
      directory: false,
      filters: [{ name: title, extensions }]
    });

    if (typeof selected !== 'string') {
      return null;
    }

    return {
      fileName: extractFileName(selected),
      filePath: selected,
      fileKind
    } satisfies SelectedLocalFile;
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('打开文件选择器失败', error));
    return null;
  }
};

const chooseSingleAudio = async () => {
  form.singleAudioFile = await selectLocalFile('选择音频文件', [...MODEL_TRAINING_AUDIO_FILE_EXTENSIONS], 'audio');
};

const chooseDatasetArchive = async () => {
  form.datasetArchiveFile = await selectLocalFile('选择 ZIP 压缩包', ['zip'], 'archive');
};

const chooseDatasetAnnotation = async () => {
  form.datasetAnnotationFile = await selectLocalFile('选择数据标注文件', [...MODEL_TRAINING_ANNOTATION_FILE_EXTENSIONS], 'annotation');
};

const addSingleSample = () => {
  if (!singleImportReady.value || !form.singleAudioFile) {
    return;
  }

  importedSamples.value.unshift({
    id: nextImportedSampleId(),
    type: ModelTrainingSampleType.Single,
    title: form.singleAudioFile.fileName,
    detail: `音频文件 · ${form.singleAudioFile.filePath}`,
    transcriptPreview: form.singleTranscript.trim(),
    primaryFile: form.singleAudioFile
  });

  form.singleAudioFile = null;
  form.singleTranscript = '';
  uiStore.notifySuccess('已将单样本加入导入列表，可继续添加更多数据。', 2600);
};

const addDatasetSample = () => {
  if (!batchImportReady.value || !form.datasetArchiveFile || !form.datasetAnnotationFile) {
    return;
  }

  importedSamples.value.unshift({
    id: nextImportedSampleId(),
    type: ModelTrainingSampleType.Dataset,
    title: form.datasetArchiveFile.fileName,
    detail: `ZIP 压缩包 + 标注文件 · ${form.datasetArchiveFile.filePath}`,
    primaryFile: form.datasetArchiveFile,
    secondaryFile: form.datasetAnnotationFile
  });

  form.datasetArchiveFile = null;
  form.datasetAnnotationFile = null;
  uiStore.notifySuccess('已将样本集加入导入列表，训练时会按数据集方式处理。', 2600);
};

const removeImportedSample = (sampleId: number) => {
  importedSamples.value = importedSamples.value.filter(sample => sample.id !== sampleId);
};

const resetForm = () => {
  form.language = AppLanguage.Chinese;
  form.baseModel = String(modelOptions.value[0]?.value ?? '');
  form.modelScale = String(modelScaleOptions.value[0]?.value ?? '');
  form.modelName = 'speaker_a_custom';
  form.modelParams = normalizeTrainingModelParams(form.baseModel, {});
  form.singleAudioFile = null;
  form.singleTranscript = '';
  form.datasetArchiveFile = null;
  form.datasetAnnotationFile = null;
  selectedLanguageOption.value = MODEL_TRAINING_LANGUAGE_OPTIONS[0];
  importedSamples.value = [];
  uiStore.notifyInfo('训练表单已重置。', 2200);
};

const mapHistorySampleToImportedSample = (sample: ModelTrainingSampleDetail): ImportedSampleItem => ({
  id: nextImportedSampleId(),
  type: sample.sampleType,
  title: sample.title,
  detail: sample.detail,
  transcriptPreview: sample.transcriptPreview ?? undefined,
  primaryFile: {
    fileName: sample.primaryFile.fileName,
    filePath: sample.primaryFile.filePath,
    fileKind: sample.primaryFile.fileKind
  },
  secondaryFile: sample.secondaryFile
    ? {
        fileName: sample.secondaryFile.fileName,
        filePath: sample.secondaryFile.filePath,
        fileKind: sample.secondaryFile.fileKind
      }
    : undefined
});

const applyTrainingHistoryToForm = (record: ModelTrainingHistoryRecord) => {
  form.language = record.detail.language;
  form.baseModel = record.detail.baseModel;
  form.modelScale = record.detail.modelScale;
  form.modelName = record.detail.modelName;
  form.modelParams = normalizeTrainingModelParams(record.detail.baseModel, { ...record.detail.modelParams });
  form.singleAudioFile = null;
  form.singleTranscript = '';
  form.datasetArchiveFile = null;
  form.datasetAnnotationFile = null;
  importedSamples.value = record.detail.samples.map(mapHistorySampleToImportedSample);
  selectedLanguageOption.value = MODEL_TRAINING_LANGUAGE_OPTIONS.find(option => option.value === form.language) ?? null;
};

const hydrateReplayTaskFromRoute = async () => {
  const historyId = getHistoryTaskReplayId(route.query[HISTORY_TASK_REPLAY_QUERY_KEY]);

  if (historyId === null) {
    await clearReplayTaskId();
    return;
  }

  try {
    const record = await invoke<HistoryRecord>('get_history_record', { historyId });

    if (record.taskType !== HistoryTaskType.ModelTraining) {
      uiStore.notifyWarning('目标历史任务与当前页面类型不匹配，无法载入配置。');
      return;
    }

    applyTrainingHistoryToForm(record as ModelTrainingHistoryRecord);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('载入历史任务配置失败，请检查任务记录是否仍然存在', error));
  } finally {
    await clearReplayTaskId();
  }
};

const loadRecentTasks = async ({ notifyOnSuccess = false, silentOnError = false, manual = false } = {}) => {
  if (isHistoryRefreshInFlight) {
    return;
  }

  isHistoryRefreshInFlight = true;
  if (manual) {
    isRefreshingHistory.value = true;
  }

  try {
    const records = await invoke<HistoryRecord[]>('list_history_records');
    recentTrainingHistory.value = records.filter(isModelTrainingHistoryRecord).slice(0, 5);

    if (notifyOnSuccess) {
      uiStore.notifySuccess('模型训练任务状态已刷新。', 2200);
    }
  } catch (error) {
    recentTrainingHistory.value = [];
    if (!silentOnError) {
      uiStore.notifyError(formatErrorMessage('刷新模型训练历史任务失败，请检查 Rust 后端日志', error));
    }
  } finally {
    isHistoryRefreshInFlight = false;
    if (manual) {
      isRefreshingHistory.value = false;
    }
  }
};

const refreshActiveTaskStatus = async () => {
  if (
    !activeTrainingTask.value ||
    activeTrainingTask.value.status === TaskStatus.Completed ||
    activeTrainingTask.value.status === TaskStatus.Cancelled ||
    activeTrainingTask.value.status === TaskStatus.Failed
  ) {
    stopActiveTaskStatusRefresh();
    return;
  }

  if (isActiveTaskRefreshInFlight) {
    return;
  }

  isActiveTaskRefreshInFlight = true;
  const currentTaskId = activeTrainingTask.value.taskId;

  try {
    const record = await invoke<HistoryRecord>('get_history_record', { historyId: currentTaskId });
    const updatedTask = mapHistoryRecordToTrainingTask(record);

    if (!updatedTask || updatedTask.taskId !== currentTaskId) {
      return;
    }

    activeTrainingTask.value = updatedTask;
    recentTrainingHistory.value = recentTrainingHistory.value.map(item =>
      item.id === currentTaskId ? ({ ...item, status: updatedTask.status } as ModelTrainingHistoryRecord) : item
    );

    if (updatedTask.status === TaskStatus.Completed || updatedTask.status === TaskStatus.Cancelled || updatedTask.status === TaskStatus.Failed) {
      stopActiveTaskStatusRefresh();
    }
  } catch (error) {
    console.log(formatErrorMessage('刷新模型训练任务状态失败，请检查 Rust 后端日志', error));
  } finally {
    isActiveTaskRefreshInFlight = false;
  }
};

const cancelActiveTrainingTask = async () => {
  if (!activeTrainingTask.value || ![TaskStatus.Pending, TaskStatus.Running].includes(activeTrainingTask.value.status)) {
    return;
  }

  isCancelling.value = true;

  try {
    const accepted = await invoke<boolean>('cancel_model_training_task', {
      historyId: activeTrainingTask.value.taskId
    });

    if (!accepted) {
      uiStore.notifyWarning('当前训练任务已经提交过终止请求。');
      return;
    }

    uiStore.notifyInfo(`已发送终止请求，任务 ${activeTrainingTask.value.taskId} 会在后端停止后刷新状态。`, 3600);
    await refreshActiveTaskStatus();
    await loadRecentTasks({ silentOnError: true });
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('终止模型训练任务失败', error));
  } finally {
    isCancelling.value = false;
  }
};

const startTraining = async () => {
  if (!canStartTraining.value) {
    return;
  }

  isStarting.value = true;
  uiStore.notifyInfo('正在创建模型训练任务。', 2200);

  try {
    const payload = await invoke<ModelTrainingTaskResultPayload>('create_model_training_task', {
      payload: {
        language: form.language,
        baseModel: form.baseModel,
        modelScale: form.modelScale,
        modelName: form.modelName.trim(),
        modelParams: form.modelParams,
        samples: importedSamples.value.map(sample => ({
          id: sample.id,
          sampleType: sample.type,
          title: sample.title,
          detail: sample.detail,
          transcriptPreview: sample.transcriptPreview,
          primaryFile: sample.primaryFile,
          secondaryFile: sample.secondaryFile ?? null
        }))
      }
    });

    activeTrainingTask.value = payload;
    syncActiveTaskStatusRefresh();
    await loadRecentTasks({ silentOnError: true });

    uiStore.notifySuccess(
      `模型训练任务已创建：${payload.modelName}，任务 ID ${payload.taskId}，基础模型 ${modelStore.getModelLabel(payload.baseModel)} ${payload.modelScale}，共 ${payload.sampleCount} 项样本。`,
      5200
    );
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('模型训练任务创建失败', error));
  } finally {
    isStarting.value = false;
  }
};

const loadHistoryItem = (taskId: number) => {
  const item = recentTrainingHistory.value.find(historyItem => historyItem.id === taskId);
  if (!item) {
    uiStore.notifyWarning('目标历史任务不存在或已被移除。');
    return;
  }

  applyTrainingHistoryToForm(item);
  activeTrainingTask.value = mapHistoryRecordToTrainingTask(item);
  syncActiveTaskStatusRefresh();
};

onMounted(async () => {
  await modelStore.ensureLoaded();
  await loadRecentTasks({ silentOnError: true });
  await hydrateReplayTaskFromRoute();
});

onBeforeUnmount(() => {
  stopActiveTaskStatusRefresh();
});
</script>

<template>
  <div class="space-y-5">
    <PageHeader title="模型训练" description="上传训练样本与标注内容，创建可用于文本转语音的本地说话人模型。" eyebrow="Model-Training" />

    <BaseLoadingBanner v-if="trainingBusyLabel" :label="trainingBusyLabel" />

    <div class="grid gap-5 xl:grid-cols-[1.2fr_1fr]">
      <PanelCard title="训练数据导入" subtitle="支持逐条导入和按数据集导入，两种方式都需要音频与文本对齐">
        <div class="grid gap-4 xl:grid-cols-2">
          <section class="flex h-full flex-col rounded-2xl border border-brand-200 bg-white/80 p-4">
            <div class="mb-3">
              <span class="rounded-full bg-brand-100 px-2.5 py-1 text-xs font-medium text-brand-700">单样本</span>
            </div>
            <div class="min-h-[68px]">
              <p class="text-sm font-semibold text-slate-800">一对一上传</p>
              <p class="mt-1 text-xs leading-5 text-stone-500">提供一段音频和对应台词，导入后记为单样本。</p>
            </div>

            <label class="mt-4 block text-sm text-slate-700">
              <span class="mb-1 block text-xs text-stone-500">音频文件</span>
              <div class="flex min-h-[120px] flex-col justify-between rounded-2xl border border-dashed border-brand-300 bg-brand-50/50 p-4">
                <BaseButton tone="ghost" @click="chooseSingleAudio">
                  <ArrowUpTrayIcon class="h-4 w-4" aria-hidden="true" />
                  <span>选择本地音频</span>
                </BaseButton>
                <p class="mt-2 text-xs text-stone-500">{{ form.singleAudioFile?.fileName ?? '尚未选择音频文件' }}</p>
                <p v-if="form.singleAudioFile" class="mt-1 break-all text-[11px] text-stone-400">{{ form.singleAudioFile.filePath }}</p>
              </div>
            </label>

            <label class="mt-4 block text-sm text-slate-700">
              <span class="mb-1 block text-xs text-stone-500">台词文本</span>
              <textarea
                v-model="form.singleTranscript"
                rows="5"
                class="min-h-[120px] w-full rounded-2xl border border-brand-200 bg-brand-50/35 px-3 py-2 text-sm text-slate-700"
                placeholder="请输入与音频严格对应的台词文本..."
              />
            </label>

            <div class="mt-auto pt-4">
              <BaseButton block :disabled="!singleImportReady" @click="addSingleSample">
                <ArrowUpTrayIcon class="h-4 w-4" aria-hidden="true" />
                <span>加入导入列表</span>
              </BaseButton>
            </div>
          </section>

          <section class="flex h-full flex-col rounded-2xl border border-brand-200 bg-white/80 p-4">
            <div class="mb-3">
              <span class="rounded-full bg-amber-100 px-2.5 py-1 text-xs font-medium text-amber-700">样本集</span>
            </div>
            <div class="min-h-[68px]">
              <p class="text-sm font-semibold text-slate-800">批量上传</p>
              <p class="mt-1 text-xs leading-5 text-stone-500">提供音频压缩包与数据标注文件，导入后记为样本集。</p>
            </div>

            <label class="mt-4 block text-sm text-slate-700">
              <span class="mb-1 block text-xs text-stone-500">音频压缩包</span>
              <div class="flex min-h-[120px] flex-col justify-between rounded-2xl border border-dashed border-brand-300 bg-brand-50/50 p-4">
                <BaseButton tone="ghost" @click="chooseDatasetArchive">
                  <ArchiveBoxArrowDownIcon class="h-4 w-4" aria-hidden="true" />
                  <span>选择 ZIP 压缩包</span>
                </BaseButton>
                <p class="mt-2 text-xs text-stone-500">{{ form.datasetArchiveFile?.fileName ?? '尚未选择 ZIP 压缩包' }}</p>
                <p v-if="form.datasetArchiveFile" class="mt-1 break-all text-[11px] text-stone-400">{{ form.datasetArchiveFile.filePath }}</p>
              </div>
            </label>

            <label class="mt-4 block text-sm text-slate-700">
              <div class="mb-1 flex items-center justify-between gap-3 text-xs text-stone-500">
                <span>数据标注文件</span>
                <BaseButton tone="quiet" size="sm" @click="isTemplateDialogOpen = true">
                  <ArrowDownTrayIcon class="h-4 w-4" aria-hidden="true" />
                  <span>下载模板</span>
                </BaseButton>
              </div>
              <div class="flex min-h-[120px] flex-col justify-between rounded-2xl border border-dashed border-brand-300 bg-brand-50/50 p-4">
                <BaseButton tone="ghost" @click="chooseDatasetAnnotation">
                  <ArrowUpTrayIcon class="h-4 w-4" aria-hidden="true" />
                  <span>选择数据标注文件</span>
                </BaseButton>
                <p class="mt-2 text-xs text-stone-500">{{ form.datasetAnnotationFile?.fileName ?? '尚未选择标注文件' }}</p>
                <p v-if="form.datasetAnnotationFile" class="mt-1 break-all text-[11px] text-stone-400">{{ form.datasetAnnotationFile.filePath }}</p>
                <p class="mt-2 text-[11px] text-stone-400">
                  支持 {{ MODEL_TRAINING_ANNOTATION_FORMAT_TEXT[ModelTrainingAnnotationFormat.Jsonl] }}、
                  {{ MODEL_TRAINING_ANNOTATION_FORMAT_TEXT[ModelTrainingAnnotationFormat.Xlsx] }} 和
                  {{ MODEL_TRAINING_ANNOTATION_FORMAT_TEXT[ModelTrainingAnnotationFormat.Xls] }}。
                </p>
              </div>
            </label>

            <div class="mt-auto pt-4">
              <BaseButton block :disabled="!batchImportReady" @click="addDatasetSample">
                <ArchiveBoxArrowDownIcon class="h-4 w-4" aria-hidden="true" />
                <span>加入导入列表</span>
              </BaseButton>
            </div>
          </section>
        </div>

        <div class="mt-4 rounded-2xl border border-brand-200 bg-brand-50/40 p-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p class="text-sm font-semibold text-slate-800">导入列表</p>
              <p class="mt-1 text-xs text-stone-500">已导入 {{ sampleSummary.total }} 项样本。</p>
            </div>
          </div>

          <ul v-if="importedSamples.length > 0" class="mt-4 space-y-3">
            <li v-for="sample in importedSamples" :key="sample.id" class="rounded-xl border border-brand-200 bg-white/90 p-3">
              <div class="flex items-start justify-between gap-3">
                <div>
                  <div class="flex items-center gap-2">
                    <p class="text-sm font-semibold text-slate-800">{{ sample.title }}</p>
                    <span class="rounded-full border border-brand-200 bg-brand-50 px-2 py-0.5 text-[11px] text-brand-700">
                      {{ MODEL_TRAINING_SAMPLE_TYPE_TEXT[sample.type] }}
                    </span>
                  </div>
                  <p class="mt-1 text-xs text-stone-500">{{ sample.detail }}</p>
                  <p v-if="sample.transcriptPreview" class="mt-2 text-xs text-slate-600">{{ sample.transcriptPreview }}</p>
                </div>
                <BaseButton tone="quiet" size="sm" @click="removeImportedSample(sample.id)">
                  <TrashIcon class="h-4 w-4" aria-hidden="true" />
                  <span>移除</span>
                </BaseButton>
              </div>
            </li>
          </ul>

          <div v-else class="mt-4 rounded-xl border border-dashed border-brand-200 bg-white/80 p-4 text-xs text-stone-500">
            还没有导入任何样本。单样本和样本集都可以加入训练列表。
          </div>
        </div>
      </PanelCard>

      <div class="space-y-5">
        <PanelCard class="z-20" title="基础参数">
          <div class="space-y-3 text-sm text-slate-700">
            <label class="block">
              <span class="mb-1 block text-xs text-stone-500">训练输出名称</span>
              <input v-model="form.modelName" class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" placeholder="请输入模型名称" />
            </label>
            <BaseListbox v-model="form.baseModel" label="基础模型" :options="modelOptions" />
            <BaseListbox v-model="form.modelScale" label="模型大小" :options="modelScaleOptions" :disabled="modelScaleOptions.length === 0" />
            <BaseListbox
              v-model="form.language"
              v-model:selected-option="selectedLanguageOption"
              label="语种"
              :options="MODEL_TRAINING_LANGUAGE_OPTIONS"
            />
            <div>
              <p class="text-base font-semibold tracking-tight text-slate-900">模型特定参数</p>
              <component :is="activeTrainingParamsComponent" class="mt-4" v-model="form.modelParams" :supports-lora="supportsSelectedModelLora" />
            </div>
            <div class="rounded-2xl border border-brand-200 bg-white/80 p-3 text-xs text-stone-600">
              <p>训练摘要</p>
              <p class="mt-1">当前将使用 {{ sampleSummary.total }} 项导入数据，语言 {{ selectedLanguageOption?.label ?? '未选择' }}。</p>
              <p class="mt-1">基础模型 {{ modelStore.getModelLabel(form.baseModel) }} {{ form.modelScale }}。{{ baseModelSummary }}</p>
              <p v-if="isVoxCpm2Model" class="mt-1">当前微调模式 {{ form.modelParams.useLora ? 'LoRA 微调' : '全量微调' }}。</p>
              <p v-if="isVoxCpm2Model && form.modelParams.useLora" class="mt-1">
                LoRA 参数 rank {{ form.modelParams.loraRank ?? 32 }}，alpha {{ form.modelParams.loraAlpha ?? 32 }}，dropout
                {{ form.modelParams.loraDropout ?? 0 }}。
              </p>
              <p class="mt-1">建议批次大小根据显存调整，样本较少时可先从 4 到 8 开始。</p>
              <p class="mt-1">
                当前梯度累积 {{ form.modelParams.gradientAccumulationSteps ?? 0 }}，梯度检查点
                {{ form.modelParams.enableGradientCheckpointing ? '已启用' : '未启用' }}。
              </p>
            </div>
          </div>
          <div class="mt-4 flex flex-wrap gap-2">
            <BaseButton :disabled="!canStartTraining" @click="startTraining">
              <BaseLoadingIndicator v-if="isStarting" size="sm" tone="muted" />
              <CpuChipIcon v-else class="h-4 w-4" aria-hidden="true" />
              <span>{{ isStarting ? '创建中...' : '开始训练' }}</span>
            </BaseButton>
            <BaseButton
              v-if="activeTrainingTask && [TaskStatus.Pending, TaskStatus.Running].includes(activeTrainingTask.status)"
              tone="quiet"
              :disabled="isCancelling"
              @click="cancelActiveTrainingTask"
            >
              <BaseLoadingIndicator v-if="isCancelling" size="sm" tone="muted" />
              <StopCircleIcon v-else class="h-4 w-4" aria-hidden="true" />
              <span>{{ isCancelling ? '终止中...' : '终止当前任务' }}</span>
            </BaseButton>
            <BaseButton tone="ghost" @click="resetForm">
              <ArrowPathIcon class="h-4 w-4" aria-hidden="true" />
              <span>重置表单</span>
            </BaseButton>
          </div>
          <div v-if="activeTrainingTask" class="mt-4 rounded-2xl border border-brand-200 bg-brand-50/40 p-3 text-xs text-stone-600">
            <p>当前活动任务</p>
            <p class="mt-1">
              任务 {{ activeTrainingTask.taskId }}，状态 {{ activeTrainingTask.status }}，模型
              {{ modelStore.getModelLabel(activeTrainingTask.baseModel) }} {{ activeTrainingTask.modelScale }}。
            </p>
            <p class="mt-1">创建时间 {{ activeTrainingTask.createTime }}，共 {{ activeTrainingTask.sampleCount }} 项导入样本。</p>
          </div>
        </PanelCard>

        <PanelCard class="z-0" title="准备清单" subtitle="正式接入训练脚本前，先确保数据质量">
          <ul class="space-y-2 text-sm text-slate-700">
            <li v-for="item in trainingChecklist" :key="item" class="flex gap-2">
              <CheckCircleIcon class="mt-0.5 h-4 w-4 shrink-0 text-brand-500" aria-hidden="true" />
              <span>{{ item }}</span>
            </li>
          </ul>
        </PanelCard>

        <PanelCard class="z-0" title="最近任务" subtitle="展示最近 5 条模型训练任务，数据来自统一历史记录">
          <template #actions>
            <BaseButton tone="ghost" size="sm" :disabled="isRefreshingHistory" @click="loadRecentTasks({ notifyOnSuccess: true, manual: true })">
              <BaseLoadingIndicator v-if="isRefreshingHistory" size="sm" tone="muted" />
              <ArrowPathIcon v-else class="h-4 w-4" aria-hidden="true" />
              <span>{{ isRefreshingHistory ? '刷新中...' : '刷新状态' }}</span>
            </BaseButton>
          </template>

          <RecentTaskList
            :items="recentTaskItems"
            :selected-task-id="activeTrainingTask?.taskId ?? null"
            empty-text="还没有历史任务。开始训练后会自动加入这里。"
            action-label="查看"
            @select="loadHistoryItem"
          />
        </PanelCard>
      </div>
    </div>

    <ModelTrainingTemplateDownloadDialog :open="isTemplateDialogOpen" @close="isTemplateDialogOpen = false" />
  </div>
</template>

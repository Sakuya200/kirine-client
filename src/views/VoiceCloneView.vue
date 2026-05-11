<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { ArrowPathIcon, SparklesIcon } from '@heroicons/vue/24/outline';
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseLoadingBanner from '@/components/common/BaseLoadingBanner.vue';
import GeneratedAudioResultCard from '@/components/common/GeneratedAudioResultCard.vue';
import BaseLoadingIndicator from '@/components/common/BaseLoadingIndicator.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import RecentTaskList, { type RecentTaskListItem } from '@/components/common/RecentTaskList.vue';
import GenericTaskParamsForm from '@/components/form/GenericTaskParamsForm.vue';
import { APP_LANGUAGE_LABELS, AppLanguage } from '@/enums/language';
import { MODEL_TRAINING_AUDIO_FILE_EXTENSIONS } from '@/enums/modelTraining';
import { TaskStatus } from '@/enums/status';
import { getHistoryTaskReplayId, HISTORY_TASK_REPLAY_QUERY_KEY, HistoryTaskType } from '@/enums/task';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { TEXT_TO_SPEECH_FORMATS, TextToSpeechFormat, type TextToSpeechOption } from '@/enums/textToSpeech';
import { useModelStore } from '@/stores/models';
import { useUiConfigStore } from '@/stores/uiConfig';
import { useUiStore } from '@/stores/ui';
import type { HistoryRecord } from '@/types/domain';
import { createTaskExportAudioName } from '@/utils/createTaskExportAudioName';
import { mergeModelParamsWithUiConfigDefaults } from '@/utils/uiConfigModelParams';

interface VoiceCloneResult {
  taskId: number;
  fileName: string;
  refAudioName: string;
  baseModel: string;
  modelScale: string;
  language: AppLanguage;
  languageLabel: string;
  format: TextToSpeechFormat;
  formatLabel: string;
  exportAudioName: string;
  durationSeconds: number;
  refText: string;
  text: string;
  modelParams: Record<string, unknown>;
  createdAt: string;
  status: TaskStatus;
  outputFilePath: string;
}

interface VoiceCloneTaskResultPayload {
  taskId: number;
  fileName: string;
  refAudioName: string;
  baseModel: string;
  modelScale: string;
  language: AppLanguage;
  format: TextToSpeechFormat;
  exportAudioName: string;
  refText: string;
  text: string;
  modelParams: Record<string, unknown>;
  durationSeconds: number;
  createdAt: string;
  status: TaskStatus;
  outputFilePath: string;
}

interface VoiceCloneAudioAssetPayload {
  taskId: number;
  fileName: string;
  contentType: string;
  bytes: number[];
}

interface SelectedAudioFile {
  fileName: string;
  filePath: string;
}

const DYNAMIC_REFERENCE_BASE_MODELS = new Set(['gpt_sovits_cpufast']);
const DEFAULT_EXPORT_AUDIO_NAME = createTaskExportAudioName(HistoryTaskType.VoiceClone);

const uiConfigStore = useUiConfigStore();

const normalizeVoiceCloneModelParams = (baseModel: string, modelParams: Record<string, unknown>) => {
  const taskConfig = uiConfigStore.getTaskConfig(baseModel, 'voice-clone');
  return mergeModelParamsWithUiConfigDefaults(taskConfig, modelParams);
};

const uiStore = useUiStore();
const modelStore = useModelStore();
const route = useRoute();
const router = useRouter();
const form = reactive({
  baseModel: '',
  modelScale: '',
  language: AppLanguage.Chinese,
  format: TextToSpeechFormat.Wav,
  exportAudioName: DEFAULT_EXPORT_AUDIO_NAME,
  refAudioFile: null as SelectedAudioFile | null,
  refText: '',
  text: '',
  modelParams: {} as Record<string, unknown>
});
const languageOptions = Object.values(AppLanguage).map(value => ({
  label: APP_LANGUAGE_LABELS[value],
  value
}));
const formatOptions = TEXT_TO_SPEECH_FORMATS;
const selectedLanguageOption = ref<{ label: string; value: AppLanguage } | null>(languageOptions[0] ?? null);
const selectedFormatOption = ref<TextToSpeechOption | null>(formatOptions[0] ?? null);
const isGenerating = ref(false);
const isRefreshingHistory = ref(false);
const activeResult = ref<VoiceCloneResult | null>(null);
const generationHistory = ref<VoiceCloneResult[]>([]);
const selectedHistoryTaskId = ref<number | null>(null);
const resultCardRef = ref<InstanceType<typeof GeneratedAudioResultCard> | null>(null);

let activeTaskStatusTimer: ReturnType<typeof setInterval> | null = null;
let isActiveTaskRefreshInFlight = false;
let isHistoryRefreshInFlight = false;
let skipHistoryTaskSelectionReload = false;

const extractFileName = (filePath: string) => {
  const segments = filePath.split(/[/\\]/);
  return segments[segments.length - 1] ?? filePath;
};

const trimmedRefText = computed(() => form.refText.trim());
const trimmedText = computed(() => form.text.trim());
const charCount = computed(() => trimmedText.value.length);
const isDynamicReferenceModel = computed(() => DYNAMIC_REFERENCE_BASE_MODELS.has(form.baseModel));
const dynamicRefAudioPath = computed(() => String(form.modelParams.refAudioPath ?? '').trim());
const dynamicRefTextPath = computed(() => String(form.modelParams.refTextPath ?? '').trim());
const effectiveRefAudioPath = computed(() => (isDynamicReferenceModel.value ? dynamicRefAudioPath.value : (form.refAudioFile?.filePath ?? '')));
const effectiveRefAudioName = computed(() => {
  const path = effectiveRefAudioPath.value;
  return path ? extractFileName(path) : '';
});
const effectiveReferenceText = computed(() => (isDynamicReferenceModel.value ? dynamicRefTextPath.value : trimmedRefText.value));
const modelOptions = computed(() =>
  modelStore.getModelsByFeature(HistoryTaskType.VoiceClone).map(item => ({
    label: item.modelName,
    value: item.baseModel
  }))
);
const modelScaleOptions = computed(() => modelStore.getModelScaleOptions(form.baseModel));
const activeVoiceCloneTaskConfig = computed(() => uiConfigStore.getTaskConfig(form.baseModel, 'voice-clone'));
const requiresReferenceText = computed(() => true);
const canGenerate = computed(() => {
  const modelParamsValid = activeVoiceCloneTaskConfig.value
    ? uiConfigStore.validateModelParams(form.baseModel, 'voice-clone', form.modelParams)
    : true;

  return (
    Boolean(form.baseModel) &&
    Boolean(form.modelScale) &&
    Boolean(effectiveRefAudioPath.value) &&
    (!requiresReferenceText.value || Boolean(effectiveReferenceText.value)) &&
    modelParamsValid
  );
});
const cloneSummary = computed(() => [
  `当前模型为 ${modelStore.getModelLabel(form.baseModel)} ${form.modelScale}。`,
  `当前语言为 ${selectedLanguageOption.value?.label ?? APP_LANGUAGE_LABELS[form.language]}。`,
  `输出格式为 ${selectedFormatOption.value?.label ?? form.format}。`,
  `导出名称为 ${form.exportAudioName || DEFAULT_EXPORT_AUDIO_NAME}。`
]);
const activeResultMetaText = computed(() => {
  if (!activeResult.value) {
    return '';
  }

  return `${modelStore.getModelLabel(activeResult.value.baseModel)} · ${activeResult.value.modelScale} · ${activeResult.value.languageLabel} · ${activeResult.value.formatLabel}`;
});
const recentTaskItems = computed<RecentTaskListItem[]>(() =>
  generationHistory.value.map(item => ({
    taskId: item.taskId,
    title: item.refAudioName,
    subtitle: `任务 ${item.taskId} · ${item.languageLabel} · ${item.fileName}`,
    status: item.status
  }))
);
const activeTaskBusyLabel = computed(() => {
  if (isGenerating.value) {
    return '正在创建声音克隆任务，请稍候';
  }

  if (activeResult.value?.status === TaskStatus.Pending || activeResult.value?.status === TaskStatus.Running) {
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
    form.modelParams = normalizeVoiceCloneModelParams(nextBaseModel, form.modelParams);
  },
  { immediate: true }
);

const findLanguageLabel = (language: AppLanguage) => APP_LANGUAGE_LABELS[language] ?? language;
const findFormatLabel = (format: TextToSpeechFormat) => TEXT_TO_SPEECH_FORMATS.find(option => option.value === format)?.label ?? format;

const clearReplayTaskId = async () => {
  if (!(HISTORY_TASK_REPLAY_QUERY_KEY in route.query)) {
    return;
  }

  const nextQuery = { ...route.query };
  delete nextQuery[HISTORY_TASK_REPLAY_QUERY_KEY];
  await router.replace({ path: route.path, query: nextQuery });
};

const mapResultPayload = (payload: VoiceCloneTaskResultPayload): VoiceCloneResult => ({
  taskId: payload.taskId,
  fileName: payload.fileName,
  refAudioName: payload.refAudioName,
  baseModel: payload.baseModel,
  modelScale: payload.modelScale,
  language: payload.language,
  languageLabel: findLanguageLabel(payload.language),
  format: payload.format,
  formatLabel: findFormatLabel(payload.format),
  exportAudioName: payload.exportAudioName,
  durationSeconds: payload.durationSeconds,
  refText: payload.refText,
  text: payload.text,
  modelParams: payload.modelParams,
  createdAt: payload.createdAt,
  status: payload.status,
  outputFilePath: payload.outputFilePath
});

const mapHistoryRecordToResult = (record: HistoryRecord): VoiceCloneResult | null => {
  if (record.taskType !== HistoryTaskType.VoiceClone) {
    return null;
  }

  return {
    taskId: record.id,
    fileName: record.detail.fileName,
    refAudioName: record.detail.refAudioName,
    baseModel: record.detail.baseModel,
    modelScale: record.detail.modelScale,
    language: record.detail.language,
    languageLabel: findLanguageLabel(record.detail.language),
    format: record.detail.format,
    formatLabel: findFormatLabel(record.detail.format),
    exportAudioName: record.detail.exportAudioName,
    durationSeconds: record.durationSeconds,
    refText: record.detail.refText,
    text: record.detail.text,
    modelParams: record.detail.modelParams,
    createdAt: record.createTime,
    status: record.status,
    outputFilePath: record.detail.outputFilePath
  };
};

const applyReplayConfig = (result: VoiceCloneResult, refAudioPath: string, notifyMessage: string) => {
  stopActiveTaskStatusRefresh();
  activeResult.value = null;
  form.baseModel = result.baseModel;
  form.modelScale = result.modelScale;
  form.language = result.language;
  form.format = result.format;
  form.exportAudioName = result.exportAudioName;
  form.refAudioFile = {
    fileName: result.refAudioName,
    filePath: refAudioPath
  };
  form.refText = result.refText;
  form.text = result.text;
  form.modelParams = normalizeVoiceCloneModelParams(result.baseModel, { ...result.modelParams });
  selectedLanguageOption.value = languageOptions.find(option => option.value === result.language) ?? null;
  selectedFormatOption.value = formatOptions.find(option => option.value === result.format) ?? null;
  uiStore.notifyInfo(notifyMessage, 2800);
};

const applyHistoryTaskToForm = (result: VoiceCloneResult, refAudioPath: string, setAsActiveResult: boolean) => {
  stopActiveTaskStatusRefresh();
  activeResult.value = setAsActiveResult ? result : null;
  form.baseModel = result.baseModel;
  form.modelScale = result.modelScale;
  form.language = result.language;
  form.format = result.format;
  form.exportAudioName = result.exportAudioName;
  form.refAudioFile = {
    fileName: result.refAudioName,
    filePath: refAudioPath
  };
  form.refText = result.refText;
  form.text = result.text;
  form.modelParams = normalizeVoiceCloneModelParams(result.baseModel, { ...result.modelParams });
  selectedLanguageOption.value = languageOptions.find(option => option.value === result.language) ?? null;
  selectedFormatOption.value = formatOptions.find(option => option.value === result.format) ?? null;

  if (setAsActiveResult) {
    syncActiveTaskStatusRefresh();
  }
};

const setSelectedHistoryTaskId = (taskId: number | null, skipReload = false) => {
  if (selectedHistoryTaskId.value === taskId) {
    skipHistoryTaskSelectionReload = false;
    return;
  }

  skipHistoryTaskSelectionReload = skipReload;
  selectedHistoryTaskId.value = taskId;
};

const loadSelectedHistoryTask = async (taskId: number) => {
  try {
    const record = await invoke<HistoryRecord>('get_history_record', { historyId: taskId });

    if (record.taskType !== HistoryTaskType.VoiceClone) {
      uiStore.notifyWarning('目标历史任务与当前页面类型不匹配，无法载入配置。');
      return;
    }

    const result = mapHistoryRecordToResult(record);
    if (!result) {
      uiStore.notifyError('历史任务配置解析失败。');
      return;
    }

    applyHistoryTaskToForm(result, record.detail.refAudioPath, true);
    generationHistory.value = generationHistory.value.map(item => (item.taskId === result.taskId ? result : item));
    await resultCardRef.value?.refreshDetailRecord();
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('载入声音克隆历史任务配置失败，请检查后端日志', error));
  }
};

watch(selectedHistoryTaskId, taskId => {
  if (skipHistoryTaskSelectionReload) {
    skipHistoryTaskSelectionReload = false;
    return;
  }

  if (taskId === null) {
    return;
  }

  void loadSelectedHistoryTask(taskId);
});

const hydrateReplayTaskFromRoute = async () => {
  const historyId = getHistoryTaskReplayId(route.query[HISTORY_TASK_REPLAY_QUERY_KEY]);

  if (historyId === null) {
    await clearReplayTaskId();
    return;
  }

  try {
    const record = await invoke<HistoryRecord>('get_history_record', { historyId });

    if (record.taskType !== HistoryTaskType.VoiceClone) {
      uiStore.notifyWarning('目标历史任务与当前页面类型不匹配，无法载入配置。');
      return;
    }

    const result = mapHistoryRecordToResult(record);
    if (!result) {
      uiStore.notifyError('历史任务配置解析失败。');
      return;
    }

    applyReplayConfig(result, record.detail.refAudioPath, `已载入历史任务 ${historyId} 的配置，请重新创建新任务。`);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('载入历史任务配置失败，请检查任务记录是否仍然存在', error));
  } finally {
    await clearReplayTaskId();
  }
};

const stopActiveTaskStatusRefresh = () => {
  if (activeTaskStatusTimer) {
    clearInterval(activeTaskStatusTimer);
    activeTaskStatusTimer = null;
  }
};

const syncActiveTaskStatusRefresh = () => {
  stopActiveTaskStatusRefresh();

  if (!activeResult.value || activeResult.value.status === TaskStatus.Completed || activeResult.value.status === TaskStatus.Failed) {
    return;
  }

  activeTaskStatusTimer = setInterval(() => {
    void refreshActiveTaskStatus();
  }, 3000);
};

const selectReferenceAudio = async () => {
  try {
    const selected = await open({
      title: '选择参考音频',
      multiple: false,
      directory: false,
      filters: [{ name: '音频文件', extensions: [...MODEL_TRAINING_AUDIO_FILE_EXTENSIONS] }]
    });

    if (typeof selected !== 'string') {
      return;
    }

    const segments = selected.split(/[/\\]/);
    form.refAudioFile = {
      fileName: segments[segments.length - 1] ?? selected,
      filePath: selected
    };
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('打开文件选择器失败', error));
  }
};

const loadRecentTasks = async ({ manual = false, notifyOnSuccess = false } = {}) => {
  if (isHistoryRefreshInFlight) {
    return;
  }

  isHistoryRefreshInFlight = true;
  if (manual) {
    isRefreshingHistory.value = true;
  }

  try {
    const records = await invoke<HistoryRecord[]>('list_history_records');
    generationHistory.value = records
      .map(mapHistoryRecordToResult)
      .filter((item): item is VoiceCloneResult => item !== null)
      .slice(0, 5);

    if (notifyOnSuccess) {
      uiStore.notifySuccess('声音克隆任务状态已刷新。', 2200);
    }
  } catch (error) {
    generationHistory.value = [];
    uiStore.notifyError(formatErrorMessage('刷新声音克隆历史任务失败，请检查 Rust 后端日志', error));
  } finally {
    isHistoryRefreshInFlight = false;
    if (manual) {
      isRefreshingHistory.value = false;
    }
  }
};

const refreshActiveTaskStatus = async () => {
  if (!activeResult.value || activeResult.value.status === TaskStatus.Completed || activeResult.value.status === TaskStatus.Failed) {
    stopActiveTaskStatusRefresh();
    return;
  }

  if (isActiveTaskRefreshInFlight) {
    return;
  }

  isActiveTaskRefreshInFlight = true;
  const currentTaskId = activeResult.value.taskId;

  try {
    const record = await invoke<HistoryRecord>('get_history_record', { historyId: currentTaskId });
    const nextResult = mapHistoryRecordToResult(record);

    if (!nextResult) {
      return;
    }

    activeResult.value = nextResult;
    generationHistory.value = generationHistory.value.map(item => (item.taskId === nextResult.taskId ? nextResult : item));
    await resultCardRef.value?.refreshDetailRecord();

    if (nextResult.status === TaskStatus.Completed || nextResult.status === TaskStatus.Failed) {
      stopActiveTaskStatusRefresh();
    }
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('刷新声音克隆任务状态失败，请检查后端日志', error));
  } finally {
    isActiveTaskRefreshInFlight = false;
  }
};

const createTask = async () => {
  if (!canGenerate.value || !effectiveRefAudioPath.value) {
    return;
  }

  isGenerating.value = true;
  uiStore.notifyInfo('正在创建声音克隆任务。', 2200);

  try {
    const payload = await invoke<VoiceCloneTaskResultPayload>('create_voice_clone_task', {
      payload: {
        baseModel: form.baseModel,
        modelScale: form.modelScale,
        language: form.language,
        format: form.format,
        exportAudioName: form.exportAudioName,
        refAudioName: effectiveRefAudioName.value || 'reference.wav',
        refAudioPath: effectiveRefAudioPath.value,
        refText: effectiveReferenceText.value,
        text: trimmedText.value,
        modelParams: form.modelParams
      }
    });
    const result = mapResultPayload(payload);
    activeResult.value = result;
    setSelectedHistoryTaskId(result.taskId, true);
    generationHistory.value = [result, ...generationHistory.value.filter(item => item.taskId !== result.taskId)].slice(0, 5);
    syncActiveTaskStatusRefresh();
    uiStore.notifySuccess(`声音克隆任务已创建，任务 ID ${result.taskId}。`, 3600);
  } catch (error) {
    console.error('创建声音克隆任务失败：', error);
    uiStore.notifyError(formatErrorMessage('声音克隆任务创建失败', error));
  } finally {
    isGenerating.value = false;
  }
};

const loadResultAudioAsset = (taskId: number) =>
  invoke<VoiceCloneAudioAssetPayload>('get_voice_clone_audio', {
    historyId: taskId
  });

const saveResultAudio = (taskId: number) =>
  invoke<boolean>('save_voice_clone_audio_as', {
    historyId: taskId
  });

onMounted(async () => {
  await uiConfigStore.ensureLoaded();
  await modelStore.ensureLoaded();
  selectedLanguageOption.value = languageOptions.find(option => option.value === form.language) ?? null;
  selectedFormatOption.value = formatOptions.find(option => option.value === form.format) ?? null;
  await loadRecentTasks();
  await hydrateReplayTaskFromRoute();
});

onBeforeUnmount(() => {
  stopActiveTaskStatusRefresh();
});
</script>

<template>
  <div class="space-y-5">
    <PageHeader title="声音克隆" description="使用参考音频、参考台词和目标文本生成新的语音音频。" eyebrow="Voice-Cloning" />

    <BaseLoadingBanner v-if="activeTaskBusyLabel" :label="activeTaskBusyLabel" />

    <div class="grid gap-5 xl:grid-cols-[1.2fr_1fr]">
      <PanelCard title="基础参数" subtitle="参考音频与参考台词必须严格对应，任务会使用设置页中的全局硬件类型执行克隆推理。">
        <div class="grid gap-4 md:grid-cols-2">
          <BaseListbox v-model="form.baseModel" label="基础模型" :options="modelOptions" />
          <BaseListbox v-model="form.modelScale" label="模型大小" :options="modelScaleOptions" :disabled="modelScaleOptions.length === 0" />
          <BaseListbox v-model="form.language" v-model:selected-option="selectedLanguageOption" label="输出语言" :options="languageOptions" />
          <BaseListbox v-model="form.format" v-model:selected-option="selectedFormatOption" label="输出格式" :options="formatOptions" />
          <label class="block text-sm text-slate-700 md:col-span-2">
            <span class="mb-1 block text-xs text-stone-500">导出音频名称</span>
            <input
              v-model="form.exportAudioName"
              class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
              placeholder="例如 clone_demo"
            />
          </label>

          <label v-if="!isDynamicReferenceModel" class="block md:col-span-2">
            <span class="mb-1 block text-xs text-stone-500">参考音频</span>
            <div class="flex flex-wrap items-center gap-3 rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700">
              <BaseButton tone="ghost" @click="selectReferenceAudio">选择音频</BaseButton>
              <span>{{ form.refAudioFile?.fileName || '尚未选择参考音频' }}</span>
              <span v-if="form.refAudioFile" class="break-all text-xs text-stone-500">{{ form.refAudioFile?.filePath }}</span>
            </div>
          </label>
        </div>
      </PanelCard>

      <div class="space-y-5">
        <GeneratedAudioResultCard
          ref="resultCardRef"
          :result="activeResult"
          :meta-text="activeResultMetaText"
          :load-audio-asset="loadResultAudioAsset"
          :download-audio="saveResultAudio"
          empty-text="还没有生成结果。完成参考音频和文本输入后，结果会显示在这里。"
        >
          <template #details>
            <div v-if="activeResult" class="space-y-1">
              <p>任务 ID：{{ activeResult.taskId }}</p>
              <p>生成时间：{{ activeResult.createdAt }}</p>
              <p>导出名称：{{ activeResult.exportAudioName }}</p>
              <p>参考音频：{{ activeResult.refAudioName }}</p>
              <p v-if="activeResult.refText" class="pt-1 line-clamp-3 text-slate-700">参考台词：{{ activeResult.refText }}</p>
              <p class="pt-1 line-clamp-4 text-slate-700">{{ activeResult.text }}</p>
            </div>
          </template>
        </GeneratedAudioResultCard>

        <PanelCard title="最近任务" subtitle="展示最近 5 条声音克隆任务，数据来自统一历史记录">
          <template #actions>
            <BaseButton tone="ghost" size="sm" :disabled="isRefreshingHistory" @click="loadRecentTasks({ manual: true, notifyOnSuccess: true })">
              <BaseLoadingIndicator v-if="isRefreshingHistory" size="sm" tone="muted" />
              <ArrowPathIcon v-else class="h-4 w-4" aria-hidden="true" />
              <span>{{ isRefreshingHistory ? '刷新中...' : '刷新状态' }}</span>
            </BaseButton>
          </template>

          <RecentTaskList
            :items="recentTaskItems"
            v-model:selected-task-id="selectedHistoryTaskId"
            empty-text="还没有历史任务。生成音频后会自动加入这里。"
            action-label="查看"
          />
        </PanelCard>
      </div>
    </div>

    <PanelCard class="z-20" title="生成参数" subtitle="输入目标台词并配置模型特定参数后生成新的语音音频。">
      <div class="space-y-5 text-sm text-slate-700">
        <label v-if="!isDynamicReferenceModel" class="block">
          <span class="mb-1 block text-xs text-stone-500">参考台词</span>
          <textarea
            v-model="form.refText"
            rows="4"
            class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 outline-none transition focus:border-brand-400"
            placeholder="填写参考音频中实际说出的文本"
          />
        </label>

        <label class="block">
          <span class="mb-1 block text-xs text-stone-500">目标台词</span>
          <textarea
            v-model="form.text"
            rows="5"
            class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 outline-none transition focus:border-brand-400"
            placeholder="填写要合成为新音频的目标文本"
          />
          <div class="mt-2 flex flex-wrap items-center justify-between gap-2 text-xs text-stone-500">
            <span
              >{{ isDynamicReferenceModel ? '参考文本文件路径' : '参考台词' }} {{ effectiveReferenceText.length }} 字，目标台词
              {{ charCount }} 字</span
            >
          </div>
        </label>

        <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-base font-semibold tracking-tight text-slate-900">模型特定参数</p>
          <GenericTaskParamsForm class="mt-4" v-model="form.modelParams" :task-config="activeVoiceCloneTaskConfig" />
        </section>

        <div class="grid gap-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(320px,0.9fr)]">
          <div class="rounded-2xl border border-brand-200 bg-white/80 p-4 text-xs text-stone-600">
            <p>生成摘要</p>
            <p v-for="tip in cloneSummary" :key="tip" class="mt-1">{{ tip }}</p>
          </div>

          <div class="rounded-2xl border border-brand-200 bg-brand-50/35 p-4">
            <div class="flex flex-wrap items-center justify-center gap-2">
              <BaseButton :disabled="!canGenerate" @click="createTask">
                <BaseLoadingIndicator v-if="isGenerating" size="sm" tone="muted" />
                <SparklesIcon v-else class="h-4 w-4" aria-hidden="true" />
                <span>{{ isGenerating ? '生成中...' : '生成音频' }}</span>
              </BaseButton>
            </div>
          </div>
        </div>
      </div>
    </PanelCard>
  </div>
</template>

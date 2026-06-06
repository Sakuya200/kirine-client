<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { ArrowPathIcon, SparklesIcon, StopCircleIcon } from '@heroicons/vue/24/outline';
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseLoadingBanner from '@/components/common/BaseLoadingBanner.vue';
import GeneratedAudioResultCard from '@/components/common/GeneratedAudioResultCard.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import RecentTaskList, { type RecentTaskListItem } from '@/components/common/RecentTaskList.vue';
import WarningConfirmDialog from '@/components/common/WarningConfirmDialog.vue';
import GenericTaskParamsForm from '@/components/form/GenericTaskParamsForm.vue';
import UiParamEmptyState from '@/components/ui/UiParamEmptyState.vue';
import { HARDWARE_TYPE_TEXT, HardwareType } from '@/enums/settings';
import { APP_LANGUAGE_LABELS, AppLanguage } from '@/enums/language';
import { TaskStatus } from '@/enums/status';
import { getHistoryTaskReplayId, HISTORY_TASK_REPLAY_QUERY_KEY, HistoryTaskType } from '@/enums/task';
import { TEXT_TO_SPEECH_FORMATS, TextToSpeechFormat, type TextToSpeechOption } from '@/enums/textToSpeech';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useTaskDeviceTypeGuard } from '@/hooks/useTaskDeviceTypeGuard';
import { useModelStore } from '@/stores/models';
import { useUiConfigStore } from '@/stores/uiConfig';
import { useUiStore } from '@/stores/ui';
import type { HistoryRecord } from '@/types/domain';
import { createTaskExportAudioName } from '@/utils/createTaskExportAudioName';
import { mergeModelParamsWithUiConfigDefaults } from '@/utils/uiConfigModelParams';

interface VoiceDesignResult {
  taskId: number;
  fileName: string;
  baseModel: string;
  modelVersion: string;
  language: AppLanguage;
  languageLabel: string;
  format: TextToSpeechFormat;
  formatLabel: string;
  exportAudioName: string;
  device: string;
  durationSeconds: number;
  prompt: string;
  text: string;
  modelParams: Record<string, unknown>;
  createdAt: string;
  status: TaskStatus;
  outputFilePath: string;
}

interface VoiceDesignTaskResultPayload {
  taskId: number;
  fileName: string;
  baseModel: string;
  modelVersion: string;
  language: AppLanguage;
  format: TextToSpeechFormat;
  exportAudioName: string;
  device: string;
  prompt: string;
  text: string;
  modelParams: Record<string, unknown>;
  durationSeconds: number;
  createdAt: string;
  status: TaskStatus;
  outputFilePath: string;
}

interface VoiceDesignAudioAssetPayload {
  taskId: number;
  fileName: string;
  contentType: string;
  bytes: number[];
}

const createDefaultExportAudioName = () => createTaskExportAudioName(HistoryTaskType.VoiceDesign);

const uiConfigStore = useUiConfigStore();

const normalizeVoiceDesignModelParams = (baseModel: string, modelParams: Record<string, unknown>) => {
  const taskConfig = uiConfigStore.getTaskConfig(baseModel, HistoryTaskType.VoiceDesign);
  return mergeModelParamsWithUiConfigDefaults(taskConfig, modelParams);
};

const uiStore = useUiStore();
const modelStore = useModelStore();
const {
  dialogOpen: showDeviceMismatchDialog,
  dialogTitle: deviceMismatchDialogTitle,
  dialogMessage: deviceMismatchDialogMessage,
  dialogDetailLines: deviceMismatchDialogDetails,
  isCheckingDeviceType,
  isAwaitingDeviceConfirmation,
  isDeviceGuardPending,
  ensureMatchedOrConfirmed,
  confirmDialog: confirmDeviceMismatchDialog,
  closeDialog: closeDeviceMismatchDialog
} = useTaskDeviceTypeGuard();
const route = useRoute();
const router = useRouter();

const form = reactive({
  baseModel: '',
  modelVersion: '',
  language: AppLanguage.Chinese,
  format: TextToSpeechFormat.Wav,
  device: HardwareType.Cpu,
  exportAudioName: createDefaultExportAudioName(),
  prompt: '',
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
const selectedDeviceOption = ref<{ label: string; value: string } | null>(null);
const isGenerating = ref(false);
const isCancelling = ref(false);
const isRefreshingHistory = ref(false);
const activeResult = ref<VoiceDesignResult | null>(null);
const generationHistory = ref<VoiceDesignResult[]>([]);
const selectedHistoryTaskId = ref<number | null>(null);
const resultCardRef = ref<InstanceType<typeof GeneratedAudioResultCard> | null>(null);

let activeTaskStatusTimer: ReturnType<typeof setInterval> | null = null;
let isActiveTaskRefreshInFlight = false;
let isHistoryRefreshInFlight = false;
let skipHistoryTaskSelectionReload = false;

const trimmedPrompt = computed(() => form.prompt.trim());
const trimmedText = computed(() => form.text.trim());
const charCount = computed(() => trimmedText.value.length);
const modelOptions = computed(() =>
  modelStore.getModelsByFeature(HistoryTaskType.VoiceDesign).map(item => ({
    label: item.modelName,
    value: item.baseModel
  }))
);
const modelVersionOptions = computed(() => modelStore.getModelVersionOptions(form.baseModel));
const deviceOptions = computed(() =>
  modelStore.getSupportedDevices(form.baseModel, form.modelVersion).map(device => ({
    value: device,
    label: HARDWARE_TYPE_TEXT[device as HardwareType] ?? device.toUpperCase()
  }))
);
const activeVoiceDesignTaskConfig = computed(() => uiConfigStore.getTaskConfig(form.baseModel, HistoryTaskType.VoiceDesign));
const canGenerate = computed(() => {
  const modelParamsValid = activeVoiceDesignTaskConfig.value
    ? uiConfigStore.validateModelParams(form.baseModel, HistoryTaskType.VoiceDesign, form.modelParams)
    : true;

  return Boolean(form.baseModel) && Boolean(form.modelVersion) && Boolean(trimmedPrompt.value) && Boolean(trimmedText.value) && modelParamsValid;
});
const canCancelActiveTask = computed(() => {
  const result = activeResult.value;
  if (!result) {
    return false;
  }

  return [TaskStatus.Pending, TaskStatus.Running].includes(result.status) && !isCancelling.value;
});
const designSummary = computed(() => [
  `当前模型：${modelStore.getModelLabel(form.baseModel)} ${form.modelVersion}`,
  `当前设备：${HARDWARE_TYPE_TEXT[form.device as HardwareType] ?? form.device.toUpperCase()}`,
  `输出语言：${selectedLanguageOption.value?.label ?? APP_LANGUAGE_LABELS[form.language]}`,
  `输出格式：${selectedFormatOption.value?.label ?? form.format}`,
  `导出名称：${form.exportAudioName}`
]);
const activeResultMetaText = computed(() => {
  if (!activeResult.value) {
    return '';
  }

  return `${modelStore.getModelLabel(activeResult.value.baseModel)} · ${activeResult.value.modelVersion} · ${activeResult.value.languageLabel} · ${activeResult.value.formatLabel}`;
});
const recentTaskItems = computed<RecentTaskListItem[]>(() =>
  generationHistory.value.map(item => ({
    taskId: item.taskId,
    title: item.fileName,
    subtitle: `任务 ${item.taskId} · ${item.languageLabel} · ${item.exportAudioName}`,
    status: item.status
  }))
);
const activeTaskBusyLabel = computed(() => {
  if (isCancelling.value) {
    return '正在发送终止请求...';
  }

  if (isCheckingDeviceType.value) {
    return '正在检查模型环境...';
  }

  if (isAwaitingDeviceConfirmation.value) {
    return '等待确认硬件环境切换';
  }

  if (isGenerating.value) {
    return '正在创建音色设计任务...';
  }

  if (activeResult.value?.status === TaskStatus.Pending || activeResult.value?.status === TaskStatus.Running) {
    return '任务执行中，状态会自动刷新。';
  }

  return '';
});
const isSubmitPending = computed(() => isGenerating.value || isDeviceGuardPending.value);
const submitButtonText = computed(() => {
  if (isCheckingDeviceType.value) {
    return '检查环境中...';
  }

  if (isAwaitingDeviceConfirmation.value) {
    return '等待确认...';
  }

  return isGenerating.value ? '生成中...' : '生成音频';
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
  modelVersionOptions,
  options => {
    if (options.length === 0) {
      form.modelVersion = '';
      return;
    }

    if (!options.some(option => option.value === form.modelVersion)) {
      form.modelVersion = String(options[0]?.value ?? '');
    }
  },
  { immediate: true }
);

watch(
  () => [form.baseModel, form.modelVersion],
  ([nextBaseModel, nextModelVersion], [prevBaseModel, prevModelVersion]) => {
    if (!nextBaseModel || !nextModelVersion) {
      return;
    }

    if (nextBaseModel === prevBaseModel && nextModelVersion === prevModelVersion) {
      return;
    }

    form.modelParams = normalizeVoiceDesignModelParams(nextBaseModel, form.modelParams);
  }
);

watch(
  () => form.language,
  next => {
    selectedLanguageOption.value = languageOptions.find(option => option.value === next) ?? null;
  },
  { immediate: true }
);

watch(
  () => form.format,
  next => {
    selectedFormatOption.value = formatOptions.find(option => option.value === next) ?? null;
  },
  { immediate: true }
);

watch(
  deviceOptions,
  options => {
    if (options.length === 0) {
      return;
    }

    if (!options.some(option => option.value === form.device)) {
      form.device = (options[0]?.value as HardwareType) ?? HardwareType.Cpu;
    }

    selectedDeviceOption.value = options.find(option => option.value === form.device) ?? null;
  },
  { immediate: true }
);

const mapResultPayload = (payload: VoiceDesignTaskResultPayload): VoiceDesignResult => ({
  taskId: payload.taskId,
  fileName: payload.fileName,
  baseModel: payload.baseModel,
  modelVersion: payload.modelVersion,
  language: payload.language,
  languageLabel: APP_LANGUAGE_LABELS[payload.language] ?? payload.language,
  format: payload.format,
  formatLabel: formatOptions.find(option => option.value === payload.format)?.label ?? payload.format,
  exportAudioName: payload.exportAudioName,
  device: payload.device,
  durationSeconds: payload.durationSeconds,
  prompt: payload.prompt,
  text: payload.text,
  modelParams: payload.modelParams,
  createdAt: payload.createdAt,
  status: payload.status,
  outputFilePath: payload.outputFilePath
});

const mapHistoryRecordToResult = (record: HistoryRecord): VoiceDesignResult | null => {
  if (record.taskType !== HistoryTaskType.VoiceDesign) {
    return null;
  }

  return {
    taskId: record.id,
    fileName: record.detail.fileName,
    baseModel: record.detail.baseModel,
    modelVersion: record.detail.modelVersion,
    language: record.detail.language,
    languageLabel: APP_LANGUAGE_LABELS[record.detail.language] ?? record.detail.language,
    format: record.detail.format,
    formatLabel: formatOptions.find(option => option.value === record.detail.format)?.label ?? record.detail.format,
    exportAudioName: record.detail.exportAudioName,
    device: record.device,
    durationSeconds: record.durationSeconds,
    prompt: record.detail.prompt,
    text: record.detail.text,
    modelParams: record.detail.modelParams,
    createdAt: record.createTime,
    status: record.status,
    outputFilePath: record.detail.outputFilePath
  };
};

const applyResultToForm = (result: VoiceDesignResult) => {
  form.baseModel = result.baseModel;
  form.modelVersion = result.modelVersion;
  form.language = result.language;
  form.format = result.format;
  form.device = result.device as HardwareType;
  form.exportAudioName = result.exportAudioName;
  form.prompt = result.prompt;
  form.text = result.text;
  form.modelParams = normalizeVoiceDesignModelParams(result.baseModel, result.modelParams);
};

const setSelectedHistoryTaskId = (taskId: number | null, skipReload = false) => {
  skipHistoryTaskSelectionReload = skipReload;
  selectedHistoryTaskId.value = taskId;
};

const syncActiveTaskStatusRefresh = () => {
  const status = activeResult.value?.status;

  if (status === TaskStatus.Pending || status === TaskStatus.Running) {
    if (activeTaskStatusTimer) {
      return;
    }

    activeTaskStatusTimer = setInterval(() => {
      void refreshActiveTaskStatus();
    }, 3000);

    return;
  }

  stopActiveTaskStatusRefresh();
};

const stopActiveTaskStatusRefresh = () => {
  if (!activeTaskStatusTimer) {
    return;
  }

  clearInterval(activeTaskStatusTimer);
  activeTaskStatusTimer = null;
};

const loadRecentTasks = async ({ manual = false, notifyOnSuccess = false, silentOnError = false } = {}) => {
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
      .filter((item): item is VoiceDesignResult => item !== null)
      .slice(0, 5);

    if (notifyOnSuccess) {
      uiStore.notifySuccess('音色设计任务状态已刷新。', 2200);
    }
  } catch (error) {
    generationHistory.value = [];
    if (!silentOnError) {
      uiStore.notifyError(formatErrorMessage('刷新音色设计历史任务失败，请检查 Rust 后端日志。', error));
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
    !activeResult.value ||
    activeResult.value.status === TaskStatus.Completed ||
    activeResult.value.status === TaskStatus.Cancelled ||
    activeResult.value.status === TaskStatus.Failed
  ) {
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
    uiStore.notifyError(formatErrorMessage('刷新音色设计任务状态失败，请检查后端日志。', error));
  } finally {
    isActiveTaskRefreshInFlight = false;
  }
};

const hydrateReplayTaskFromRoute = async () => {
  const historyId = getHistoryTaskReplayId(route.query[HISTORY_TASK_REPLAY_QUERY_KEY]);
  if (!historyId) {
    return;
  }

  try {
    const record = await invoke<HistoryRecord>('get_history_record', { historyId });
    const replayResult = mapHistoryRecordToResult(record);
    if (!replayResult) {
      uiStore.notifyWarning('当前历史任务不是音色设计任务，无法回填。');
      return;
    }

    applyResultToForm(replayResult);
    activeResult.value = replayResult;
    setSelectedHistoryTaskId(replayResult.taskId, true);
    syncActiveTaskStatusRefresh();
    uiStore.notifyInfo(`已回填任务 ${replayResult.taskId} 的参数，可直接再次生成。`, 3200);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('读取回放任务失败', error));
  } finally {
    if (!(HISTORY_TASK_REPLAY_QUERY_KEY in route.query)) {
      return;
    }

    const nextQuery = { ...route.query };
    delete nextQuery[HISTORY_TASK_REPLAY_QUERY_KEY];
    await router.replace({ path: route.path, query: nextQuery });
  }
};

watch(selectedHistoryTaskId, async taskId => {
  if (!taskId || skipHistoryTaskSelectionReload) {
    skipHistoryTaskSelectionReload = false;
    return;
  }

  try {
    const record = await invoke<HistoryRecord>('get_history_record', { historyId: taskId });
    const result = mapHistoryRecordToResult(record);

    if (!result) {
      uiStore.notifyWarning('当前记录不是音色设计任务。');
      return;
    }

    activeResult.value = result;
    applyResultToForm(result);
    syncActiveTaskStatusRefresh();
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('加载历史任务详情失败，请检查后端日志。', error));
  }
});

const createTask = async () => {
  if (!canGenerate.value) {
    return;
  }

  const accepted = await ensureMatchedOrConfirmed({
    baseModel: form.baseModel,
    modelVersion: form.modelVersion,
    selectedDevice: form.device
  });
  if (!accepted) {
    return;
  }

  isGenerating.value = true;
  uiStore.notifyInfo('正在创建音色设计任务。', 2200);

  try {
    const payload = await invoke<VoiceDesignTaskResultPayload>('create_voice_design_task', {
      payload: {
        baseModel: form.baseModel,
        modelVersion: form.modelVersion,
        language: form.language,
        format: form.format,
        exportAudioName: form.exportAudioName,
        device: form.device,
        prompt: trimmedPrompt.value,
        text: trimmedText.value,
        modelParams: form.modelParams
      }
    });
    const result = mapResultPayload(payload);
    activeResult.value = result;
    setSelectedHistoryTaskId(result.taskId, true);
    generationHistory.value = [result, ...generationHistory.value.filter(item => item.taskId !== result.taskId)].slice(0, 5);
    syncActiveTaskStatusRefresh();
    uiStore.notifySuccess(`音色设计任务已创建，任务 ID ${result.taskId}。`, 3600);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('音色设计任务创建失败', error));
  } finally {
    isGenerating.value = false;
  }
};

const cancelActiveTask = async () => {
  if (!activeResult.value || ![TaskStatus.Pending, TaskStatus.Running].includes(activeResult.value.status)) {
    return;
  }

  isCancelling.value = true;
  const taskId = activeResult.value.taskId;

  try {
    const accepted = await invoke<boolean>('cancel_history_task', {
      historyId: taskId
    });

    if (!accepted) {
      uiStore.notifyWarning('当前任务已经提交过终止请求。');
      return;
    }

    uiStore.notifyInfo(`已发送终止请求，任务 ${taskId} 会在后端停止后刷新状态。`, 3600);
    await refreshActiveTaskStatus();
    await loadRecentTasks({ silentOnError: true });
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('终止任务失败', error));
  } finally {
    isCancelling.value = false;
  }
};

const loadResultAudioAsset = (taskId: number) =>
  invoke<VoiceDesignAudioAssetPayload>('get_voice_design_audio', {
    historyId: taskId
  });

const saveResultAudio = (taskId: number) =>
  invoke<boolean>('save_voice_design_audio_as', {
    historyId: taskId
  });

const resetForm = () => {
  form.baseModel = modelOptions.value[0]?.value ?? '';
  form.modelVersion = modelVersionOptions.value[0]?.value ?? '';
  form.language = AppLanguage.Chinese;
  form.format = TextToSpeechFormat.Wav;
  form.device = HardwareType.Cpu;
  form.exportAudioName = createDefaultExportAudioName();
  form.prompt = '';
  form.text = '';
  form.modelParams = {};
  selectedLanguageOption.value = languageOptions.find(option => option.value === form.language) ?? null;
  selectedFormatOption.value = formatOptions.find(option => option.value === form.format) ?? null;
  selectedDeviceOption.value = deviceOptions.value.find(option => option.value === HardwareType.Cpu) ?? null;
  uiStore.notifyInfo('表单已重置。', 2200);
};

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
    <PageHeader title="音色设计" description="输入音色描述与目标台词，生成符合指定风格的语音音频。" eyebrow="Voice-Design" />

    <BaseLoadingBanner v-if="activeTaskBusyLabel" :label="activeTaskBusyLabel" />

    <div class="grid gap-5 xl:grid-cols-[1.2fr_1fr]">
      <PanelCard title="基础参数" subtitle="支持 qwen3_tts 的 1.7B 与 0.6B 版本，设备类型按任务单独选择。">
        <div class="grid gap-4 md:grid-cols-2">
          <BaseListbox v-model="form.baseModel" label="基础模型" :options="modelOptions" />
          <BaseListbox v-model="form.modelVersion" label="模型版本" :options="modelVersionOptions" :disabled="modelVersionOptions.length === 0" />
          <BaseListbox
            v-model="form.device"
            v-model:selected-option="selectedDeviceOption"
            label="设备类型"
            :options="deviceOptions"
            :disabled="deviceOptions.length === 0"
          />
          <BaseListbox v-model="form.language" v-model:selected-option="selectedLanguageOption" label="输出语言" :options="languageOptions" />
          <BaseListbox v-model="form.format" v-model:selected-option="selectedFormatOption" label="输出格式" :options="formatOptions" />
          <label class="block text-sm text-slate-700 md:col-span-2">
            <span class="mb-1 block text-xs text-stone-500">导出音频名称</span>
            <input
              v-model="form.exportAudioName"
              class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
              placeholder="例如 voice_design_demo"
            />
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
          empty-text="还没有生成结果。完成音色描述和目标文本输入后，结果会显示在这里。"
          @cancel="cancelActiveTask"
        >
          <template #details>
            <div v-if="activeResult" class="space-y-1">
              <p>任务 ID：{{ activeResult.taskId }}</p>
              <p>生成时间：{{ activeResult.createdAt }}</p>
              <p>导出名称：{{ activeResult.exportAudioName }}</p>
              <p class="pt-1 line-clamp-3 text-slate-700">音色描述：{{ activeResult.prompt }}</p>
              <p class="pt-1 line-clamp-4 text-slate-700">{{ activeResult.text }}</p>
            </div>
          </template>
        </GeneratedAudioResultCard>

        <PanelCard title="最近任务" subtitle="展示最近 5 条音色设计任务，数据来自统一历史记录">
          <template #actions>
            <BaseButton tone="ghost" size="sm" :loading="isRefreshingHistory" @click="loadRecentTasks({ manual: true, notifyOnSuccess: true })">
              <ArrowPathIcon v-if="!isRefreshingHistory" class="h-4 w-4" aria-hidden="true" />
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

    <PanelCard class="z-20" title="生成参数" subtitle="输入音色描述 Prompt 与目标台词，生成符合需求的人声。">
      <div class="space-y-5 text-sm text-slate-700">
        <label class="block">
          <span class="mb-1 block text-xs text-stone-500">音色描述 Prompt</span>
          <textarea
            v-model="form.prompt"
            rows="4"
            class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 outline-none transition focus:border-brand-400"
            placeholder="例如：体现温柔成熟的女声，语速平稳，音色偏暖，带轻微气声。"
          />
        </label>

        <label class="block">
          <span class="mb-1 block text-xs text-stone-500">目标台词</span>
          <textarea
            v-model="form.text"
            rows="5"
            class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 outline-none transition focus:border-brand-400"
            placeholder="填写需要合成的目标文本"
          />
          <div class="mt-2 flex flex-wrap items-center justify-between gap-2 text-xs text-stone-500">
            <span>当前字符数 {{ charCount }}</span>
          </div>
        </label>

        <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-base font-semibold tracking-tight text-slate-900">模型特定参数</p>
          <div class="mt-4">
            <GenericTaskParamsForm v-if="activeVoiceDesignTaskConfig" v-model="form.modelParams" :task-config="activeVoiceDesignTaskConfig" />
            <UiParamEmptyState v-else title="当前模型没有可配置的特定参数" description="这个任务下没有额外参数需要配置，可以直接继续生成音色设计。" />
          </div>
        </section>

        <div class="grid gap-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(320px,0.9fr)]">
          <div class="rounded-2xl border border-brand-200 bg-white/80 p-4 text-xs text-stone-600">
            <p>生成摘要</p>
            <p v-for="tip in designSummary" :key="tip" class="mt-1">{{ tip }}</p>
          </div>

          <div class="rounded-2xl border border-brand-200 bg-brand-50/35 p-4">
            <div class="flex flex-wrap items-center justify-center gap-2">
              <BaseButton :loading="isSubmitPending" :disabled="!canGenerate || isSubmitPending" @click="createTask">
                <SparklesIcon v-if="!isSubmitPending" class="h-4 w-4" aria-hidden="true" />
                <span>{{ submitButtonText }}</span>
              </BaseButton>
              <BaseButton tone="quiet" :loading="isCancelling" :disabled="!canCancelActiveTask" @click="cancelActiveTask">
                <StopCircleIcon v-if="!isCancelling" class="h-4 w-4" aria-hidden="true" />
                <span>{{ isCancelling ? '终止中...' : '终止任务' }}</span>
              </BaseButton>
              <BaseButton tone="ghost" @click="resetForm">
                <ArrowPathIcon class="h-4 w-4" aria-hidden="true" />
                <span>重置表单</span>
              </BaseButton>
            </div>
          </div>
        </div>
      </div>
    </PanelCard>

    <WarningConfirmDialog
      :open="showDeviceMismatchDialog"
      :title="deviceMismatchDialogTitle"
      :message="deviceMismatchDialogMessage"
      :detail-lines="deviceMismatchDialogDetails"
      @confirm="confirmDeviceMismatchDialog"
      @close="closeDeviceMismatchDialog"
    />
  </div>
</template>

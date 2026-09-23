<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { ArrowPathIcon, SparklesIcon, StopCircleIcon } from '@heroicons/vue/24/outline';
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
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
import { loadRecentHistoryRecords } from '@/hooks/loadRecentHistoryRecords';
import { usePollingResume } from '@/hooks/usePollingResume';
import { useTaskDeviceTypeGuard } from '@/hooks/useTaskDeviceTypeGuard';
import { useModels } from '@/hooks/useModels';
import { useUiConfigStore } from '@/stores/uiConfig';
import { useUiStore } from '@/stores/ui';
import type { HistoryRecord } from '@/types/domain';
import { saveGeneratedAudio } from '@/utils/audioDownload';
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
// 单次状态刷新被允许挂起的最长时间：超过即视为被系统睡眠冻结，重置占用标志恢复轮询。
const ACTIVE_TASK_REFRESH_STALE_MS = 15_000;

const uiConfigStore = useUiConfigStore();

const normalizeVoiceDesignModelParams = (baseModel: string, modelParams: Record<string, unknown>) => {
  const taskConfig = uiConfigStore.getTaskConfig(baseModel, HistoryTaskType.VoiceDesign);
  return mergeModelParamsWithUiConfigDefaults(taskConfig, modelParams);
};

const uiStore = useUiStore();
const { getModelsByFeature, getModelVersionOptions, getSupportedDevices, getSupportedLanguages, getModelLabel } = useModels();
const { t } = useI18n();
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

const formatOptions = computed(() => TEXT_TO_SPEECH_FORMATS.map(option => ({ ...option, label: t(option.label) })));
const selectedLanguageOption = ref<{ label: string; value: AppLanguage } | null>(null);
const selectedFormatOption = ref<TextToSpeechOption | null>(formatOptions.value[0] ?? null);
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
// 单次刷新开始时间 + 代际令牌：用于检测被系统睡眠冻结的 invoke 并防止其晚到的
// finally 错误清掉新一次刷新的占用标志。见 refreshActiveTaskStatus。
let activeTaskRefreshStartedAt = 0;
let activeTaskRefreshGeneration = 0;
let isHistoryRefreshInFlight = false;
let skipHistoryTaskSelectionReload = false;

const trimmedPrompt = computed(() => form.prompt.trim());
const trimmedText = computed(() => form.text.trim());
const charCount = computed(() => trimmedText.value.length);
const modelOptions = computed(() =>
  getModelsByFeature(HistoryTaskType.VoiceDesign).map(item => ({
    label: item.modelName,
    value: item.baseModel
  }))
);
const modelVersionOptions = computed(() => getModelVersionOptions(form.baseModel));
const deviceOptions = computed(() =>
  getSupportedDevices(form.baseModel, form.modelVersion).map(device => ({
    value: device,
    label: HARDWARE_TYPE_TEXT[device as HardwareType] ?? device.toUpperCase()
  }))
);
const languageOptions = computed(() =>
  getSupportedLanguages(form.baseModel, form.modelVersion).map(language => ({
    value: language,
    label: APP_LANGUAGE_LABELS[language] ?? language
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
  t('voiceDesign.summary.model', { model: getModelLabel(form.baseModel), version: form.modelVersion }),
  t('voiceDesign.summary.device', { device: HARDWARE_TYPE_TEXT[form.device as HardwareType] ?? form.device.toUpperCase() }),
  t('voiceDesign.summary.language', { language: selectedLanguageOption.value?.label ?? APP_LANGUAGE_LABELS[form.language] }),
  t('voiceDesign.summary.format', { format: selectedFormatOption.value?.label ?? form.format }),
  t('voiceDesign.summary.exportName', { name: form.exportAudioName })
]);
const activeResultMetaText = computed(() => {
  if (!activeResult.value) {
    return '';
  }

  return `${getModelLabel(activeResult.value.baseModel)} · ${activeResult.value.modelVersion} · ${activeResult.value.languageLabel} · ${activeResult.value.formatLabel}`;
});
const recentTaskItems = computed<RecentTaskListItem[]>(() =>
  generationHistory.value.map(item => ({
    taskId: item.taskId,
    title: item.fileName,
    subtitle: t('voiceDesign.recent.subtitle', { taskId: item.taskId, language: item.languageLabel, fileName: item.exportAudioName }),
    status: item.status
  }))
);
const activeTaskBusyLabel = computed(() => {
  if (isCancelling.value) {
    return t('voiceDesign.busy.cancelling');
  }

  if (isCheckingDeviceType.value) {
    return t('voiceDesign.busy.checkingDevice');
  }

  if (isAwaitingDeviceConfirmation.value) {
    return t('voiceDesign.busy.awaitingDeviceConfirm');
  }

  if (isGenerating.value) {
    return t('voiceDesign.busy.creating');
  }

  if (activeResult.value?.status === TaskStatus.Pending || activeResult.value?.status === TaskStatus.Running) {
    return t('voiceDesign.busy.running');
  }

  return '';
});
const isSubmitPending = computed(() => isGenerating.value || isDeviceGuardPending.value);
const submitButtonText = computed(() => {
  if (isCheckingDeviceType.value) {
    return t('voiceDesign.submit.checking');
  }

  if (isAwaitingDeviceConfirmation.value) {
    return t('voiceDesign.submit.awaitingConfirm');
  }

  return isGenerating.value ? t('voiceDesign.submit.generating') : t('voiceDesign.submit.generate');
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
  languageOptions,
  options => {
    if (options.length === 0) {
      form.language = AppLanguage.Chinese;
      selectedLanguageOption.value = null;
      return;
    }

    const matched = options.find(option => option.value === form.language) ?? options[0] ?? null;
    form.language = (matched?.value ?? AppLanguage.Chinese) as AppLanguage;
    selectedLanguageOption.value = matched;
  },
  { immediate: true }
);

watch(
  () => form.format,
  next => {
    selectedFormatOption.value = formatOptions.value.find(option => option.value === next) ?? null;
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
  formatLabel: t(TEXT_TO_SPEECH_FORMATS.find(option => option.value === payload.format)?.label ?? payload.format),
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
    formatLabel: t(TEXT_TO_SPEECH_FORMATS.find(option => option.value === record.detail.format)?.label ?? record.detail.format),
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
    const records = await loadRecentHistoryRecords(HistoryTaskType.VoiceDesign, 5);
    generationHistory.value = records
      .map(mapHistoryRecordToResult)
      .filter((item): item is VoiceDesignResult => item !== null)
      .slice(0, 5);

    if (notifyOnSuccess) {
      uiStore.notifySuccess(t('voiceDesign.notice.statusRefreshed'), 2200);
    }
  } catch (error) {
    generationHistory.value = [];
    if (!silentOnError) {
      uiStore.notifyError(formatErrorMessage(t('voiceDesign.notice.refreshFailed'), error));
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
    // 系统睡眠 / 锁屏会使正在 await 的 invoke 被冻结，占用标志长期为 true，
    // 唤醒后所有 tick 都被这里拦截 → 状态不再更新。超过阈值视为被冻结卡死，
    // 重置占用标志以恢复轮询（代际令牌保证旧 invoke 晚到时不会错误复位）。
    if (activeTaskRefreshStartedAt && Date.now() - activeTaskRefreshStartedAt > ACTIVE_TASK_REFRESH_STALE_MS) {
      isActiveTaskRefreshInFlight = false;
    } else {
      return;
    }
  }

  isActiveTaskRefreshInFlight = true;
  activeTaskRefreshStartedAt = Date.now();
  const generation = ++activeTaskRefreshGeneration;
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
    uiStore.notifyError(formatErrorMessage(t('voiceDesign.notice.refreshCurrentFailed'), error));
  } finally {
    if (generation === activeTaskRefreshGeneration) {
      isActiveTaskRefreshInFlight = false;
      activeTaskRefreshStartedAt = 0;
    }
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
      uiStore.notifyWarning(t('voiceDesign.notice.notDesignTask'));
      return;
    }

    applyResultToForm(replayResult);
    activeResult.value = replayResult;
    setSelectedHistoryTaskId(replayResult.taskId, true);
    syncActiveTaskStatusRefresh();
    uiStore.notifyInfo(t('voiceDesign.notice.replayFilled', { taskId: replayResult.taskId }), 3200);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('voiceDesign.notice.replayFailed'), error));
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
      uiStore.notifyWarning(t('voiceDesign.notice.notDesignRecord'));
      return;
    }

    activeResult.value = result;
    applyResultToForm(result);
    syncActiveTaskStatusRefresh();
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('voiceDesign.notice.loadDetailFailed'), error));
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
  uiStore.notifyInfo(t('voiceDesign.notice.submitting'), 2200);

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
    uiStore.notifySuccess(t('voiceDesign.notice.created', { taskId: result.taskId }), 3600);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('voiceDesign.notice.createFailed'), error));
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
      uiStore.notifyWarning(t('tts.notice.alreadyCancelling'));
      return;
    }

    uiStore.notifyInfo(t('tts.notice.cancelRequested', { taskId }), 3600);
    await refreshActiveTaskStatus();
    await loadRecentTasks({ silentOnError: true });
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('tts.notice.cancelFailed'), error));
  } finally {
    isCancelling.value = false;
  }
};

const loadResultAudioAsset = (taskId: number) =>
  invoke<VoiceDesignAudioAssetPayload>('get_generated_audio', {
    source: { kind: 'voice-design', historyId: taskId }
  });

const saveResultAudio = (taskId: number) => saveGeneratedAudio({ kind: 'voice-design', historyId: taskId });

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
  selectedLanguageOption.value = languageOptions.value.find(option => option.value === form.language) ?? null;
  selectedFormatOption.value = formatOptions.value.find(option => option.value === form.format) ?? null;
  selectedDeviceOption.value = deviceOptions.value.find(option => option.value === HardwareType.Cpu) ?? null;
  uiStore.notifyInfo(t('voiceDesign.notice.formReset'), 2200);
};

onMounted(async () => {
  await uiConfigStore.ensureLoaded();
  selectedLanguageOption.value = languageOptions.value.find(option => option.value === form.language) ?? null;
  selectedFormatOption.value = formatOptions.value.find(option => option.value === form.format) ?? null;
  await loadRecentTasks();
  await hydrateReplayTaskFromRoute();
});

onBeforeUnmount(() => {
  stopActiveTaskStatusRefresh();
});

// 解除锁屏 / 唤醒后立即补一次刷新，避免 setInterval 被节流期间状态停滞。
usePollingResume(() => {
  void refreshActiveTaskStatus();
});
</script>

<template>
  <div class="space-y-5">
    <PageHeader :title="t('voiceDesign.title')" :description="t('voiceDesign.description')" eyebrow="Voice-Design" />

    <BaseLoadingBanner v-if="activeTaskBusyLabel" :label="activeTaskBusyLabel" />

    <div class="grid gap-5 xl:grid-cols-[1.2fr_1fr]">
      <PanelCard :title="t('voiceDesign.panels.basic')" :subtitle="t('voiceDesign.panels.basicSubtitle')">
        <div class="grid gap-4 md:grid-cols-2">
          <BaseListbox v-model="form.baseModel" :label="t('tts.form.baseModel')" :options="modelOptions" />
          <BaseListbox v-model="form.modelVersion" :label="t('tts.form.modelVersion')" :options="modelVersionOptions" :disabled="modelVersionOptions.length === 0" />
          <BaseListbox
            v-model="form.device"
            v-model:selected-option="selectedDeviceOption"
            :label="t('tts.form.deviceType')"
            :options="deviceOptions"
            :disabled="deviceOptions.length === 0"
          />
          <BaseListbox v-model="form.language" v-model:selected-option="selectedLanguageOption" :label="t('tts.form.language')" :options="languageOptions" />
          <BaseListbox v-model="form.format" v-model:selected-option="selectedFormatOption" :label="t('tts.form.format')" :options="formatOptions" />
          <label class="block text-sm text-slate-700 md:col-span-2">
            <span class="mb-1 block text-xs text-stone-500">{{ t('tts.form.exportAudioName') }}</span>
            <input
              v-model="form.exportAudioName"
              class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
              :placeholder="t('voiceDesign.form.exportNamePlaceholder')"
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
          :empty-text="t('voiceDesign.result.emptyText')"
          @cancel="cancelActiveTask"
        >
          <template #details>
            <div v-if="activeResult" class="space-y-1">
              <p>{{ t('voiceDesign.result.taskId', { id: activeResult.taskId }) }}</p>
              <p>{{ t('voiceDesign.result.createdAt', { time: activeResult.createdAt }) }}</p>
              <p>{{ t('voiceDesign.result.exportName', { name: activeResult.exportAudioName }) }}</p>
              <p class="pt-1 line-clamp-3 text-slate-700">{{ t('voiceDesign.result.prompt', { text: activeResult.prompt }) }}</p>
              <p class="pt-1 line-clamp-4 text-slate-700">{{ activeResult.text }}</p>
            </div>
          </template>
        </GeneratedAudioResultCard>

        <PanelCard :title="t('voiceDesign.panels.recent')" :subtitle="t('voiceDesign.panels.recentSubtitle')">
          <template #actions>
            <BaseButton tone="ghost" size="sm" :loading="isRefreshingHistory" @click="loadRecentTasks({ manual: true, notifyOnSuccess: true })">
              <ArrowPathIcon v-if="!isRefreshingHistory" class="h-4 w-4" aria-hidden="true" />
              <span>{{ isRefreshingHistory ? t('tts.form.refreshing') : t('tts.form.refresh') }}</span>
            </BaseButton>
          </template>

          <RecentTaskList
            :items="recentTaskItems"
            v-model:selected-task-id="selectedHistoryTaskId"
            :empty-text="t('tts.result.historyEmptyText')"
            :action-label="t('tts.form.view')"
          />
        </PanelCard>
      </div>
    </div>

    <PanelCard class="z-20" :title="t('voiceDesign.panels.params')" :subtitle="t('voiceDesign.panels.paramsSubtitle')">
      <div class="space-y-5 text-sm text-slate-700">
        <label class="block">
          <span class="mb-1 block text-xs text-stone-500">{{ t('voiceDesign.form.prompt') }}</span>
          <textarea
            v-model="form.prompt"
            rows="4"
            class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 outline-none transition focus:border-brand-400"
            :placeholder="t('voiceDesign.form.promptPlaceholder')"
          />
        </label>

        <label class="block">
          <span class="mb-1 block text-xs text-stone-500">{{ t('voiceDesign.form.text') }}</span>
          <textarea
            v-model="form.text"
            rows="5"
            class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 outline-none transition focus:border-brand-400"
            :placeholder="t('voiceDesign.form.textPlaceholder')"
          />
          <div class="mt-2 flex flex-wrap items-center justify-between gap-2 text-xs text-stone-500">
            <span>{{ t('voiceDesign.form.charStats', { chars: charCount }) }}</span>
          </div>
        </label>

        <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-base font-semibold tracking-tight text-slate-900">{{ t('tts.form.modelParams') }}</p>
          <div class="mt-4">
            <GenericTaskParamsForm v-if="activeVoiceDesignTaskConfig" v-model="form.modelParams" :task-config="activeVoiceDesignTaskConfig" />
            <UiParamEmptyState v-else :title="t('voiceDesign.form.emptyParamsTitle')" :description="t('voiceDesign.form.emptyParamsDesc')" />
          </div>
        </section>

        <div class="grid gap-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(320px,0.9fr)]">
          <div class="rounded-2xl border border-brand-200 bg-white/80 p-4 text-xs text-stone-600">
            <p>{{ t('tts.form.summary') }}</p>
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
                <span>{{ isCancelling ? t('tts.form.cancelling') : t('tts.form.cancel') }}</span>
              </BaseButton>
              <BaseButton tone="ghost" @click="resetForm">
                <ArrowPathIcon class="h-4 w-4" aria-hidden="true" />
                <span>{{ t('tts.form.resetForm') }}</span>
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

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { ArrowPathIcon, ClipboardDocumentIcon, SparklesIcon, StopCircleIcon } from '@heroicons/vue/24/outline';
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseLoadingBanner from '@/components/common/BaseLoadingBanner.vue';
import BaseDialog from '@/components/common/BaseDialog.vue';
import GeneratedAudioResultCard from '@/components/common/GeneratedAudioResultCard.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import RecentTaskList, { type RecentTaskListItem } from '@/components/common/RecentTaskList.vue';
import WarningConfirmDialog from '@/components/common/WarningConfirmDialog.vue';
import GenericTaskParamsForm from '@/components/form/GenericTaskParamsForm.vue';
import { HARDWARE_TYPE_TEXT, HardwareType } from '@/enums/settings';
import { AppLanguage, APP_LANGUAGE_LABELS } from '@/enums/language';
import { SpeakerStatus, TaskStatus } from '@/enums/status';
import { getHistoryTaskReplayId, HISTORY_TASK_REPLAY_QUERY_KEY, HistoryTaskType } from '@/enums/task';
import { TEXT_TO_SPEECH_FORMATS, TextToSpeechFormat, type TextToSpeechOption, type TextToSpeechSpeakerOption } from '@/enums/textToSpeech';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { loadRecentHistoryRecords } from '@/hooks/loadRecentHistoryRecords';
import { usePollingResume } from '@/hooks/usePollingResume';
import { useTaskDeviceTypeGuard } from '@/hooks/useTaskDeviceTypeGuard';
import { useModels } from '@/hooks/useModels';
import { useUiConfigStore } from '@/stores/uiConfig';
import { useUiStore } from '@/stores/ui';
import type { HistoryRecord, SpeakerPagedResult, SpeakerProfile } from '@/types/domain';
import { saveGeneratedAudio } from '@/utils/audioDownload';
import { createTaskExportAudioName } from '@/utils/createTaskExportAudioName';
import { mergeModelParamsWithUiConfigDefaults } from '@/utils/uiConfigModelParams';

interface TtsResult {
  taskId: number;
  fileName: string;
  speakerId: number | null;
  speakerLabel: string;
  baseModel: string;
  modelVersion: string;
  language: AppLanguage;
  languageLabel: string;
  format: TextToSpeechFormat;
  formatLabel: string;
  exportAudioName: string;
  device: string;
  durationSeconds: number;
  text: string;
  modelParams: Record<string, unknown>;
  createdAt: string;
  status: TaskStatus;
  outputFilePath: string;
}

interface TextToSpeechTaskResultPayload {
  taskId: number;
  fileName: string;
  speakerId: number | null;
  speakerLabel: string;
  baseModel: string;
  modelVersion: string;
  language: AppLanguage;
  format: TextToSpeechFormat;
  exportAudioName: string;
  device: string;
  text: string;
  modelParams: Record<string, unknown>;
  durationSeconds: number;
  createdAt: string;
  status: TaskStatus;
  outputFilePath: string;
}

interface TextToSpeechAudioAssetPayload {
  taskId: number;
  fileName: string;
  contentType: string;
  bytes: number[];
}

const DYNAMIC_REFERENCE_BASE_MODELS = new Set(['gpt_sovits_cpufast']);
// 单次状态刷新被允许挂起的最长时间：超过即视为被系统睡眠冻结，重置占用标志恢复轮询。
const ACTIVE_TASK_REFRESH_STALE_MS = 15_000;
const createDefaultExportAudioName = () => createTaskExportAudioName(HistoryTaskType.TextToSpeech);

const uiConfigStore = useUiConfigStore();
const { t } = useI18n();

const normalizeTtsModelParams = (baseModel: string, modelParams: Record<string, unknown>) => {
  const taskConfig = uiConfigStore.getTaskConfig(baseModel, HistoryTaskType.TextToSpeech);
  return mergeModelParamsWithUiConfigDefaults(taskConfig, modelParams);
};

const form = reactive({
  speakerId: null as number | null,
  baseModel: '',
  modelVersion: '',
  language: AppLanguage.Chinese,
  format: TextToSpeechFormat.Wav,
  device: HardwareType.Cpu,
  exportAudioName: createDefaultExportAudioName(),
  text: '',
  modelParams: {} as Record<string, unknown>
});

const selectedSpeakerOption = ref<TextToSpeechSpeakerOption | null>(null);
const selectedLanguageOption = ref<{ label: string; value: AppLanguage } | null>(null);
const formatOptions = computed(() => TEXT_TO_SPEECH_FORMATS.map(option => ({ ...option, label: t(option.label) })));
const selectedFormatOption = ref<TextToSpeechOption | null>(formatOptions.value[0] ?? null);
const selectedDeviceOption = ref<{ label: string; value: string } | null>(null);
const isGenerating = ref(false);
const isCancelling = ref(false);
const isRefreshingHistory = ref(false);
const activeResult = ref<TtsResult | null>(null);
const generationHistory = ref<TtsResult[]>([]);
const selectedHistoryTaskId = ref<number | null>(null);
const showClearDialog = ref(false);
const resultCardRef = ref<InstanceType<typeof GeneratedAudioResultCard> | null>(null);
// 说话人选项按 baseModel 直连后端取数（后端过滤 ready + baseModel），
// 不再消费说话人管理页共享的分页/筛选状态
const speakers = ref<SpeakerProfile[]>([]);
const {
  getModelsByFeature,
  getModelVersionOptions,
  getSupportedDevices,
  getSupportedLanguages,
  getModelLabel
} = useModels();
const uiStore = useUiStore();
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

const trimmedText = computed(() => form.text.trim());
let isHistoryRefreshInFlight = false;
let activeTaskStatusTimer: ReturnType<typeof setInterval> | null = null;
let isActiveTaskRefreshInFlight = false;
// 单次刷新开始时间 + 代际令牌：用于检测被系统睡眠冻结的 invoke 并防止其晚到的
// finally 错误清掉新一次刷新的占用标志。见 refreshActiveTaskStatus。
let activeTaskRefreshStartedAt = 0;
let activeTaskRefreshGeneration = 0;
let skipHistoryTaskSelectionReload = false;

const isDynamicReferenceModel = computed(() => DYNAMIC_REFERENCE_BASE_MODELS.has(form.baseModel));
const dynamicRefAudioPath = computed(() => String(form.modelParams.refAudioPath ?? '').trim());
const dynamicRefTextPath = computed(() => String(form.modelParams.refTextPath ?? '').trim());

const modelOptions = computed(() =>
  getModelsByFeature(HistoryTaskType.TextToSpeech).map(item => ({
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
const activeTextToSpeechTaskConfig = computed(() => uiConfigStore.getTaskConfig(form.baseModel, HistoryTaskType.TextToSpeech));
const speakerOptions = computed<TextToSpeechSpeakerOption[]>(() => [
  {
    value: null,
    label: t('tts.speaker.autoSelect'),
    description: t('tts.speaker.autoSelectDesc')
  },
  ...speakers.value.map(speaker => ({
    value: speaker.id,
    label: speaker.speakerName,
    description: speaker.description || t('tts.speaker.noDescription')
  }))
]);
const charCount = computed(() => trimmedText.value.length);
const paragraphCount = computed(() => trimmedText.value.split(/\n+/).filter(Boolean).length || 0);
const canGenerate = computed(() => {
  const modelParamsValid = activeTextToSpeechTaskConfig.value
    ? uiConfigStore.validateModelParams(form.baseModel, HistoryTaskType.TextToSpeech, form.modelParams)
    : true;

  return (
    Boolean(form.language) &&
    charCount.value > 0 &&
    modelParamsValid &&
    !isGenerating.value &&
    !!form.modelVersion &&
    (!isDynamicReferenceModel.value || (Boolean(dynamicRefAudioPath.value) && Boolean(dynamicRefTextPath.value)))
  );
});
const canCancelActiveTask = computed(() => {
  const result = activeResult.value;
  if (!result) {
    return false;
  }

  return [TaskStatus.Pending, TaskStatus.Running].includes(result.status) && !isCancelling.value;
});
const generationTips = computed(() => [
  t('tts.summary.model', { model: getModelLabel(form.baseModel), version: form.modelVersion }),
  t('tts.summary.device', { device: HARDWARE_TYPE_TEXT[form.device as HardwareType] ?? form.device.toUpperCase() }),
  isDynamicReferenceModel.value
    ? t('tts.summary.dynamicReference')
    : t('tts.summary.speaker', { speaker: selectedSpeakerOption.value?.label ?? t('tts.summary.notSelected') }),
  t('tts.summary.chars', { chars: charCount.value, paragraphs: paragraphCount.value }),
  t('tts.summary.format', { format: selectedFormatOption.value?.label ?? form.format, name: form.exportAudioName })
]);
const activeResultMetaText = computed(() => {
  if (!activeResult.value) {
    return '';
  }

  return `${activeResult.value.speakerLabel} · ${getModelLabel(activeResult.value.baseModel)} · ${activeResult.value.modelVersion} · ${activeResult.value.languageLabel} · ${activeResult.value.formatLabel}`;
});
const recentTaskItems = computed<RecentTaskListItem[]>(() =>
  generationHistory.value.map(item => ({
    taskId: item.taskId,
    title: item.fileName,
    subtitle: t('tts.recent.subtitle', { taskId: item.taskId, speaker: item.speakerLabel, language: item.languageLabel }),
    status: item.status
  }))
);
const activeTaskBusyLabel = computed(() => {
  if (isCancelling.value) {
    return t('tts.busy.cancelling');
  }

  if (isCheckingDeviceType.value) {
    return t('tts.busy.checkingDevice');
  }

  if (isAwaitingDeviceConfirmation.value) {
    return t('tts.busy.awaitingDeviceConfirm');
  }

  if (isGenerating.value) {
    return t('tts.busy.creating');
  }

  if (activeResult.value?.status === TaskStatus.Pending || activeResult.value?.status === TaskStatus.Running) {
    return t('tts.busy.running');
  }

  return '';
});
const isSubmitPending = computed(() => isGenerating.value || isDeviceGuardPending.value);
const submitButtonText = computed(() => {
  if (isCheckingDeviceType.value) {
    return t('tts.submit.checking');
  }

  if (isAwaitingDeviceConfirmation.value) {
    return t('tts.submit.awaitingConfirm');
  }

  return isGenerating.value ? t('tts.submit.generating') : t('tts.submit.generate');
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
  deviceOptions,
  options => {
    if (options.length === 0) {
      form.device = HardwareType.Cpu;
      selectedDeviceOption.value = null;
      return;
    }

    const matched = options.find(option => option.value === form.device) ?? options[0] ?? null;
    form.device = (matched?.value ?? HardwareType.Cpu) as HardwareType;
    selectedDeviceOption.value = matched;
  },
  { immediate: true }
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
  speakerOptions,
  options => {
    if (options.length === 0) {
      form.speakerId = null;
      selectedSpeakerOption.value = null;
      return;
    }

    const matched = options.find(option => option.value === form.speakerId) ?? options[0] ?? null;
    form.speakerId = typeof matched?.value === 'number' ? matched.value : null;
    selectedSpeakerOption.value = matched;
  },
  { immediate: true }
);

watch(
  () => form.baseModel,
  nextBaseModel => {
    form.modelParams = normalizeTtsModelParams(nextBaseModel, form.modelParams);
  },
  { immediate: true }
);

const loadSpeakers = async () => {
  try {
    const result = await invoke<SpeakerPagedResult>('list_speaker_infos', {
      request: {
        page: 1,
        pageSize: 500,
        filter: { keyword: null, status: SpeakerStatus.Ready, baseModel: form.baseModel || null }
      }
    });
    speakers.value = Array.isArray(result?.items) ? result.items : [];
  } catch (error) {
    speakers.value = [];
    uiStore.notifyError(formatErrorMessage(t('common.store.speakers.loadFailed'), error));
  }
};

watch(
  () => form.baseModel,
  nextBaseModel => {
    if (nextBaseModel) {
      void loadSpeakers();
    } else {
      speakers.value = [];
    }
  },
  { immediate: true }
);

const stopActiveTaskStatusRefresh = () => {
  if (activeTaskStatusTimer) {
    clearInterval(activeTaskStatusTimer);
    activeTaskStatusTimer = null;
  }
};

const syncActiveTaskStatusRefresh = () => {
  stopActiveTaskStatusRefresh();

  if (
    !activeResult.value ||
    activeResult.value.status === TaskStatus.Completed ||
    activeResult.value.status === TaskStatus.Cancelled ||
    activeResult.value.status === TaskStatus.Failed
  ) {
    return;
  }

  activeTaskStatusTimer = setInterval(() => {
    void refreshActiveTaskStatus();
  }, 3000);
};

const findLanguageLabel = (language: AppLanguage) => APP_LANGUAGE_LABELS[language] ?? language;
const findFormatLabel = (format: TextToSpeechFormat) => {
  const found = TEXT_TO_SPEECH_FORMATS.find(option => option.value === format);
  return found ? t(found.label) : format;
};

const clearReplayTaskId = async () => {
  if (!(HISTORY_TASK_REPLAY_QUERY_KEY in route.query)) {
    return;
  }

  const nextQuery = { ...route.query };
  delete nextQuery[HISTORY_TASK_REPLAY_QUERY_KEY];
  await router.replace({ path: route.path, query: nextQuery });
};

const mapResultPayload = (payload: TextToSpeechTaskResultPayload): TtsResult => ({
  taskId: payload.taskId,
  fileName: payload.fileName,
  speakerId: payload.speakerId,
  speakerLabel: payload.speakerLabel,
  baseModel: payload.baseModel,
  modelVersion: payload.modelVersion,
  language: payload.language,
  languageLabel: findLanguageLabel(payload.language),
  format: payload.format,
  formatLabel: findFormatLabel(payload.format),
  exportAudioName: payload.exportAudioName,
  device: payload.device,
  durationSeconds: payload.durationSeconds,
  text: payload.text,
  modelParams: payload.modelParams,
  createdAt: payload.createdAt,
  status: payload.status,
  outputFilePath: payload.outputFilePath
});

const mapHistoryRecordToResult = (record: HistoryRecord): TtsResult | null => {
  if (record.taskType !== HistoryTaskType.TextToSpeech) {
    return null;
  }

  return {
    taskId: record.id,
    fileName: record.detail.fileName,
    speakerId: record.detail.speakerId,
    speakerLabel: record.speaker,
    baseModel: record.detail.baseModel,
    modelVersion: record.detail.modelVersion,
    language: record.detail.language,
    languageLabel: findLanguageLabel(record.detail.language),
    format: record.detail.format,
    formatLabel: findFormatLabel(record.detail.format),
    exportAudioName: record.detail.exportAudioName,
    device: record.device,
    durationSeconds: record.durationSeconds,
    text: record.detail.text,
    modelParams: record.detail.modelParams,
    createdAt: record.createTime,
    status: record.status,
    outputFilePath: record.detail.outputFilePath
  };
};

const applyResultToForm = (item: TtsResult, setAsActiveResult: boolean) => {
  stopActiveTaskStatusRefresh();
  activeResult.value = setAsActiveResult ? item : null;
  form.baseModel = item.baseModel;

  const matchedSpeakerOption = speakerOptions.value.find(option => option.value === item.speakerId) ?? null;
  form.speakerId = matchedSpeakerOption ? item.speakerId : null;
  form.modelVersion = item.modelVersion;
  form.language = item.language;
  form.format = item.format;
  form.device = item.device as HardwareType;
  form.exportAudioName = createDefaultExportAudioName();
  form.text = item.text;
  form.modelParams = normalizeTtsModelParams(item.baseModel, { ...item.modelParams });
  selectedSpeakerOption.value = matchedSpeakerOption;
  selectedLanguageOption.value = languageOptions.value.find(option => option.value === item.language) ?? null;
  selectedFormatOption.value = formatOptions.value.find(option => option.value === item.format) ?? null;
  selectedDeviceOption.value = deviceOptions.value.find(option => option.value === item.device) ?? null;

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

    if (record.taskType !== HistoryTaskType.TextToSpeech) {
      uiStore.notifyWarning(t('tts.notice.mismatchWarning'));
      return;
    }

    const result = mapHistoryRecordToResult(record);
    if (!result) {
      uiStore.notifyError(t('tts.notice.parseFailed'));
      return;
    }

    applyResultToForm(result, true);
    generationHistory.value = generationHistory.value.map(item => (item.taskId === result.taskId ? result : item));
    await resultCardRef.value?.refreshDetailRecord();
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('tts.notice.loadFailed'), error));
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

    if (record.taskType !== HistoryTaskType.TextToSpeech) {
      uiStore.notifyWarning(t('tts.notice.mismatchWarning'));
      return;
    }

    const result = mapHistoryRecordToResult(record);
    if (!result) {
      uiStore.notifyError(t('tts.notice.parseFailed'));
      return;
    }

    applyResultToForm(result, false);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('tts.notice.loadGenericFailed'), error));
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
    const records = await loadRecentHistoryRecords(HistoryTaskType.TextToSpeech, 5);
    generationHistory.value = records
      .map(mapHistoryRecordToResult)
      .filter((item): item is TtsResult => item !== null)
      .slice(0, 5);

    if (notifyOnSuccess) {
      uiStore.notifySuccess(t('tts.notice.statusRefreshed'), 2200);
    }
  } catch (error) {
    generationHistory.value = [];
    if (!silentOnError) {
      uiStore.notifyError(formatErrorMessage(t('tts.notice.refreshFailed'), error));
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
    const updated = mapHistoryRecordToResult(record);
    if (!updated || updated.taskId !== currentTaskId) {
      return;
    }

    activeResult.value = updated;
    generationHistory.value = generationHistory.value.map(item => (item.taskId === updated.taskId ? updated : item));
    syncActiveTaskStatusRefresh();
    await resultCardRef.value?.refreshDetailRecord();

    if (updated.status === TaskStatus.Completed || updated.status === TaskStatus.Failed) {
      stopActiveTaskStatusRefresh();
    }
  } catch (error) {
    console.log(formatErrorMessage(t('tts.notice.refreshCurrentFailed'), error));
  } finally {
    if (generation === activeTaskRefreshGeneration) {
      isActiveTaskRefreshInFlight = false;
      activeTaskRefreshStartedAt = 0;
    }
  }
};

const generateAudio = async () => {
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
  uiStore.notifyInfo(t('tts.notice.submitting'), 2200);

  try {
    const payload = await invoke<TextToSpeechTaskResultPayload>('create_text_to_speech_task', {
      payload: {
        speakerId: isDynamicReferenceModel.value ? null : form.speakerId,
        baseModel: form.baseModel,
        modelVersion: form.modelVersion,
        language: form.language,
        format: form.format,
        exportAudioName: form.exportAudioName,
        device: form.device,
        text: trimmedText.value,
        modelParams: form.modelParams
      }
    });
    const result = mapResultPayload(payload);

    activeResult.value = result;
    syncActiveTaskStatusRefresh();
    setSelectedHistoryTaskId(result.taskId, true);
    generationHistory.value = [result, ...generationHistory.value].slice(0, 5);
    uiStore.notifySuccess(t('tts.notice.submitted'));
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('tts.notice.generateFailed'), error));
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

const requestClearText = () => {
  const hasChanges =
    trimmedText.value ||
    form.speakerId ||
    form.baseModel ||
    form.language !== AppLanguage.Chinese ||
    form.format !== TextToSpeechFormat.Wav ||
    form.device !== HardwareType.Cpu ||
    JSON.stringify(form.modelParams) !== '{}';
  if (!hasChanges) {
    uiStore.notifyInfo(t('tts.notice.formDefault'), 2200);
    return;
  }

  showClearDialog.value = true;
};

const confirmClearText = () => {
  form.speakerId = null;
  form.language = AppLanguage.Chinese;
  form.format = TextToSpeechFormat.Wav;
  form.device = HardwareType.Cpu;
  form.exportAudioName = createDefaultExportAudioName();
  form.text = '';
  form.modelParams = {};
  selectedSpeakerOption.value = null;
  selectedLanguageOption.value = languageOptions.value[0] ?? null;
  selectedFormatOption.value = formatOptions.value[0] ?? null;
  selectedDeviceOption.value = deviceOptions.value.find(option => option.value === HardwareType.Cpu) ?? null;
  showClearDialog.value = false;
  uiStore.notifyInfo(t('tts.notice.formReset'), 2200);
};

const cancelClearText = () => {
  showClearDialog.value = false;
};

const copyTaskId = async () => {
  if (!activeResult.value) {
    uiStore.notifyWarning(t('tts.notice.noTaskId'));
    return;
  }

  try {
    await navigator.clipboard.writeText(String(activeResult.value.taskId));
    uiStore.notifySuccess(t('tts.notice.taskIdCopied', { taskId: activeResult.value.taskId }));
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('tts.notice.copyFailed'), error));
  }
};

const loadResultAudioAsset = (taskId: number) =>
  invoke<TextToSpeechAudioAssetPayload>('get_generated_audio', {
    source: { kind: 'text-to-speech', historyId: taskId }
  });

const saveResultAudio = (taskId: number) => saveGeneratedAudio({ kind: 'text-to-speech', historyId: taskId });

onBeforeUnmount(() => {
  stopActiveTaskStatusRefresh();
});

// 解除锁屏 / 唤醒后立即补一次刷新，避免 setInterval 被节流期间状态停滞。
usePollingResume(() => {
  void refreshActiveTaskStatus();
});

onMounted(async () => {
  await uiConfigStore.ensureLoaded();
  await loadRecentTasks();
  await hydrateReplayTaskFromRoute();
});
</script>

<template>
  <div class="space-y-5">
    <PageHeader :title="t('tts.title')" :description="t('tts.description')" eyebrow="Text-to-Speech" />

    <BaseLoadingBanner v-if="activeTaskBusyLabel" :label="activeTaskBusyLabel" />

    <div class="grid gap-5 xl:grid-cols-[1.2fr_1fr]">
      <PanelCard :title="t('tts.panels.basic')">
        <div class="grid gap-4 md:grid-cols-2">
          <BaseListbox
            v-model="form.speakerId"
            v-model:selected-option="selectedSpeakerOption"
            :label="t('tts.form.speaker')"
            :options="speakerOptions"
            :placeholder="t('tts.form.speakerPlaceholder')"
          />
          <BaseListbox v-model="form.language" v-model:selected-option="selectedLanguageOption" :label="t('tts.form.language')" :options="languageOptions" />
          <BaseListbox v-model="form.baseModel" :label="t('tts.form.baseModel')" :options="modelOptions" />
          <BaseListbox v-model="form.modelVersion" :label="t('tts.form.modelVersion')" :options="modelVersionOptions" :disabled="modelVersionOptions.length === 0" />
          <BaseListbox
            v-model="form.device"
            v-model:selected-option="selectedDeviceOption"
            :label="t('tts.form.deviceType')"
            :options="deviceOptions"
            :disabled="deviceOptions.length === 0"
          />
          <BaseListbox v-model="form.format" v-model:selected-option="selectedFormatOption" :label="t('tts.form.format')" :options="formatOptions" />
          <label class="block text-sm text-slate-700">
            <span class="mb-1 block text-xs text-stone-500">{{ t('tts.form.exportAudioName') }}</span>
            <input
              v-model="form.exportAudioName"
              class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
              :placeholder="t('tts.form.exportNamePlaceholder')"
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
          :empty-text="t('tts.result.emptyText')"
          @cancel="cancelActiveTask"
        >
          <template #details>
            <div v-if="activeResult" class="space-y-1">
              <p>{{ t('tts.result.taskId', { id: activeResult.taskId }) }}</p>
              <p>{{ t('tts.result.createdAt', { time: activeResult.createdAt }) }}</p>
              <p>{{ t('tts.result.exportName', { name: activeResult.exportAudioName }) }}</p>
              <p class="pt-1 line-clamp-4 text-slate-700">{{ activeResult.text }}</p>
            </div>
          </template>
          <template #actions>
            <BaseButton v-if="activeResult" tone="ghost" @click="copyTaskId">
              <ClipboardDocumentIcon class="h-4 w-4" aria-hidden="true" />
              <span>{{ t('tts.result.copyTaskId') }}</span>
            </BaseButton>
          </template>
        </GeneratedAudioResultCard>

        <PanelCard :title="t('tts.panels.recent')" :subtitle="t('tts.panels.recentSubtitle')">
          <template #actions>
            <BaseButton tone="ghost" size="sm" :loading="isRefreshingHistory" @click="loadRecentTasks({ notifyOnSuccess: true, manual: true })">
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

    <PanelCard :title="t('tts.panels.params')" :subtitle="t('tts.panels.paramsSubtitle')">
      <div class="space-y-5 text-sm text-slate-700">
        <label class="block text-sm text-slate-700">
          <span class="mb-1 block text-xs text-stone-500">{{ t('tts.form.inputText') }}</span>
          <textarea
            v-model="form.text"
            rows="8"
            class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2"
            :placeholder="t('tts.form.textPlaceholder')"
          />
          <div class="mt-2 flex flex-wrap items-center justify-between gap-2 text-xs text-stone-500">
            <span>{{ t('tts.form.charStats', { chars: charCount, paragraphs: paragraphCount }) }}</span>
          </div>
        </label>

        <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-base font-semibold tracking-tight text-slate-900">{{ t('tts.form.modelParams') }}</p>
          <GenericTaskParamsForm class="mt-4" v-model="form.modelParams" :task-config="activeTextToSpeechTaskConfig" />
        </section>

        <div class="grid gap-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(320px,0.9fr)]">
          <div class="rounded-2xl border border-brand-200 bg-white/80 p-4 text-xs text-stone-600">
            <p>{{ t('tts.form.summary') }}</p>
            <p v-for="tip in generationTips" :key="tip" class="mt-1">{{ tip }}</p>
          </div>

          <div class="rounded-2xl border border-brand-200 bg-brand-50/35 p-4">
            <div class="flex flex-wrap items-center justify-center gap-2">
              <BaseButton :loading="isSubmitPending" :disabled="!canGenerate || isSubmitPending" @click="generateAudio">
                <SparklesIcon v-if="!isSubmitPending" class="h-4 w-4" aria-hidden="true" />
                <span>{{ submitButtonText }}</span>
              </BaseButton>
              <BaseButton tone="quiet" :loading="isCancelling" :disabled="!canCancelActiveTask" @click="cancelActiveTask">
                <StopCircleIcon v-if="!isCancelling" class="h-4 w-4" aria-hidden="true" />
                <span>{{ isCancelling ? t('tts.form.cancelling') : t('tts.form.cancel') }}</span>
              </BaseButton>
              <BaseButton tone="ghost" @click="requestClearText">
                <ArrowPathIcon class="h-4 w-4" aria-hidden="true" />
                <span>{{ t('tts.form.resetForm') }}</span>
              </BaseButton>
            </div>
          </div>
        </div>
      </div>
    </PanelCard>

    <BaseDialog :open="showClearDialog" :title="t('tts.dialog.resetTitle')" @close="cancelClearText">
      <p class="text-sm leading-6 text-slate-700">{{ t('tts.dialog.resetBody') }}</p>
      <template #footer>
        <BaseButton tone="ghost" @click="cancelClearText">
          <span>{{ t('common.cancel') }}</span>
        </BaseButton>
        <BaseButton @click="confirmClearText">
          <span>{{ t('tts.dialog.confirmReset') }}</span>
        </BaseButton>
      </template>
    </BaseDialog>

    <WarningConfirmDialog
      :open="showDeviceMismatchDialog"
      :title="deviceMismatchDialogTitle"
      :message="deviceMismatchDialogMessage"
      :detail-lines="deviceMismatchDialogDetails"
      @close="closeDeviceMismatchDialog"
      @confirm="confirmDeviceMismatchDialog"
    />
  </div>
</template>

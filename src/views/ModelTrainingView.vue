<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  ArrowDownTrayIcon,
  ArrowPathIcon,
  ArrowUpTrayIcon,
  CheckCircleIcon,
  CpuChipIcon,
  EyeIcon,
  StopCircleIcon,
  TrashIcon,
  ArchiveBoxArrowDownIcon
} from '@heroicons/vue/24/outline';
import { useRoute, useRouter } from 'vue-router';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseLoadingBanner from '@/components/common/BaseLoadingBanner.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import StatusPill from '@/components/common/StatusPill.vue';
import RecentTaskList, { type RecentTaskListItem } from '@/components/common/RecentTaskList.vue';
import WarningConfirmDialog from '@/components/common/WarningConfirmDialog.vue';
import GenericTaskParamsForm from '@/components/form/GenericTaskParamsForm.vue';
import ModelTrainingTemplateDownloadDialog from '@/components/form/ModelTrainingTemplateDownloadDialog.vue';
import HistoryTaskDetailDialog from '@/components/history/HistoryTaskDetailDialog.vue';
import { AppLanguage, APP_LANGUAGE_SHORT_LABELS } from '@/enums/language';
import { HARDWARE_TYPE_TEXT, HardwareType } from '@/enums/settings';
import {
  MODEL_TRAINING_ANNOTATION_FILE_EXTENSIONS,
  MODEL_TRAINING_ANNOTATION_FORMAT_TEXT,
  MODEL_TRAINING_AUDIO_FILE_EXTENSIONS,
  MODEL_TRAINING_SAMPLE_TYPE_TEXT_KEY,
  ModelTrainingAnnotationFormat,
  ModelTrainingSampleType,
  type ModelTrainingOption
} from '@/enums/modelTraining';
import { TaskStatus } from '@/enums/status';
import { getHistoryTaskReplayId, HISTORY_TASK_REPLAY_QUERY_KEY, HistoryTaskType } from '@/enums/task';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { loadRecentHistoryRecords } from '@/hooks/loadRecentHistoryRecords';
import { usePollingResume } from '@/hooks/usePollingResume';
import { useTaskDeviceTypeGuard } from '@/hooks/useTaskDeviceTypeGuard';
import { useModelStore } from '@/stores/models';
import { useSpeakerStore } from '@/stores/speakers';
import { useUiConfigStore } from '@/stores/uiConfig';
import { useUiStore } from '@/stores/ui';
import type { HistoryRecord, ModelTrainingHistoryRecord, ModelTrainingSampleDetail } from '@/types/domain';
import { mergeModelParamsWithUiConfigDefaults } from '@/utils/uiConfigModelParams';

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
  modelVersion: string;
  speakerName: string;
  device: string;
  modelParams: Record<string, unknown>;
  sampleCount: number;
  createTime: string;
  status: TaskStatus;
}
const uiConfigStore = useUiConfigStore();

const normalizeTrainingModelParams = (baseModel: string, modelParams: Record<string, unknown>) => {
  const taskConfig = uiConfigStore.getTaskConfig(baseModel, HistoryTaskType.ModelTraining);
  return mergeModelParamsWithUiConfigDefaults(taskConfig, modelParams);
};

const form = reactive({
  language: AppLanguage.Chinese,
  baseModel: '',
  modelVersion: '',
  device: HardwareType.Cpu,
  speakerName: 'speaker_a_custom',
  description: '',
  modelParams: {} as Record<string, unknown>,
  singleAudioFile: null as SelectedLocalFile | null,
  singleTranscript: '',
  datasetArchiveFile: null as SelectedLocalFile | null,
  datasetAnnotationFile: null as SelectedLocalFile | null
});
const selectedLanguageOption = ref<ModelTrainingOption | null>(null);
const selectedDeviceOption = ref<{ label: string; value: string } | null>(null);
const isStarting = ref(false);
const isCancelling = ref(false);
const isRefreshingHistory = ref(false);
const activeTrainingTask = ref<ModelTrainingTaskResultPayload | null>(null);
const recentTrainingHistory = ref<ModelTrainingHistoryRecord[]>([]);
const selectedHistoryTaskId = ref<number | null>(null);
const isTemplateDialogOpen = ref(false);
const detailRecordId = ref<number | null>(null);
const detailReloadToken = ref(0);
const modelStore = useModelStore();
const speakerStore = useSpeakerStore();
const uiStore = useUiStore();
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
let importedSampleIdSeed = Date.now();
let activeTaskStatusTimer: ReturnType<typeof setInterval> | null = null;
let isActiveTaskRefreshInFlight = false;
// 单次刷新开始时间 + 代际令牌：用于检测被系统睡眠冻结的 invoke 并防止其晚到的
// finally 错误清掉新一次刷新的占用标志。见 refreshActiveTaskStatus。
let activeTaskRefreshStartedAt = 0;
let activeTaskRefreshGeneration = 0;
let isHistoryRefreshInFlight = false;
let skipHistoryTaskSelectionReload = false;
// 单次状态刷新被允许挂起的最长时间：超过即视为被系统睡眠冻结，重置占用标志恢复轮询。
const ACTIVE_TASK_REFRESH_STALE_MS = 15_000;

const trainingChecklist = computed(() => [
  t('training.checklist.item1'),
  t('training.checklist.item2'),
  t('training.checklist.item3')
]);

const importedSamples = ref<ImportedSampleItem[]>([]);
const modelOptions = computed(() =>
  modelStore.getModelsByFeature(HistoryTaskType.ModelTraining).map(item => ({
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
const languageOptions = computed(() =>
  modelStore.getSupportedLanguages(form.baseModel, form.modelVersion).map(language => ({
    value: language,
    label: APP_LANGUAGE_SHORT_LABELS[language] ?? language
  }))
);
const activeTrainingTaskConfig = computed(() => uiConfigStore.getTaskConfig(form.baseModel, HistoryTaskType.ModelTraining));

const singleImportReady = computed(() => Boolean(form.singleAudioFile) && form.singleTranscript.trim().length > 0);
const batchImportReady = computed(() => Boolean(form.datasetArchiveFile) && Boolean(form.datasetAnnotationFile));
const canStartTraining = computed(() => {
  const epochCount = Number(form.modelParams.epochCount ?? 0);
  const batchSize = Number(form.modelParams.batchSize ?? 0);
  const gradientAccumulationSteps = Number(form.modelParams.gradientAccumulationSteps ?? 0);

  // 判断模型特有参数是否正确填写
  const modelParamsValid = activeTrainingTaskConfig.value
    ? uiConfigStore.validateModelParams(form.baseModel, HistoryTaskType.ModelTraining, form.modelParams)
    : true;

  return (
    form.speakerName.trim().length > 0 &&
    form.description.trim().length > 0 &&
    importedSamples.value.length > 0 &&
    epochCount > 0 &&
    batchSize > 0 &&
    gradientAccumulationSteps > 0 &&
    modelParamsValid &&
    !isStarting.value &&
    !!form.modelVersion
  );
});

const sampleSummary = computed(() => ({
  total: importedSamples.value.length
}));
const recentTaskItems = computed<RecentTaskListItem[]>(() =>
  recentTrainingHistory.value.map(item => ({
    taskId: item.id,
    title: item.detail.speakerName,
    subtitle: t('training.recent.subtitle', { taskId: item.id, model: modelStore.getModelLabel(item.detail.baseModel) + ' ' + item.detail.modelVersion }),
    status: item.status
  }))
);
const selectedTrainingRecord = computed(() => recentTrainingHistory.value.find(item => item.id === selectedHistoryTaskId.value) ?? null);
const activeTrainingRecord = computed(() => recentTrainingHistory.value.find(item => item.id === activeTrainingTask.value?.taskId) ?? null);
const currentTrainingInfo = computed(() => {
  const record = selectedTrainingRecord.value ?? activeTrainingRecord.value ?? recentTrainingHistory.value[0] ?? null;

  if (record) {
    return {
      taskId: record.id,
      speakerName: record.detail.speakerName,
      description: record.detail.description?.trim() || t('training.info.notFilled'),
      baseModel: record.detail.baseModel,
      modelVersion: record.detail.modelVersion,
      device: record.device,
      sampleCount: record.detail.sampleCount,
      status: record.status,
      createTime: record.createTime
    };
  }

  if (!activeTrainingTask.value) {
    return null;
  }

  return {
    taskId: activeTrainingTask.value.taskId,
    speakerName: activeTrainingTask.value.speakerName,
    description: form.description.trim() || t('training.info.notFilled'),
    baseModel: activeTrainingTask.value.baseModel,
    modelVersion: activeTrainingTask.value.modelVersion,
    device: activeTrainingTask.value.device,
    sampleCount: activeTrainingTask.value.sampleCount,
    status: activeTrainingTask.value.status,
    createTime: activeTrainingTask.value.createTime
  };
});

const trainingBusyLabel = computed(() => {
  if (isCheckingDeviceType.value) {
    return t('training.busy.checkingDevice');
  }

  if (isAwaitingDeviceConfirmation.value) {
    return t('training.busy.awaitingDeviceConfirm');
  }

  if (isStarting.value) {
    return t('training.busy.creating');
  }

  if (isCancelling.value) {
    return t('training.busy.cancelling');
  }

  if (activeTrainingTask.value?.status === TaskStatus.Pending || activeTrainingTask.value?.status === TaskStatus.Running) {
    return t('training.busy.running');
  }

  return '';
});
const isSubmitPending = computed(() => isStarting.value || isDeviceGuardPending.value);
const submitButtonText = computed(() => {
  if (isCheckingDeviceType.value) {
    return t('training.submit.checking');
  }

  if (isAwaitingDeviceConfirmation.value) {
    return t('training.submit.awaitingConfirm');
  }

  return isStarting.value ? t('training.submit.creating') : t('training.submit.start');
});

const openCurrentTaskDetail = () => {
  if (!currentTrainingInfo.value) {
    return;
  }

  detailRecordId.value = currentTrainingInfo.value.taskId;
  detailReloadToken.value += 1;
};

const closeTaskDetail = () => {
  detailRecordId.value = null;
};

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
    modelVersion: trainingRecord.detail.modelVersion,
    speakerName: trainingRecord.detail.speakerName,
    device: trainingRecord.device,
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
    uiStore.notifyError(formatErrorMessage(t('training.notice.pickerFailed'), error));
    return null;
  }
};

const chooseSingleAudio = async () => {
  form.singleAudioFile = await selectLocalFile(t('training.dialog.selectAudio'), [...MODEL_TRAINING_AUDIO_FILE_EXTENSIONS], 'audio');
};

const chooseDatasetArchive = async () => {
  form.datasetArchiveFile = await selectLocalFile(t('training.dialog.selectZip'), ['zip'], 'archive');
};

const chooseDatasetAnnotation = async () => {
  form.datasetAnnotationFile = await selectLocalFile(t('training.dialog.selectAnnotation'), [...MODEL_TRAINING_ANNOTATION_FILE_EXTENSIONS], 'annotation');
};

const addSingleSample = () => {
  if (!singleImportReady.value || !form.singleAudioFile) {
    return;
  }

  importedSamples.value.unshift({
    id: nextImportedSampleId(),
    type: ModelTrainingSampleType.Single,
    title: form.singleAudioFile.fileName,
    detail: t('training.single.detail', { path: form.singleAudioFile.filePath }),
    transcriptPreview: form.singleTranscript.trim(),
    primaryFile: form.singleAudioFile
  });

  form.singleAudioFile = null;
  form.singleTranscript = '';
  uiStore.notifySuccess(t('training.single.added'), 2600);
};

const addDatasetSample = () => {
  if (!batchImportReady.value || !form.datasetArchiveFile || !form.datasetAnnotationFile) {
    return;
  }

  importedSamples.value.unshift({
    id: nextImportedSampleId(),
    type: ModelTrainingSampleType.Dataset,
    title: form.datasetArchiveFile.fileName,
    detail: t('training.dataset.detail', { path: form.datasetArchiveFile.filePath }),
    primaryFile: form.datasetArchiveFile,
    secondaryFile: form.datasetAnnotationFile
  });

  form.datasetArchiveFile = null;
  form.datasetAnnotationFile = null;
  uiStore.notifySuccess(t('training.dataset.added'), 2600);
};

const removeImportedSample = (sampleId: number) => {
  importedSamples.value = importedSamples.value.filter(sample => sample.id !== sampleId);
};

const resetForm = () => {
  form.language = AppLanguage.Chinese;
  form.baseModel = String(modelOptions.value[0]?.value ?? '');
  form.modelVersion = String(modelVersionOptions.value[0]?.value ?? '');
  form.device = HardwareType.Cpu;
  form.speakerName = 'speaker_a_custom';
  form.description = '';
  form.modelParams = normalizeTrainingModelParams(form.baseModel, {});
  form.singleAudioFile = null;
  form.singleTranscript = '';
  form.datasetArchiveFile = null;
  form.datasetAnnotationFile = null;
  selectedLanguageOption.value = languageOptions.value[0] ?? null;
  selectedDeviceOption.value = deviceOptions.value.find(option => option.value === HardwareType.Cpu) ?? null;
  importedSamples.value = [];
  uiStore.notifyInfo(t('training.notice.formReset'), 2200);
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
  form.modelVersion = record.detail.modelVersion;
  form.device = record.device as HardwareType;
  form.speakerName = record.detail.speakerName;
  form.description = record.detail.description ?? '';
  form.modelParams = normalizeTrainingModelParams(record.detail.baseModel, { ...record.detail.modelParams });
  form.singleAudioFile = null;
  form.singleTranscript = '';
  form.datasetArchiveFile = null;
  form.datasetAnnotationFile = null;
  importedSamples.value = record.detail.samples.map(mapHistorySampleToImportedSample);
  selectedLanguageOption.value = languageOptions.value.find(option => option.value === form.language) ?? null;
  selectedDeviceOption.value = deviceOptions.value.find(option => option.value === record.device) ?? null;
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

    if (!isModelTrainingHistoryRecord(record)) {
      uiStore.notifyWarning(t('tts.notice.mismatchWarning'));
      return;
    }

    applyTrainingHistoryToForm(record);
    activeTrainingTask.value = mapHistoryRecordToTrainingTask(record);
    recentTrainingHistory.value = recentTrainingHistory.value.map(item => (item.id === record.id ? record : item));
    syncActiveTaskStatusRefresh();
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('training.notice.loadFailed'), error));
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

    if (record.taskType !== HistoryTaskType.ModelTraining) {
      uiStore.notifyWarning(t('tts.notice.mismatchWarning'));
      return;
    }

    applyTrainingHistoryToForm(record as ModelTrainingHistoryRecord);
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
    const records = await loadRecentHistoryRecords(HistoryTaskType.ModelTraining, 5);
    recentTrainingHistory.value = records.filter(isModelTrainingHistoryRecord).slice(0, 5);

    if (notifyOnSuccess) {
      uiStore.notifySuccess(t('training.notice.statusRefreshed'), 2200);
    }
  } catch (error) {
    recentTrainingHistory.value = [];
    if (!silentOnError) {
      uiStore.notifyError(formatErrorMessage(t('training.notice.refreshFailed'), error));
    }
  } finally {
    isHistoryRefreshInFlight = false;
    if (manual) {
      isRefreshingHistory.value = false;
    }
  }
};

const syncSpeakerStore = async () => {
  await speakerStore.refreshSpeakers({ silent: true });
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
  const currentTaskId = activeTrainingTask.value.taskId;
  const previousStatus = activeTrainingTask.value.status;

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

    if (updatedTask.status !== previousStatus) {
      await syncSpeakerStore();
    }

    if (detailRecordId.value === currentTaskId) {
      detailReloadToken.value += 1;
    }

    if (updatedTask.status === TaskStatus.Completed || updatedTask.status === TaskStatus.Cancelled || updatedTask.status === TaskStatus.Failed) {
      stopActiveTaskStatusRefresh();
    }
  } catch (error) {
    console.log(formatErrorMessage(t('training.notice.refreshCurrentFailed'), error));
  } finally {
    if (generation === activeTaskRefreshGeneration) {
      isActiveTaskRefreshInFlight = false;
      activeTaskRefreshStartedAt = 0;
    }
  }
};

const cancelActiveTrainingTask = async () => {
  if (!activeTrainingTask.value || ![TaskStatus.Pending, TaskStatus.Running].includes(activeTrainingTask.value.status)) {
    return;
  }

  isCancelling.value = true;

  try {
    const accepted = await invoke<boolean>('cancel_history_task', {
      historyId: activeTrainingTask.value.taskId
    });

    if (!accepted) {
      uiStore.notifyWarning(t('tts.notice.alreadyCancelling'));
      return;
    }

    uiStore.notifyInfo(t('tts.notice.cancelRequested', { taskId: activeTrainingTask.value.taskId }), 3600);
    await refreshActiveTaskStatus();
    await loadRecentTasks({ silentOnError: true });
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('tts.notice.cancelFailed'), error));
  } finally {
    isCancelling.value = false;
  }
};

const cancelTrainingTask = async (historyId: number) => {
  if (!historyId) {
    return;
  }

  isCancelling.value = true;

  try {
    const accepted = await invoke<boolean>('cancel_history_task', {
      historyId
    });

    if (!accepted) {
      uiStore.notifyWarning(t('tts.notice.alreadyCancelling'));
      return;
    }

    if (activeTrainingTask.value?.taskId === historyId) {
      await refreshActiveTaskStatus();
    }

    await loadRecentTasks({ silentOnError: true });
    uiStore.notifyInfo(t('tts.notice.cancelRequested', { taskId: historyId }), 3600);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('tts.notice.cancelFailed'), error));
  } finally {
    isCancelling.value = false;
  }
};

const startTraining = async () => {
  if (!canStartTraining.value) {
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

  isStarting.value = true;
  uiStore.notifyInfo(t('training.notice.submitting'), 2200);

  try {
    const payload = await invoke<ModelTrainingTaskResultPayload>('create_model_training_task', {
      payload: {
        language: form.language,
        baseModel: form.baseModel,
        modelVersion: form.modelVersion,
        device: form.device,
        speakerName: form.speakerName.trim(),
        description: form.description.trim(),
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
    setSelectedHistoryTaskId(payload.taskId, true);
    await syncSpeakerStore();
    await loadRecentTasks({ silentOnError: true });

    uiStore.notifySuccess(
      t('training.notice.created', { speaker: payload.speakerName, taskId: payload.taskId, model: modelStore.getModelLabel(payload.baseModel), version: payload.modelVersion, count: payload.sampleCount }),
      5200
    );
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('training.notice.createFailed'), error));
  } finally {
    isStarting.value = false;
  }
};

onMounted(async () => {
  await uiConfigStore.ensureLoaded();
  await modelStore.ensureLoaded();
  await loadRecentTasks({ silentOnError: true });
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
    <PageHeader :title="t('training.title')" :description="t('training.description')" eyebrow="Model-Fine-Tuning" />

    <BaseLoadingBanner v-if="trainingBusyLabel" :label="trainingBusyLabel" />

    <div class="grid gap-5 xl:grid-cols-[1.2fr_0.8fr]">
      <PanelCard :title="t('training.panels.import')" :subtitle="t('training.panels.importSubtitle')">
        <div class="grid gap-4 xl:grid-cols-2">
          <section class="flex h-full flex-col rounded-2xl border border-brand-200 bg-white/80 p-4">
            <div class="mb-3">
              <span class="rounded-full bg-brand-100 px-2.5 py-1 text-xs font-medium text-brand-700">{{ t('training.single.badge') }}</span>
            </div>
            <div class="min-h-[68px]">
              <p class="text-sm font-semibold text-slate-800">{{ t('training.single.heading') }}</p>
              <p class="mt-1 text-xs leading-5 text-stone-500">{{ t('training.single.desc') }}</p>
            </div>

            <label class="mt-4 block text-sm text-slate-700">
              <span class="mb-1 block text-xs text-stone-500">{{ t('training.single.audio') }}</span>
              <div class="flex min-h-[120px] flex-col justify-between rounded-2xl border border-dashed border-brand-300 bg-brand-50/50 p-4">
                <BaseButton tone="ghost" @click="chooseSingleAudio">
                  <ArrowUpTrayIcon class="h-4 w-4" aria-hidden="true" />
                  <span>{{ t('training.single.selectAudio') }}</span>
                </BaseButton>
                <p class="mt-2 text-xs text-stone-500">{{ form.singleAudioFile?.fileName ?? t('training.single.noAudio') }}</p>
                <p v-if="form.singleAudioFile" class="mt-1 break-all text-[11px] text-stone-400">{{ form.singleAudioFile.filePath }}</p>
              </div>
            </label>

            <label class="mt-4 block text-sm text-slate-700">
              <span class="mb-1 block text-xs text-stone-500">{{ t('training.single.text') }}</span>
              <textarea
                v-model="form.singleTranscript"
                rows="5"
                class="min-h-[120px] w-full rounded-2xl border border-brand-200 bg-brand-50/35 px-3 py-2 text-sm text-slate-700"
                :placeholder="t('training.single.textPlaceholder')"
              />
            </label>

            <div class="mt-auto pt-4">
              <BaseButton block :disabled="!singleImportReady" @click="addSingleSample">
                <ArrowUpTrayIcon class="h-4 w-4" aria-hidden="true" />
                <span>{{ t('training.dataset.add') }}</span>
              </BaseButton>
            </div>
          </section>

          <section class="flex h-full flex-col rounded-2xl border border-brand-200 bg-white/80 p-4">
            <div class="mb-3">
              <span class="rounded-full bg-amber-100 px-2.5 py-1 text-xs font-medium text-amber-700">{{ t('training.dataset.badge') }}</span>
            </div>
            <div class="min-h-[68px]">
              <p class="text-sm font-semibold text-slate-800">{{ t('training.dataset.heading') }}</p>
              <p class="mt-1 text-xs leading-5 text-stone-500">{{ t('training.dataset.desc') }}</p>
            </div>

            <label class="mt-4 block text-sm text-slate-700">
              <span class="mb-1 block text-xs text-stone-500">{{ t('training.dataset.archive') }}</span>
              <div class="flex min-h-[120px] flex-col justify-between rounded-2xl border border-dashed border-brand-300 bg-brand-50/50 p-4">
                <BaseButton tone="ghost" @click="chooseDatasetArchive">
                  <ArchiveBoxArrowDownIcon class="h-4 w-4" aria-hidden="true" />
                  <span>{{ t('training.dataset.selectZip') }}</span>
                </BaseButton>
                <p class="mt-2 text-xs text-stone-500">{{ form.datasetArchiveFile?.fileName ?? t('training.dataset.noZip') }}</p>
                <p v-if="form.datasetArchiveFile" class="mt-1 break-all text-[11px] text-stone-400">{{ form.datasetArchiveFile.filePath }}</p>
              </div>
            </label>

            <label class="mt-4 block text-sm text-slate-700">
              <div class="mb-1 flex items-center justify-between gap-3 text-xs text-stone-500">
                <span>{{ t('training.dataset.annotation') }}</span>
                <BaseButton tone="quiet" size="sm" @click="isTemplateDialogOpen = true">
                  <ArrowDownTrayIcon class="h-4 w-4" aria-hidden="true" />
                  <span>{{ t('training.dataset.downloadTemplate') }}</span>
                </BaseButton>
              </div>
              <div class="flex min-h-[120px] flex-col justify-between rounded-2xl border border-dashed border-brand-300 bg-brand-50/50 p-4">
                <BaseButton tone="ghost" @click="chooseDatasetAnnotation">
                  <ArrowUpTrayIcon class="h-4 w-4" aria-hidden="true" />
                  <span>{{ t('training.dataset.selectAnnotation') }}</span>
                </BaseButton>
                <p class="mt-2 text-xs text-stone-500">{{ form.datasetAnnotationFile?.fileName ?? t('training.dataset.noAnnotation') }}</p>
                <p v-if="form.datasetAnnotationFile" class="mt-1 break-all text-[11px] text-stone-400">{{ form.datasetAnnotationFile.filePath }}</p>
                <p class="mt-2 text-[11px] text-stone-400">
                  {{ t('training.dataset.formatHint', { jsonl: MODEL_TRAINING_ANNOTATION_FORMAT_TEXT[ModelTrainingAnnotationFormat.Jsonl], xlsx: MODEL_TRAINING_ANNOTATION_FORMAT_TEXT[ModelTrainingAnnotationFormat.Xlsx], xls: MODEL_TRAINING_ANNOTATION_FORMAT_TEXT[ModelTrainingAnnotationFormat.Xls] }) }}
                </p>
              </div>
            </label>

            <div class="mt-auto pt-4">
              <BaseButton block :disabled="!batchImportReady" @click="addDatasetSample">
                <ArchiveBoxArrowDownIcon class="h-4 w-4" aria-hidden="true" />
                <span>{{ t('training.single.add') }}</span>
              </BaseButton>
            </div>
          </section>
        </div>

        <div class="mt-4 rounded-2xl border border-brand-200 bg-brand-50/40 p-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p class="text-sm font-semibold text-slate-800">{{ t('training.list.heading') }}</p>
              <p class="mt-1 text-xs text-stone-500">{{ t('training.list.summary', { total: sampleSummary.total }) }}</p>
            </div>
          </div>

          <div class="mt-4 max-h-[360px] overflow-y-auto pr-1">
            <ul v-if="importedSamples.length > 0" class="space-y-3">
              <li v-for="sample in importedSamples" :key="sample.id" class="rounded-xl border border-brand-200 bg-white/90 p-3">
                <div class="flex items-start justify-between gap-3">
                  <div>
                    <div class="flex items-center gap-2">
                      <p class="text-sm font-semibold text-slate-800">{{ sample.title }}</p>
                      <span class="rounded-full border border-brand-200 bg-brand-50 px-2 py-0.5 text-[11px] text-brand-700">
                        {{ t(MODEL_TRAINING_SAMPLE_TYPE_TEXT_KEY[sample.type]) }}
                      </span>
                    </div>
                    <p class="mt-1 text-xs text-stone-500">{{ sample.detail }}</p>
                    <p v-if="sample.transcriptPreview" class="mt-2 text-xs text-slate-600">{{ sample.transcriptPreview }}</p>
                  </div>
                  <BaseButton tone="quiet" size="sm" @click="removeImportedSample(sample.id)">
                    <TrashIcon class="h-4 w-4" aria-hidden="true" />
                    <span>{{ t('training.list.remove') }}</span>
                  </BaseButton>
                </div>
              </li>
            </ul>

            <div v-else class="rounded-xl border border-dashed border-brand-200 bg-white/80 p-4 text-xs text-stone-500">
              {{ t('training.list.empty') }}
            </div>
          </div>
        </div>
      </PanelCard>

      <div class="space-y-5">
        <PanelCard class="z-0" :title="t('training.panels.checklist')" :subtitle="t('training.panels.checklistSubtitle')">
          <ul class="space-y-2 text-sm text-slate-700">
            <li v-for="item in trainingChecklist" :key="item" class="flex gap-2">
              <CheckCircleIcon class="mt-0.5 h-4 w-4 shrink-0 text-brand-500" aria-hidden="true" />
              <span>{{ item }}</span>
            </li>
          </ul>
        </PanelCard>

        <PanelCard class="z-0" :title="t('training.panels.taskInfo')" :subtitle="t('training.panels.taskInfoSubtitle')">
          <div v-if="currentTrainingInfo" class="rounded-2xl border border-brand-200 bg-white/80 p-4 text-xs text-stone-600">
            <div class="flex items-start justify-between gap-3">
              <div>
                <p class="text-sm font-semibold text-slate-900">{{ currentTrainingInfo.speakerName }}</p>
                <p class="mt-1 text-xs text-stone-500">{{ t('training.info.taskMeta', { taskId: currentTrainingInfo.taskId, createTime: currentTrainingInfo.createTime }) }}</p>
              </div>
              <StatusPill :status="currentTrainingInfo.status" />
            </div>

            <div class="mt-4 space-y-2">
              <p>{{ t('training.info.speakerName', { name: currentTrainingInfo.speakerName }) }}</p>
              <p>{{ t('training.info.speakerDescription', { description: currentTrainingInfo.description }) }}</p>
              <p>{{ t('training.info.model', { model: modelStore.getModelLabel(currentTrainingInfo.baseModel), version: currentTrainingInfo.modelVersion }) }}</p>
              <p>{{ t('training.info.sampleCount', { count: currentTrainingInfo.sampleCount }) }}</p>
            </div>

            <div class="mt-4 flex justify-end">
              <BaseButton tone="ghost" size="sm" @click="openCurrentTaskDetail">
                <EyeIcon class="h-4 w-4" aria-hidden="true" />
                <span>{{ t('training.info.viewDetail') }}</span>
              </BaseButton>
            </div>
          </div>

          <div v-else class="rounded-2xl border border-dashed border-brand-200 bg-white/82 p-5 text-sm text-stone-500">
            {{ t('training.info.empty') }}
          </div>
        </PanelCard>

        <PanelCard class="z-0" :title="t('training.panels.recent')" :subtitle="t('training.panels.recentSubtitle')">
          <template #actions>
            <BaseButton tone="ghost" size="sm" :loading="isRefreshingHistory" @click="loadRecentTasks({ notifyOnSuccess: true, manual: true })">
              <ArrowPathIcon v-if="!isRefreshingHistory" class="h-4 w-4" aria-hidden="true" />
              <span>{{ isRefreshingHistory ? t('tts.form.refreshing') : t('tts.form.refresh') }}</span>
            </BaseButton>
          </template>

          <div class="max-h-[420px] overflow-y-auto pr-1">
            <RecentTaskList
              :items="recentTaskItems"
              v-model:selected-task-id="selectedHistoryTaskId"
              :empty-text="t('tts.result.historyEmptyText')"
              :action-label="t('tts.form.view')"
            />
          </div>
        </PanelCard>
      </div>
    </div>

    <PanelCard class="z-20" :title="t('training.panels.params')" :subtitle="t('training.panels.paramsSubtitle')">
      <div class="space-y-5 text-sm text-slate-700">
        <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
          <label class="block xl:col-span-1">
            <span class="mb-1 block text-xs text-stone-500">{{ t('training.form.speakerName') }}</span>
            <input
              v-model="form.speakerName"
              class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
              :placeholder="t('training.form.speakerNamePlaceholder')"
            />
          </label>
          <div class="xl:col-span-1">
            <BaseListbox v-model="form.baseModel" :label="t('tts.form.baseModel')" :options="modelOptions" />
          </div>
          <div class="xl:col-span-1">
            <BaseListbox v-model="form.modelVersion" :label="t('tts.form.modelVersion')" :options="modelVersionOptions" :disabled="modelVersionOptions.length === 0" />
          </div>
          <div class="xl:col-span-1">
            <BaseListbox
              v-model="form.device"
              v-model:selected-option="selectedDeviceOption"
              :label="t('tts.form.deviceType')"
              :options="deviceOptions"
              :disabled="deviceOptions.length === 0"
            />
          </div>
          <div class="md:col-span-2 xl:col-span-1">
            <BaseListbox
              v-model="form.language"
              v-model:selected-option="selectedLanguageOption"
              :label="t('training.form.language')"
              :options="languageOptions"
            />
          </div>
        </div>

        <label class="block">
          <span class="mb-1 block text-xs text-stone-500">{{ t('training.form.speakerDescription') }}</span>
          <textarea
            v-model="form.description"
            rows="2"
            class="min-h-[42px] w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
            :placeholder="t('training.form.speakerDescriptionPlaceholder')"
          />
        </label>

        <section class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-base font-semibold tracking-tight text-slate-900">{{ t('training.form.modelParams') }}</p>
          <p class="mt-1 text-xs leading-5 text-stone-500">{{ t('training.form.modelParamsHint') }}</p>
          <GenericTaskParamsForm class="mt-4" v-model="form.modelParams" :task-config="activeTrainingTaskConfig" />
        </section>

        <div class="grid gap-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(320px,0.9fr)]">
          <div class="rounded-2xl border border-brand-200 bg-white/80 p-4 text-xs text-stone-600">
            <p>{{ t('training.form.summary') }}</p>
            <p class="mt-1">{{ t('training.form.summaryData', { total: sampleSummary.total, language: selectedLanguageOption?.label ?? t('training.form.notSelected') }) }}</p>
            <p class="mt-1">{{ t('training.form.summaryModel', { model: modelStore.getModelLabel(form.baseModel), version: form.modelVersion }) }}</p>
            <p class="mt-1">{{ t('training.form.summaryDevice', { device: HARDWARE_TYPE_TEXT[form.device as HardwareType] ?? form.device.toUpperCase() }) }}</p>
            <p class="mt-1">{{ t('training.form.summaryDescription', { description: form.description.trim() || t('training.info.notFilled') }) }}</p>
            <p class="mt-1">{{ t('training.form.summaryBatch') }}</p>
            <p class="mt-1">{{ t('training.form.summaryGradAccum', { steps: form.modelParams.gradientAccumulationSteps ?? 0 }) }}</p>
          </div>

          <div class="rounded-2xl border border-brand-200 bg-brand-50/35 p-4">
            <div class="flex items-center justify-center gap-2">
              <BaseButton :loading="isSubmitPending" :disabled="!canStartTraining || isSubmitPending" @click="startTraining">
                <CpuChipIcon v-if="!isSubmitPending" class="h-4 w-4" aria-hidden="true" />
                <span>{{ submitButtonText }}</span>
              </BaseButton>
              <BaseButton
                tone="quiet"
                :loading="isCancelling"
                :disabled="isCancelling || !activeTrainingTask || ![TaskStatus.Pending, TaskStatus.Running].includes(activeTrainingTask.status)"
                @click="cancelActiveTrainingTask"
              >
                <StopCircleIcon v-if="!isCancelling" class="h-4 w-4" aria-hidden="true" />
                <span>{{ isCancelling ? t('tts.form.cancelling') : t('training.form.cancel') }}</span>
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

    <ModelTrainingTemplateDownloadDialog :open="isTemplateDialogOpen" @close="isTemplateDialogOpen = false" />
    <HistoryTaskDetailDialog
      :open="detailRecordId !== null"
      :record-id="detailRecordId"
      :reload-token="detailReloadToken"
      @close="closeTaskDetail"
      @cancel="cancelTrainingTask"
    />
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

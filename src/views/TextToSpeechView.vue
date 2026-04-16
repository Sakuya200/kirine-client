<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { ArrowPathIcon, ClipboardDocumentIcon, SparklesIcon, XMarkIcon } from '@heroicons/vue/24/outline';
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import AudioResultPlayer from '@/components/common/AudioResultPlayer.vue';
import BaseButton from '@/components/common/BaseButton.vue';
import BaseLoadingBanner from '@/components/common/BaseLoadingBanner.vue';
import BaseDialog from '@/components/common/BaseDialog.vue';
import BaseLoadingIndicator from '@/components/common/BaseLoadingIndicator.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import RecentTaskList, { type RecentTaskListItem } from '@/components/common/RecentTaskList.vue';
import StatusPill from '@/components/common/StatusPill.vue';
import Qwen3TtsTextToSpeechParamsForm from '@/components/qwen3_tts/Qwen3TtsTextToSpeechParamsForm.vue';
import { AppLanguage } from '@/enums/language';
import { TaskStatus } from '@/enums/status';
import { getHistoryTaskReplayId, HISTORY_TASK_REPLAY_QUERY_KEY, HistoryTaskType } from '@/enums/task';
import {
  TEXT_TO_SPEECH_FORMATS,
  TEXT_TO_SPEECH_LANGUAGES,
  TextToSpeechFormat,
  type TextToSpeechOption,
  type TextToSpeechSpeakerOption
} from '@/enums/textToSpeech';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useModelStore } from '@/stores/models';
import { useSpeakerStore } from '@/stores/speakers';
import { useUiStore } from '@/stores/ui';
import type { HistoryRecord } from '@/types/domain';

interface TtsResult {
  taskId: number;
  fileName: string;
  speakerId: number;
  speakerLabel: string;
  baseModel: string;
  modelScale: string;
  language: AppLanguage;
  languageLabel: string;
  format: TextToSpeechFormat;
  formatLabel: string;
  exportAudioName: string;
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
  speakerId: number;
  speakerLabel: string;
  baseModel: string;
  modelScale: string;
  language: AppLanguage;
  format: TextToSpeechFormat;
  exportAudioName: string;
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

const form = reactive({
  speakerId: null as number | null,
  baseModel: 'qwen3_tts',
  modelScale: '1.7B',
  language: AppLanguage.Chinese,
  format: TextToSpeechFormat.Wav,
  exportAudioName: 'kirine_tts',
  text: '',
  modelParams: {
    voicePrompt: ''
  } as Record<string, unknown>
});

const selectedSpeakerOption = ref<TextToSpeechSpeakerOption | null>(null);
const selectedLanguageOption = ref<TextToSpeechOption | null>(TEXT_TO_SPEECH_LANGUAGES[0]);
const selectedFormatOption = ref<TextToSpeechOption | null>(TEXT_TO_SPEECH_FORMATS[0]);
const isGenerating = ref(false);
const isRefreshingHistory = ref(false);
const activeResult = ref<TtsResult | null>(null);
const generationHistory = ref<TtsResult[]>([]);
const showClearDialog = ref(false);
const speakerStore = useSpeakerStore();
const modelStore = useModelStore();
const uiStore = useUiStore();
const route = useRoute();
const router = useRouter();

let isHistoryRefreshInFlight = false;
let activeTaskStatusTimer: ReturnType<typeof setInterval> | null = null;
let isActiveTaskRefreshInFlight = false;

const trimmedText = computed(() => form.text.trim());
const trimmedVoicePrompt = computed(() => String(form.modelParams.voicePrompt ?? '').trim());
const modelOptions = computed(() =>
  modelStore.getModelsByFeature(HistoryTaskType.TextToSpeech).map(item => ({
    label: item.modelName,
    value: item.baseModel
  }))
);
const modelScaleOptions = computed(() => modelStore.getModelScaleOptions(form.baseModel as never));
const speakerOptions = computed<TextToSpeechSpeakerOption[]>(() =>
  speakerStore.speakers
    .filter(speaker => speaker.status === 'ready' && speaker.baseModel === form.baseModel)
    .map(speaker => ({
      value: speaker.id,
      label: speaker.name,
      description: speaker.description || '该说话人暂无备注。'
    }))
);
const charCount = computed(() => trimmedText.value.length);
const paragraphCount = computed(() => trimmedText.value.split(/\n+/).filter(Boolean).length || 0);
const canGenerate = computed(
  () => form.speakerId !== null && Boolean(form.language) && charCount.value > 0 && !isGenerating.value && !!form.modelScale
);
const generationTips = computed(() => [
  `当前模型为 ${modelStore.getModelLabel(form.baseModel as never)} ${form.modelScale}。`,
  `当前字符数 ${charCount.value}，共 ${paragraphCount.value} 段。`,
  `输出格式为 ${selectedFormatOption.value?.label ?? form.format}，导出名称为 ${form.exportAudioName || 'kirine_tts'}。`,
  trimmedVoicePrompt.value ? `声音 Prompt：${trimmedVoicePrompt.value}` : '未填写声音 Prompt，将使用默认声音风格。'
]);
const recentTaskItems = computed<RecentTaskListItem[]>(() =>
  generationHistory.value.map(item => ({
    taskId: item.taskId,
    title: item.fileName,
    subtitle: `任务 ${item.taskId} · ${item.speakerLabel} · ${item.languageLabel}`,
    status: item.status
  }))
);
const activeTaskBusyLabel = computed(() => {
  if (isGenerating.value) {
    return '正在创建文本转语音任务，请稍候';
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
      form.baseModel = String(options[0]?.value ?? 'qwen3_tts');
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
  speakerOptions,
  options => {
    if (options.length === 0) {
      form.speakerId = null;
      selectedSpeakerOption.value = null;
      return;
    }

    const matched = options.find(option => option.value === form.speakerId) ?? options[0];
    form.speakerId = typeof matched.value === 'number' ? matched.value : Number(matched.value);
    selectedSpeakerOption.value = matched;
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

  if (!activeResult.value || activeResult.value.status === TaskStatus.Completed || activeResult.value.status === TaskStatus.Failed) {
    return;
  }

  activeTaskStatusTimer = setInterval(() => {
    void refreshActiveTaskStatus();
  }, 3000);
};

const findLanguageLabel = (language: AppLanguage) => TEXT_TO_SPEECH_LANGUAGES.find(option => option.value === language)?.label ?? language;
const findFormatLabel = (format: TextToSpeechFormat) => TEXT_TO_SPEECH_FORMATS.find(option => option.value === format)?.label ?? format;

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
  modelScale: payload.modelScale,
  language: payload.language,
  languageLabel: findLanguageLabel(payload.language),
  format: payload.format,
  formatLabel: findFormatLabel(payload.format),
  exportAudioName: payload.exportAudioName,
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
    modelScale: record.detail.modelScale,
    language: record.detail.language,
    languageLabel: findLanguageLabel(record.detail.language),
    format: record.detail.format,
    formatLabel: findFormatLabel(record.detail.format),
    exportAudioName: record.detail.exportAudioName,
    durationSeconds: record.durationSeconds,
    text: record.detail.text,
    modelParams: record.detail.modelParams,
    createdAt: record.createTime,
    status: record.status,
    outputFilePath: record.detail.outputFilePath
  };
};

const applyResultToForm = (item: TtsResult, setAsActiveResult: boolean) => {
  const matchedSpeakerOption = speakerOptions.value.find(option => option.value === item.speakerId) ?? null;

  stopActiveTaskStatusRefresh();
  activeResult.value = setAsActiveResult ? item : null;
  form.speakerId = matchedSpeakerOption ? item.speakerId : null;
  form.baseModel = item.baseModel;
  form.modelScale = item.modelScale;
  form.language = item.language;
  form.format = item.format;
  form.exportAudioName = item.exportAudioName;
  form.text = item.text;
  form.modelParams = { ...item.modelParams };
  selectedSpeakerOption.value = matchedSpeakerOption;
  selectedLanguageOption.value = TEXT_TO_SPEECH_LANGUAGES.find(option => option.value === item.language) ?? null;
  selectedFormatOption.value = TEXT_TO_SPEECH_FORMATS.find(option => option.value === item.format) ?? null;

  if (setAsActiveResult) {
    syncActiveTaskStatusRefresh();
  }
};

const hydrateReplayTaskFromRoute = async () => {
  const historyId = getHistoryTaskReplayId(route.query[HISTORY_TASK_REPLAY_QUERY_KEY]);

  if (historyId === null) {
    await clearReplayTaskId();
    return;
  }

  try {
    const record = await invoke<HistoryRecord>('get_history_record', { historyId });

    if (record.taskType !== HistoryTaskType.TextToSpeech) {
      uiStore.notifyWarning('目标历史任务与当前页面类型不匹配，无法载入配置。');
      return;
    }

    const result = mapHistoryRecordToResult(record);
    if (!result) {
      uiStore.notifyError('历史任务配置解析失败。');
      return;
    }

    applyResultToForm(result, false);
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
    generationHistory.value = records
      .map(mapHistoryRecordToResult)
      .filter((item): item is TtsResult => item !== null)
      .slice(0, 5);

    if (notifyOnSuccess) {
      uiStore.notifySuccess('文本转语音任务状态已刷新。', 2200);
    }
  } catch (error) {
    generationHistory.value = [];
    if (!silentOnError) {
      uiStore.notifyError(formatErrorMessage('刷新文本转语音历史任务失败，请检查 Rust 后端日志', error));
    }
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
    const updated = mapHistoryRecordToResult(record);
    if (!updated || updated.taskId !== currentTaskId) {
      return;
    }

    activeResult.value = updated;
    generationHistory.value = generationHistory.value.map(item => (item.taskId === updated.taskId ? updated : item));
    syncActiveTaskStatusRefresh();

    if (updated.status === TaskStatus.Completed || updated.status === TaskStatus.Failed) {
      stopActiveTaskStatusRefresh();
    }
  } catch (error) {
    console.log(formatErrorMessage('刷新当前任务状态失败，请检查 Rust 后端日志', error));
  } finally {
    isActiveTaskRefreshInFlight = false;
  }
};

const generateAudio = async () => {
  if (!canGenerate.value || form.speakerId === null) {
    return;
  }

  isGenerating.value = true;
  uiStore.notifyInfo('正在提交生成任务。', 2200);

  try {
    const payload = await invoke<TextToSpeechTaskResultPayload>('create_text_to_speech_task', {
      payload: {
        speakerId: form.speakerId,
        baseModel: form.baseModel,
        modelScale: form.modelScale,
        language: form.language,
        format: form.format,
        exportAudioName: form.exportAudioName,
        text: trimmedText.value,
        modelParams: form.modelParams
      }
    });
    const result = mapResultPayload(payload);

    activeResult.value = result;
    syncActiveTaskStatusRefresh();
    generationHistory.value = [result, ...generationHistory.value].slice(0, 5);
    uiStore.notifySuccess('任务已提交，可在历史记录中查看执行状态与结果。');
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('生成失败，请检查 Rust 后端日志', error));
  } finally {
    isGenerating.value = false;
  }
};

const requestClearText = () => {
  if (!trimmedText.value) {
    form.text = '';
    uiStore.notifyInfo('输入文本已清空。', 2200);
    return;
  }

  showClearDialog.value = true;
};

const confirmClearText = () => {
  form.text = '';
  showClearDialog.value = false;
  uiStore.notifyInfo('输入文本已清空。', 2200);
};

const cancelClearText = () => {
  showClearDialog.value = false;
};

const copyTaskId = async () => {
  if (!activeResult.value) {
    uiStore.notifyWarning('没有可复制的任务 ID。');
    return;
  }

  try {
    await navigator.clipboard.writeText(String(activeResult.value.taskId));
    uiStore.notifySuccess(`任务 ID 已复制：${activeResult.value.taskId}`);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('复制失败，当前环境未开放剪贴板权限', error));
  }
};

const loadResultAudioAsset = (taskId: number) =>
  invoke<TextToSpeechAudioAssetPayload>('get_text_to_speech_audio', {
    historyId: taskId
  });

const saveResultAudio = (taskId: number) =>
  invoke<boolean>('save_text_to_speech_audio_as', {
    historyId: taskId
  });

const loadHistoryItem = (taskId: number) => {
  const item = generationHistory.value.find(historyItem => historyItem.taskId === taskId);
  if (!item) {
    uiStore.notifyWarning('目标历史任务不存在或已被移除。');
    return;
  }

  applyResultToForm(item, true);
};

onBeforeUnmount(() => {
  stopActiveTaskStatusRefresh();
});

onMounted(async () => {
  await modelStore.ensureLoaded();
  if (!speakerStore.initialized) {
    await speakerStore.loadSpeakers();
  }
  await loadRecentTasks();
  await hydrateReplayTaskFromRoute();
});
</script>

<template>
  <div class="space-y-5">
    <PageHeader title="文本转语音" description="选择说话人和模型，输入文本并配置模型参数，生成目标音频。" eyebrow="Text-to-Speech" />

    <BaseLoadingBanner v-if="activeTaskBusyLabel" :label="activeTaskBusyLabel" />

    <div class="grid gap-5 xl:grid-cols-[1.2fr_1fr]">
      <PanelCard title="基础参数">
        <div class="grid gap-4 md:grid-cols-2">
          <BaseListbox
            v-model="form.speakerId"
            v-model:selected-option="selectedSpeakerOption"
            label="说话人"
            :options="speakerOptions"
            :placeholder="speakerOptions.length > 0 ? '请选择说话人' : '暂无可用说话人'"
          />
          <BaseListbox v-model="form.language" v-model:selected-option="selectedLanguageOption" label="语言" :options="TEXT_TO_SPEECH_LANGUAGES" />
          <BaseListbox v-model="form.baseModel" label="基础模型" :options="modelOptions" />
          <BaseListbox v-model="form.modelScale" label="模型大小" :options="modelScaleOptions" :disabled="modelScaleOptions.length === 0" />
          <BaseListbox v-model="form.format" v-model:selected-option="selectedFormatOption" label="输出格式" :options="TEXT_TO_SPEECH_FORMATS" />
          <label class="block text-sm text-slate-700">
            <span class="mb-1 block text-xs text-stone-500">导出音频名称</span>
            <input
              v-model="form.exportAudioName"
              class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
              placeholder="例如 news_broadcast"
            />
          </label>
        </div>

        <div class="mt-4">
          <label class="block text-sm text-slate-700">
            <span class="mb-1 block text-xs text-stone-500">输入文本</span>
            <textarea
              v-model="form.text"
              rows="8"
              class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2"
              placeholder="请输入要合成的文本内容..."
            />
            <div class="mt-2 flex flex-wrap items-center justify-between gap-2 text-xs text-stone-500">
              <span>字符数 {{ charCount }}，段落数 {{ paragraphCount }}</span>
            </div>
          </label>
        </div>

        <div class="mt-4">
          <p class="text-base font-semibold tracking-tight text-slate-900">模型特定参数</p>
          <Qwen3TtsTextToSpeechParamsForm class="mt-4" v-model="form.modelParams" />
        </div>

        <div class="mt-4 rounded-2xl border border-brand-200 bg-brand-50/40 p-4 text-xs text-stone-600">
          <p class="font-semibold text-slate-700">生成摘要</p>
          <ul class="mt-2 space-y-1.5">
            <li v-for="tip in generationTips" :key="tip">{{ tip }}</li>
          </ul>
        </div>

        <div class="mt-4 flex flex-wrap gap-2">
          <BaseButton :disabled="!canGenerate" @click="generateAudio">
            <BaseLoadingIndicator v-if="isGenerating" size="sm" tone="muted" />
            <SparklesIcon v-else class="h-4 w-4" aria-hidden="true" />
            <span>{{ isGenerating ? '生成中...' : '生成音频' }}</span>
          </BaseButton>
          <BaseButton tone="ghost" @click="requestClearText">
            <XMarkIcon class="h-4 w-4" aria-hidden="true" />
            <span>清空文本</span>
          </BaseButton>
        </div>
      </PanelCard>

      <div class="space-y-5">
        <PanelCard title="生成结果" subtitle="展示最近一次文本转语音任务的返回结果和输出文件信息">
          <div v-if="activeResult" class="surface-grid rounded-2xl border border-brand-200 bg-white/82 p-4">
            <div class="flex items-start justify-between gap-3">
              <div>
                <p class="text-sm font-medium text-slate-700">{{ activeResult.fileName }}</p>
                <p class="mt-1 text-xs text-stone-500">
                  {{ activeResult.speakerLabel }} · {{ modelStore.getModelLabel(activeResult.baseModel as never) }} · {{ activeResult.modelScale }} ·
                  {{ activeResult.languageLabel }} · {{ activeResult.formatLabel }}
                </p>
              </div>
              <StatusPill :status="activeResult.status" />
            </div>

            <div class="mt-3">
              <AudioResultPlayer
                :task="activeResult"
                :load-audio-asset="loadResultAudioAsset"
                :download-audio="saveResultAudio"
                download-label="下载"
                download-tone="ghost"
              />
            </div>

            <div class="mt-3 rounded-2xl border border-brand-200 bg-white/80 p-3 text-xs text-stone-600">
              <p>任务 ID：{{ activeResult.taskId }}</p>
              <p class="mt-1">生成时间：{{ activeResult.createdAt }}</p>
              <p class="mt-1">导出名称：{{ activeResult.exportAudioName }}</p>
              <p v-if="activeResult.modelParams.voicePrompt" class="mt-1">声音 Prompt：{{ activeResult.modelParams.voicePrompt }}</p>
              <p class="mt-2 line-clamp-4 text-slate-700">{{ activeResult.text }}</p>
            </div>

            <div class="mt-4 flex flex-wrap gap-2">
              <BaseButton tone="ghost" @click="copyTaskId">
                <ClipboardDocumentIcon class="h-4 w-4" aria-hidden="true" />
                <span>复制任务ID</span>
              </BaseButton>
            </div>
          </div>

          <div v-else class="rounded-2xl border border-dashed border-brand-200 bg-white/82 p-5 text-sm text-stone-500">
            还没有生成结果。完成文本输入并点击“生成音频”后，结果会显示在这里。
          </div>
        </PanelCard>

        <PanelCard title="最近任务" subtitle="展示最近 5 条文本转语音任务，数据来自统一历史记录">
          <template #actions>
            <BaseButton tone="ghost" size="sm" :disabled="isRefreshingHistory" @click="loadRecentTasks({ notifyOnSuccess: true, manual: true })">
              <BaseLoadingIndicator v-if="isRefreshingHistory" size="sm" tone="muted" />
              <ArrowPathIcon v-else class="h-4 w-4" aria-hidden="true" />
              <span>{{ isRefreshingHistory ? '刷新中...' : '刷新状态' }}</span>
            </BaseButton>
          </template>

          <RecentTaskList
            :items="recentTaskItems"
            :selected-task-id="activeResult?.taskId ?? null"
            empty-text="还没有历史任务。生成音频后会自动加入这里。"
            action-label="查看"
            @select="loadHistoryItem"
          />
        </PanelCard>
      </div>
    </div>

    <BaseDialog :open="showClearDialog" title="清空文本" @close="cancelClearText">
      <p class="text-sm leading-6 text-slate-700">当前输入的文本会被清空，但不会删除已经生成的结果记录。确定继续吗？</p>
      <template #footer>
        <BaseButton tone="ghost" @click="cancelClearText">
          <span>取消</span>
        </BaseButton>
        <BaseButton @click="confirmClearText">
          <span>确认清空</span>
        </BaseButton>
      </template>
    </BaseDialog>
  </div>
</template>

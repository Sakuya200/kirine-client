<script setup lang="ts">
import { ArrowDownTrayIcon, PauseIcon, PlayIcon } from '@heroicons/vue/24/outline';
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

import BaseButton from '@/components/common/BaseButton.vue';
import { TaskStatus } from '@/enums/status';
import { useTaskAudioPlayer } from '@/hooks/useTaskAudioPlayer';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useUiStore } from '@/stores/ui';
import { formatDurationClock } from '@/utils/formatDurationClock';

interface AudioAssetPayload {
  fileName: string;
  contentType: string;
  bytes: number[];
}

interface AudioPlayerTask {
  taskId: number;
  status: TaskStatus;
}

interface Props {
  task: AudioPlayerTask | null;
  loadAudioAsset: (taskId: number) => Promise<AudioAssetPayload>;
  downloadAudio?: (taskId: number) => Promise<boolean>;
  showDownload?: boolean;
  downloadLabel?: string;
  pendingMessage?: string;
  failedMessage?: string;
  cancelledMessage?: string;
  downloadTone?: 'solid' | 'ghost' | 'quiet';
}

const { t } = useI18n();

const props = withDefaults(defineProps<Props>(), {
  showDownload: true,
  downloadLabel: undefined,
  pendingMessage: undefined,
  failedMessage: undefined,
  cancelledMessage: undefined,
  downloadTone: 'ghost'
});

const uiStore = useUiStore();
const isDownloading = ref(false);

const { isPlaying, playbackProgress, currentPlaybackSeconds, playbackTotalSeconds, togglePlayback, resetPlayback } =
  useTaskAudioPlayer<AudioPlayerTask>({
    loadAudioAsset: taskId => props.loadAudioAsset(taskId),
    onPlaybackEnded: () => {
      uiStore.notifyInfo(t('common.audio.ended'), 2200);
    },
    onPlaybackError: () => {
      uiStore.notifyError(t('common.audio.playFailed'));
    },
    onPlayFailed: error => {
      uiStore.notifyError(formatErrorMessage(t('common.audio.playBlocked'), error));
    }
  });

const playbackActionLabel = computed(() => (isPlaying.value ? t('common.audio.pause') : t('common.audio.play')));
const resolvedTotalSeconds = computed(() => playbackTotalSeconds.value);

// 非完成态的占位提示按状态区分：失败/终止属于终态，不能再提示"仍在执行中"。
const statusMessage = computed(() => {
  if (props.task?.status === TaskStatus.Failed) {
    return props.failedMessage ?? t('common.audio.failedMessage');
  }
  if (props.task?.status === TaskStatus.Cancelled) {
    return props.cancelledMessage ?? t('common.audio.cancelledMessage');
  }
  return props.pendingMessage ?? t('common.audio.pendingMessage');
});

const statusMessageTone = computed(() => {
  if (props.task?.status === TaskStatus.Failed) {
    return 'border-rose-200 bg-rose-50/60 text-rose-600';
  }
  return 'border-brand-200 bg-white/85 text-stone-600';
});

const handleTogglePlayback = () => {
  if (!props.task) {
    uiStore.notifyWarning(t('common.audio.noResult'));
    return;
  }

  if (props.task.status !== TaskStatus.Completed) {
    uiStore.notifyInfo(t('common.audio.notCompleted'), 2600);
    return;
  }

  const didStartPlayback = togglePlayback(props.task);
  if (!didStartPlayback && isPlaying.value === false) {
    uiStore.notifyInfo(t('common.audio.paused'), 2200);
  }
};

const handleDownload = async () => {
  if (!props.task || !props.downloadAudio) {
    uiStore.notifyWarning(t('common.audio.noDownload'));
    return;
  }

  if (props.task.status !== TaskStatus.Completed) {
    uiStore.notifyInfo(t('common.audio.notCompletedDownload'), 3200);
    return;
  }

  isDownloading.value = true;

  try {
    const saved = await props.downloadAudio(props.task.taskId);
    if (!saved) {
      uiStore.notifyInfo(t('common.audio.downloadCancelled'), 2200);
      return;
    }

    uiStore.notifySuccess(t('common.audio.saved'), 3200);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('common.audio.downloadFailed'), error));
  } finally {
    isDownloading.value = false;
  }
};

watch(
  () => props.task?.taskId ?? null,
  () => {
    resetPlayback({ releaseSource: true });
  }
);
</script>

<template>
  <div v-if="task?.status === TaskStatus.Completed" class="space-y-3 rounded-2xl border border-brand-200 bg-white/85 p-4">
    <div class="flex items-center justify-between gap-3 text-sm text-slate-600">
      <span>{{ formatDurationClock(currentPlaybackSeconds) }}</span>
      <span>{{ formatDurationClock(resolvedTotalSeconds) }}</span>
    </div>
    <div class="h-2 overflow-hidden rounded-full bg-brand-100">
      <div class="h-full rounded-full bg-brand-500 transition-all duration-200" :style="{ width: `${playbackProgress}%` }" />
    </div>
    <div class="flex flex-wrap gap-3">
      <BaseButton tone="ghost" @click="handleTogglePlayback">
        <component :is="isPlaying ? PauseIcon : PlayIcon" class="h-4 w-4" aria-hidden="true" />
        <span>{{ playbackActionLabel }}</span>
      </BaseButton>
      <BaseButton v-if="showDownload" :tone="downloadTone" :loading="isDownloading" :disabled="!downloadAudio" @click="handleDownload">
        <ArrowDownTrayIcon v-if="!isDownloading" class="h-4 w-4" aria-hidden="true" />
        <span>{{ isDownloading ? t('common.loading') : (downloadLabel ?? t('common.audio.downloadLabel')) }}</span>
      </BaseButton>
    </div>
  </div>

  <div v-else class="rounded-2xl border p-4 text-sm leading-6" :class="statusMessageTone">
    {{ statusMessage }}
  </div>
</template>

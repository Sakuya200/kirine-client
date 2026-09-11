<script setup lang="ts">
import { ArrowDownTrayIcon, PauseIcon, PlayIcon } from '@heroicons/vue/24/outline';
import { computed, ref, watch } from 'vue';

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

const props = withDefaults(defineProps<Props>(), {
  showDownload: true,
  downloadLabel: '下载音频',
  pendingMessage: '任务仍在执行中，音频结果会在状态变为“已完成”后显示。',
  failedMessage: '任务执行失败，无音频结果。可在历史任务详情中查看「任务日志」了解失败原因。',
  cancelledMessage: '任务已终止，无音频结果。',
  downloadTone: 'ghost'
});

const uiStore = useUiStore();
const isDownloading = ref(false);

const { isPlaying, playbackProgress, currentPlaybackSeconds, playbackTotalSeconds, togglePlayback, resetPlayback } =
  useTaskAudioPlayer<AudioPlayerTask>({
    loadAudioAsset: taskId => props.loadAudioAsset(taskId),
    onPlaybackEnded: () => {
      uiStore.notifyInfo('音频播放结束。', 2200);
    },
    onPlaybackError: () => {
      uiStore.notifyError('音频播放失败，请检查音频文件是否仍然可读。');
    },
    onPlayFailed: error => {
      uiStore.notifyError(formatErrorMessage('音频播放失败，当前环境可能阻止了播放', error));
    }
  });

const playbackActionLabel = computed(() => (isPlaying.value ? '暂停播放' : '播放音频'));
const resolvedTotalSeconds = computed(() => playbackTotalSeconds.value);

// 非完成态的占位提示按状态区分：失败/终止属于终态，不能再提示"仍在执行中"。
const statusMessage = computed(() => {
  if (props.task?.status === TaskStatus.Failed) {
    return props.failedMessage;
  }
  if (props.task?.status === TaskStatus.Cancelled) {
    return props.cancelledMessage;
  }
  return props.pendingMessage;
});

const statusMessageTone = computed(() => {
  if (props.task?.status === TaskStatus.Failed) {
    return 'border-rose-200 bg-rose-50/60 text-rose-600';
  }
  return 'border-brand-200 bg-white/85 text-stone-600';
});

const handleTogglePlayback = () => {
  if (!props.task) {
    uiStore.notifyWarning('当前还没有可播放的音频结果。');
    return;
  }

  if (props.task.status !== TaskStatus.Completed) {
    uiStore.notifyInfo('当前任务尚未完成，完成后才可播放音频。', 2600);
    return;
  }

  const didStartPlayback = togglePlayback(props.task);
  if (!didStartPlayback && isPlaying.value === false) {
    uiStore.notifyInfo('已暂停音频播放。', 2200);
  }
};

const handleDownload = async () => {
  if (!props.task || !props.downloadAudio) {
    uiStore.notifyWarning('当前没有可下载的音频结果。');
    return;
  }

  if (props.task.status !== TaskStatus.Completed) {
    uiStore.notifyInfo('当前任务尚未完成，请等待状态更新后再下载。', 3200);
    return;
  }

  isDownloading.value = true;

  try {
    const saved = await props.downloadAudio(props.task.taskId);
    if (!saved) {
      uiStore.notifyInfo('已取消下载。', 2200);
      return;
    }

    uiStore.notifySuccess('音频已保存。', 3200);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('下载失败，请检查系统保存对话框权限和输出文件状态', error));
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
        <span>{{ isDownloading ? '处理中...' : downloadLabel }}</span>
      </BaseButton>
    </div>
  </div>

  <div v-else class="rounded-2xl border p-4 text-sm leading-6" :class="statusMessageTone">
    {{ statusMessage }}
  </div>
</template>

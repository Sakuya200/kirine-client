<script setup lang="ts">
import { ArrowDownTrayIcon, PauseIcon, PlayIcon } from '@heroicons/vue/24/outline';
import { computed, ref, watch } from 'vue';

import BaseButton from '@/components/common/BaseButton.vue';
import { useStreamableAudioPlayer } from '@/hooks/useStreamableAudioPlayer';
import { useStreamingSpeechStore } from '@/stores/streamingSpeech';
import { useUiStore } from '@/stores/ui';

interface Props {
  mode: 'stream' | 'path';
  audioPath?: string;
  /** assistant 消息 id，流式音频由 store 持有。 */
  messageId?: string;
  speakerName?: string;
}

const props = withDefaults(defineProps<Props>(), {
  audioPath: undefined,
  messageId: undefined,
  speakerName: 'stream'
});

const uiStore = useUiStore();
const store = useStreamingSpeechStore();

const audioState = computed(() => (props.messageId ? store.audioStates[props.messageId] : undefined));
const hasData = computed(() => !!audioState.value?.hasData);
const isStreaming = computed(() => !!audioState.value?.isStreaming);
const streamComplete = computed(() => !!audioState.value?.streamComplete);
const sourceUrl = computed(() => (props.messageId ? store.getAudioUrl(props.messageId) : null));
const requiresManualResume = ref(false);
const isDownloading = ref(false);

const { isPlaying, togglePlayback, startPlayback, setAudioPath } = useStreamableAudioPlayer({
  sourceUrl: () => sourceUrl.value,
  hasData,
  onPlaybackEnded: () => {
    if (props.mode === 'stream') {
      // 流式播放在分片耗尽后改为手动续播，避免后续分片到达时自动抢播。
      requiresManualResume.value = true;
    }
  },
  onPlaybackError: () => {
    uiStore.notifyError('音频播放失败，请检查音频数据是否可解码。');
  }
});

const actionLabel = computed(() => {
  if (isPlaying.value) return '暂停播放';
  if (!hasData.value) return '等待数据';
  return '播放音频';
});

const showDownload = computed(() => props.mode === 'stream' && hasData.value && streamComplete.value);

const downloadAudio = async () => {
  if (!sourceUrl.value || isDownloading.value) {
    return;
  }
  isDownloading.value = true;
  try {
    const response = await fetch(sourceUrl.value);
    const blob = await response.blob();
    const link = document.createElement('a');
    const downloadUrl = URL.createObjectURL(blob);
    const speakerSegment = (props.speakerName || 'stream').trim().replace(/\s+/g, '-');
    const messageSegment = props.messageId || 'message';
    link.href = downloadUrl;
    link.download = `${speakerSegment}-${messageSegment}.wav`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(downloadUrl);
    uiStore.notifySuccess('音频下载已开始。', 2200);
  } catch (error) {
    uiStore.notifyError(error instanceof Error ? error.message : String(error));
  } finally {
    isDownloading.value = false;
  }
};

watch(
  () => props.audioPath,
  path => {
    if (path !== undefined) {
      setAudioPath(path);
    }
  },
  { immediate: true }
);

watch(
  () => props.messageId,
  () => {
    requiresManualResume.value = false;
  }
);

watch(
  [hasData, sourceUrl, isStreaming, streamComplete],
  ([nextHasData, nextSource]) => {
    if (props.mode !== 'stream') {
      return;
    }
    if (!nextHasData || !nextSource || isPlaying.value || requiresManualResume.value) {
      return;
    }
    startPlayback();
  },
  { immediate: true }
);
</script>

<template>
  <div class="flex items-center justify-end gap-1.5 bg-transparent p-0">
    <BaseButton
      tone="ghost"
      size="sm"
      :disabled="!hasData"
      class="h-8 min-h-0 w-8 min-w-0 rounded-full px-0"
      :title="actionLabel"
      :aria-label="actionLabel"
      @click="togglePlayback"
    >
      <component :is="isPlaying ? PauseIcon : PlayIcon" class="h-4 w-4" aria-hidden="true" />
    </BaseButton>

    <BaseButton
      v-if="showDownload"
      tone="ghost"
      size="sm"
      :loading="isDownloading"
      :disabled="isDownloading"
      class="h-8 min-h-0 w-8 min-w-0 rounded-full px-0"
      title="下载音频"
      aria-label="下载音频"
      @click="downloadAudio"
    >
      <ArrowDownTrayIcon v-if="!isDownloading" class="h-4 w-4" aria-hidden="true" />
    </BaseButton>
  </div>
</template>

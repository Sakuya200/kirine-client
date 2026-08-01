<script setup lang="ts">
import { ArrowDownTrayIcon, PauseIcon, PlayIcon } from '@heroicons/vue/24/outline';
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';

import BaseButton from '@/components/common/BaseButton.vue';
import { useStreamableAudioPlayer } from '@/hooks/useStreamableAudioPlayer';
import { useStreamingSpeechStore } from '@/stores/streamingSpeech';
import { useUiStore } from '@/stores/ui';
import { saveGeneratedAudio } from '@/utils/audioDownload';

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
const replayAudioUrl = ref<string | null>(null);
const isLoadingReplayAudio = ref(false);

const sourceUrl = computed(() => {
  if (props.audioPath) {
    return replayAudioUrl.value;
  }
  if (props.messageId) {
    return store.getAudioUrl(props.messageId);
  }
  return null;
});
const hasData = computed(() => Boolean(props.audioPath || sourceUrl.value));
const audioState = computed(() => (props.messageId ? store.audioStates[props.messageId] : undefined));
const message = computed(() => (props.messageId ? store.getMessage(props.messageId) : null));
const isDownloading = ref(false);

const { isPlaying, togglePlayback, setAudioPath } = useStreamableAudioPlayer({
  sourceUrl: () => sourceUrl.value,
  hasData,
  onPlaybackError: () => {
    uiStore.notifyError('音频播放失败，请检查音频数据是否可解码。');
  }
});

const actionLabel = computed(() => {
  if (isPlaying.value) return '暂停播放';
  if (isLoadingReplayAudio.value) return '加载音频中';
  if (!hasData.value) return '等待数据';
  return '播放音频';
});

const showDownload = computed(() => props.mode === 'stream' && (!!props.audioPath || audioState.value?.streamComplete === true));

interface StreamingSpeechAudioAsset {
  fileName: string;
  contentType: string;
  bytes: number[];
}

const releaseReplayAudioUrl = () => {
  if (replayAudioUrl.value) {
    URL.revokeObjectURL(replayAudioUrl.value);
    replayAudioUrl.value = null;
  }
};

const ensureReplayAudioLoaded = async () => {
  if (!props.audioPath || replayAudioUrl.value) {
    return true;
  }

  const historyId = message.value?.taskId;
  const messageId = props.messageId;
  if (historyId === undefined || !messageId) {
    uiStore.notifyWarning('当前没有可播放的音频索引。');
    return false;
  }

  isLoadingReplayAudio.value = true;
  try {
    const asset = await invoke<StreamingSpeechAudioAsset>('get_generated_audio', {
      source: { kind: 'streaming-speech', historyId, messageId }
    });
    replayAudioUrl.value = URL.createObjectURL(new Blob([Uint8Array.from(asset.bytes)], { type: asset.contentType || 'audio/wav' }));
    return true;
  } catch (error) {
    uiStore.notifyError(error instanceof Error ? error.message : String(error));
    return false;
  } finally {
    isLoadingReplayAudio.value = false;
  }
};

const handleTogglePlayback = async () => {
  if (isPlaying.value) {
    togglePlayback();
    return;
  }
  if (!(await ensureReplayAudioLoaded())) {
    return;
  }
  togglePlayback();
};

const downloadAudio = async () => {
  if (!props.messageId || isDownloading.value) {
    return;
  }
  if (!showDownload.value) {
    return;
  }
  isDownloading.value = true;
  try {
    const historyId = message.value?.taskId;
    const messageId = props.messageId;
    if (historyId === undefined || messageId === undefined) {
      uiStore.notifyWarning('当前没有可下载的音频索引。');
      return;
    }

    const saved = await saveGeneratedAudio({
      kind: 'streaming-speech',
      historyId,
      messageId
    });
    if (!saved) {
      uiStore.notifyInfo('已取消下载。', 2200);
      return;
    }

    uiStore.notifySuccess('音频已保存。', 2200);
  } catch (error) {
    uiStore.notifyError(error instanceof Error ? error.message : String(error));
  } finally {
    isDownloading.value = false;
  }
};

watch(
  () => props.audioPath,
  path => {
    releaseReplayAudioUrl();
    if (path !== undefined && !props.messageId) {
      setAudioPath(path);
    }
  },
  { immediate: true }
);

onBeforeUnmount(releaseReplayAudioUrl);
</script>

<template>
  <div class="flex items-center justify-end gap-1.5 bg-transparent p-0">
    <BaseButton
      tone="ghost"
      size="sm"
      :disabled="!hasData || isLoadingReplayAudio"
      class="h-8 min-h-0 w-8 min-w-0 rounded-full px-0"
      :title="actionLabel"
      :aria-label="actionLabel"
      @click="handleTogglePlayback"
    >
      <component :is="isPlaying ? PauseIcon : PlayIcon" class="h-4 w-4" aria-hidden="true" />
    </BaseButton>

    <BaseButton
      v-if="showDownload"
      tone="ghost"
      size="sm"
      :disabled="isDownloading"
      class="h-8 min-h-0 w-8 min-w-0 rounded-full px-0"
      :title="isDownloading ? '下载中…' : '下载音频'"
      :aria-label="isDownloading ? '下载中…' : '下载音频'"
      @click="downloadAudio"
    >
      <ArrowDownTrayIcon class="h-4 w-4" :class="isDownloading ? 'opacity-60' : ''" aria-hidden="true" />
    </BaseButton>
  </div>
</template>

<script setup lang="ts">
import { PauseIcon, PlayIcon } from '@heroicons/vue/24/outline';
import { computed, watch } from 'vue';

import BaseButton from '@/components/common/BaseButton.vue';
import { useStreamableAudioPlayer } from '@/hooks/useStreamableAudioPlayer';
import { useUiStore } from '@/stores/ui';

interface Props {
  mode: 'stream' | 'path';
  taskId?: number;
  contextId?: string;
  audioPath?: string;
  speakerName?: string;
  synthText?: string;
}

const props = withDefaults(defineProps<Props>(), {
  taskId: undefined,
  contextId: '',
  audioPath: undefined,
  speakerName: '',
  synthText: '',
});

const uiStore = useUiStore();

const { isPlaying, hasData, isStreaming, togglePlayback, startStreaming, setAudioPath } =
  useStreamableAudioPlayer({
    onPlaybackEnded: () => {
      uiStore.notifyInfo('音频播放结束。', 2200);
    },
    onPlaybackError: () => {
      uiStore.notifyError('音频播放失败，请检查音频数据是否可解码。');
    },
    onStreamError: message => {
      uiStore.notifyError(`音频流式接收失败：${message}`);
    },
  });

const actionLabel = computed(() => {
  if (isPlaying.value) return '暂停播放';
  if (!hasData.value) return '等待数据';
  return '播放音频';
});

if (props.mode === 'stream') {
  watch(
    () => [props.taskId, props.contextId, props.speakerName, props.synthText] as const,
    ([taskId, contextId, speakerName, synthText]) => {
      if (taskId != null) {
        void startStreaming(taskId, contextId, speakerName, synthText);
      }
    },
    { immediate: true },
  );
} else {
  watch(
    () => props.audioPath,
    path => {
      if (path !== undefined) {
        setAudioPath(path);
      }
    },
    { immediate: true },
  );
}
</script>

<template>
  <div class="inline-flex items-center gap-2 rounded-2xl border border-brand-200 bg-white/85 p-3">
    <BaseButton tone="ghost" :disabled="!hasData" @click="togglePlayback">
      <component :is="isPlaying ? PauseIcon : PlayIcon" class="h-4 w-4" aria-hidden="true" />
      <span>{{ actionLabel }}</span>
    </BaseButton>
    <span v-if="mode === 'stream' && !hasData" class="text-xs text-stone-500">等待音频数据…</span>
    <span v-else-if="mode === 'stream' && isStreaming" class="text-xs text-stone-500">流式接收中…</span>
  </div>
</template>

<script setup lang="ts">
import { PauseIcon, PlayIcon } from '@heroicons/vue/24/outline';
import { computed, watch } from 'vue';

import BaseButton from '@/components/common/BaseButton.vue';
import { useStreamableAudioPlayer } from '@/hooks/useStreamableAudioPlayer';
import { useStreamingSpeechStore } from '@/stores/streamingSpeech';
import { useUiStore } from '@/stores/ui';

interface Props {
  mode: 'stream' | 'path';
  audioPath?: string;
  /** assistant 消息 id，流式音频由 store 持有。 */
  messageId?: string;
}

const props = withDefaults(defineProps<Props>(), {
  audioPath: undefined,
  messageId: undefined
});

const uiStore = useUiStore();
const store = useStreamingSpeechStore();

const audioState = computed(() => (props.messageId ? store.audioStates[props.messageId] : undefined));
const hasData = computed(() => !!audioState.value?.hasData);
const isStreaming = computed(() => !!audioState.value?.isStreaming);
const sourceUrl = computed(() => (props.messageId ? store.getAudioUrl(props.messageId) : null));

const { isPlaying, togglePlayback, setAudioPath } = useStreamableAudioPlayer({
  sourceUrl: () => sourceUrl.value,
  hasData,
  onPlaybackEnded: () => {
    uiStore.notifyInfo('音频播放结束。', 2200);
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

watch(
  () => props.audioPath,
  path => {
    if (path !== undefined) {
      setAudioPath(path);
    }
  },
  { immediate: true }
);
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

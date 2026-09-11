<script setup lang="ts">
import { computed } from 'vue';

import StreamableAudioPlayer from '@/components/common/StreamableAudioPlayer.vue';
import StreamingMessageAvatar from '@/components/streaming/StreamingMessageAvatar.vue';
import StreamingMessageStatusPill from '@/components/streaming/StreamingMessageStatusPill.vue';
import { useStreamingSpeechStore } from '@/stores/streamingSpeech';
import type { StreamingChatMessage } from '@/types/streaming';

/**
 * 流式语音聊天单条消息整行：头像（左/右侧）、昵称 + 状态标签、
 * 聊天气泡（hover 微动画）与流式音频播放器。
 * 单一 message prop，speaker 派生（side/hasAvatar）在组件内经 store 计算，
 * 播放/下载逻辑由内嵌 StreamableAudioPlayer 自包含。
 * compact 为 true 时进入简洁展示：隐藏状态标签与播放/下载按钮区，仅保留头像、昵称与文本。
 */
interface Props {
  message: StreamingChatMessage;
  compact?: boolean;
}

const props = withDefaults(defineProps<Props>(), { compact: false });

const store = useStreamingSpeechStore();

/** 消息所属说话人；回放消息按 speakerId 匹配，匹配不到返回 null（占位头像兜底）。 */
const speaker = computed(() => store.getSpeaker(props.message.speakerId));
/** 消息展示侧由说话人配置的左右标记决定，缺省视为 right。 */
const isLeft = computed(() => speaker.value?.side === 'left');
const hasAvatar = computed(() => Boolean(speaker.value?.avatarPath));
</script>

<template>
  <div class="flex items-start gap-2.5" :class="isLeft ? 'justify-start' : 'justify-end'">
    <StreamingMessageAvatar
      v-if="isLeft"
      :task-id="message.taskId"
      :speaker-name="message.speakerName"
      :has-avatar="hasAvatar"
    />

    <div class="flex w-fit max-w-[92%] flex-col sm:max-w-[82%]" :class="isLeft ? 'items-start' : 'items-end'">
      <div
        class="mb-1.5 flex items-center gap-2 text-[11px] text-stone-500"
        :class="isLeft ? 'justify-start' : 'justify-end'"
      >
        <span class="truncate font-medium text-slate-600">{{ message.speakerName || '默认说话人' }}</span>
        <template v-if="!compact">
          <span class="h-1 w-1 rounded-full bg-stone-300" />
          <StreamingMessageStatusPill :status="message.status" />
        </template>
      </div>

      <div
        class="relative max-w-full rounded-[18px] border px-4 py-3 transition-[transform,box-shadow] duration-200 ease-out hover:-translate-y-0.5 motion-reduce:transition-none motion-reduce:hover:translate-y-0"
        :class="
          isLeft
            ? 'rounded-bl-[8px] border-brand-200/80 bg-brand-50/70 shadow-[0_10px_24px_rgba(180,83,9,0.08)] hover:shadow-[0_14px_30px_rgba(180,83,9,0.14)]'
            : 'rounded-br-[8px] border-sky-200/80 bg-sky-50/70 shadow-[0_10px_24px_rgba(14,116,144,0.08)] hover:shadow-[0_14px_30px_rgba(14,116,144,0.14)]'
        "
      >
        <p class="whitespace-pre-wrap break-words text-sm leading-6 text-slate-700">{{ message.text }}</p>
      </div>

      <div v-if="!compact" class="mt-2 flex" :class="isLeft ? 'justify-start self-start' : 'justify-end self-end'">
        <div class="max-w-[28rem]">
          <StreamableAudioPlayer mode="stream" :message-id="message.id" :audio-path="message.audioPath" :speaker-name="message.speakerName" />
        </div>
      </div>
    </div>

    <StreamingMessageAvatar
      v-if="!isLeft"
      :task-id="message.taskId"
      :speaker-name="message.speakerName"
      :has-avatar="hasAvatar"
    />
  </div>
</template>

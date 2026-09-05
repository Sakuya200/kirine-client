<script setup lang="ts">
import type { StreamingMessageStatus } from '@/types/streaming';

/**
 * 流式语音聊天消息状态标签：生成中 / 已完成 / 异常。
 * 纯展示组件，文案与配色由本地映射表驱动（对齐 enums/status.ts 的 Record 模式）。
 */
interface Props {
  status: StreamingMessageStatus;
}

const props = defineProps<Props>();

const STATUS_TEXT: Record<StreamingMessageStatus, string> = {
  streaming: '生成中',
  completed: '已完成',
  error: '异常'
};

const STATUS_STYLES: Record<StreamingMessageStatus, string> = {
  streaming: 'border-amber-200 bg-amber-50 text-amber-700',
  completed: 'border-emerald-200 bg-emerald-50 text-emerald-700',
  error: 'border-rose-200 bg-rose-50 text-rose-700'
};
</script>

<template>
  <span class="shrink-0 rounded-full border px-2 py-0.5" :class="STATUS_STYLES[props.status]">
    {{ STATUS_TEXT[props.status] }}
  </span>
</template>

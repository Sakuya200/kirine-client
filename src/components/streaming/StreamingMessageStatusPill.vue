<script setup lang="ts">
import { useI18n } from 'vue-i18n';

import type { StreamingMessageStatus } from '@/types/streaming';

/**
 * 流式语音聊天消息状态标签：生成中 / 已完成 / 异常。
 * 纯展示组件，配色由本地映射表驱动，文案走 i18n（对齐 enums/status.ts 的 key 模式）。
 */
interface Props {
  status: StreamingMessageStatus;
}

const props = defineProps<Props>();
const { t } = useI18n();

const STATUS_TEXT_KEY: Record<StreamingMessageStatus, string> = {
  streaming: 'streaming.statusPill.streaming',
  completed: 'streaming.statusPill.completed',
  error: 'streaming.statusPill.error'
};

const STATUS_STYLES: Record<StreamingMessageStatus, string> = {
  streaming: 'border-amber-200 bg-amber-50 text-amber-700',
  completed: 'border-emerald-200 bg-emerald-50 text-emerald-700',
  error: 'border-rose-200 bg-rose-50 text-rose-700'
};
</script>

<template>
  <span class="shrink-0 rounded-full border px-2 py-0.5" :class="STATUS_STYLES[props.status]">
    {{ t(STATUS_TEXT_KEY[props.status]) }}
  </span>
</template>

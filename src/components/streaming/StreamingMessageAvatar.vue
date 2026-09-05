<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';

import { useStreamingSpeechStore } from '@/stores/streamingSpeech';

/**
 * 聊天消息头像：有头像时经后端字节 command 加载 Blob URL 显示，
 * 无头像/加载失败时回退说话人名字首字圆形占位。
 */
interface Props {
  taskId: number;
  speakerName?: string;
  hasAvatar: boolean;
}

const props = withDefaults(defineProps<Props>(), { speakerName: '' });

const store = useStreamingSpeechStore();
const url = ref<string | null>(null);

const loadAvatar = async () => {
  if (!props.hasAvatar || !props.speakerName) {
    url.value = null;
    return;
  }
  // 旧 URL 可能已被 clearAvatarUrls revoke，先回退占位避免破图帧，待新 URL 就绪再上屏
  url.value = null;
  url.value = await store.ensureSpeakerAvatar(props.taskId, props.speakerName);
};

onMounted(loadAvatar);
watch(() => [props.taskId, props.speakerName, props.hasAvatar, store.avatarCacheVersion], loadAvatar);

const initial = computed(() => Array.from((props.speakerName || '？').trim())[0]?.toUpperCase() ?? '？');
</script>

<template>
  <img v-if="url" :src="url" class="mt-12 h-12 w-12 shrink-0 rounded-full border border-brand-200 object-cover shadow-soft" alt="" />
  <div
    v-else
    class="mt-12 flex h-12 w-12 shrink-0 items-center justify-center rounded-full border border-brand-200 bg-brand-100 text-sm font-semibold text-brand-700"
  >
    {{ initial }}
  </div>
</template>

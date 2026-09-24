<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { computed, onBeforeUnmount, ref, watch } from 'vue';

/**
 * 流式聊天消息头像：经 get_streaming_speaker_avatar 直取字节（后端按优先级返回
 * 会话覆盖头像或说话人记录头像）构造 Blob URL 显示，无头像/加载失败时回退
 * 说话人名字首字圆形占位。URL 由组件自己创建并在卸载/重载时 revoke，不做跨组件缓存。
 */
interface Props {
  taskId: number;
  speakerName?: string;
  hasAvatar: boolean;
}

const props = withDefaults(defineProps<Props>(), { speakerName: '' });

const url = ref<string | null>(null);

let currentUrl: string | null = null;

const revokeCurrent = () => {
  if (currentUrl) {
    URL.revokeObjectURL(currentUrl);
    currentUrl = null;
  }
  url.value = null;
};

const loadAvatar = async () => {
  revokeCurrent();
  if (!props.hasAvatar || !props.speakerName) {
    return;
  }
  try {
    const asset = await invoke<{ contentType: string; bytes: number[] }>('get_streaming_speaker_avatar', {
      historyId: props.taskId,
      speakerName: props.speakerName
    });
    const blob = new Blob([new Uint8Array(asset.bytes)], { type: asset.contentType });
    currentUrl = URL.createObjectURL(blob);
    url.value = currentUrl;
  } catch {
    revokeCurrent();
  }
};

onBeforeUnmount(revokeCurrent);
watch(() => [props.taskId, props.speakerName, props.hasAvatar], loadAvatar, { immediate: true });

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

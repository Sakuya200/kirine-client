<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { computed, onBeforeUnmount, ref, watch } from 'vue';

/**
 * 说话人头像：有头像时经 get_speaker_avatar 取字节构造 Blob URL 显示，
 * 无头像/加载失败时回退说话人名字首字圆形占位。URL 由组件自己创建并在
 * 卸载/重载时 revoke，不做跨组件缓存。
 */
interface Props {
  speakerId: number;
  hasAvatar: boolean;
  speakerName: string;
  /** 头像尺寸（Tailwind 类），默认卡片用 h-12 w-12 */
  sizeClass?: string;
}

const props = withDefaults(defineProps<Props>(), { sizeClass: 'h-12 w-12' });

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
  if (!props.hasAvatar || !props.speakerId) {
    return;
  }
  try {
    const asset = await invoke<{ contentType: string; bytes: number[] }>('get_speaker_avatar', { speakerId: props.speakerId });
    const blob = new Blob([new Uint8Array(asset.bytes)], { type: asset.contentType || 'image/png' });
    currentUrl = URL.createObjectURL(blob);
    url.value = currentUrl;
  } catch {
    revokeCurrent();
  }
};

onBeforeUnmount(revokeCurrent);
watch(() => [props.speakerId, props.hasAvatar], loadAvatar, { immediate: true });

const initial = computed(() => Array.from((props.speakerName || '？').trim())[0]?.toUpperCase() ?? '？');
</script>

<template>
  <img v-if="url" :src="url" :class="['shrink-0 rounded-full border border-brand-200 object-cover shadow-soft', sizeClass]" alt="" />
  <div
    v-else
    :class="['flex shrink-0 items-center justify-center rounded-full border border-brand-200 bg-brand-100 text-sm font-semibold text-brand-700', sizeClass]"
  >
    {{ initial }}
  </div>
</template>

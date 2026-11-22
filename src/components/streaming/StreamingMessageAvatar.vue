<script setup lang="ts">
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { computed, ref, watch } from 'vue';

/**
 * 流式聊天消息头像。URL 解析优先级：
 * ① 会话覆盖头像文件 → get_streaming_speaker_avatar_path + convertFileSrc（asset 协议零拷贝）；
 * ② 仅记录头像（speakers 表字节列）→ 回退 get_streaming_speaker_avatar 字节命令构造 Blob URL。
 * 解析结果按 `taskId:speakerName` 做模块级缓存：同说话人多条消息只发起一次 IPC（字节版
 * 的 Blob URL 缓存后不 revoke，量级 ≈ 说话人数，可忽略）。无头像/解析失败回退首字占位。
 */
interface Props {
  taskId: number;
  speakerName?: string;
  hasAvatar: boolean;
}

const props = withDefaults(defineProps<Props>(), { speakerName: '' });

const avatarUrlCache = new Map<string, Promise<string | null>>();

const resolveAvatarUrl = (historyId: number, speakerName: string): Promise<string | null> => {
  const key = `${historyId}:${speakerName}`;
  let entry = avatarUrlCache.get(key);
  if (!entry) {
    entry = (async () => {
      try {
        const asset = await invoke<{ absolutePath: string }>('get_streaming_speaker_avatar_path', {
          historyId,
          speakerName
        });
        return convertFileSrc(asset.absolutePath);
      } catch {
        // 无会话覆盖头像 → 回退字节命令（记录头像场景）
        try {
          const asset = await invoke<{ contentType: string; bytes: number[] }>('get_streaming_speaker_avatar', {
            historyId,
            speakerName
          });
          return URL.createObjectURL(new Blob([new Uint8Array(asset.bytes)], { type: asset.contentType }));
        } catch {
          return null;
        }
      }
    })();
    // 失败不缓存：下次访问重试（头像可能随后被配置）
    entry.then(resolved => {
      if (resolved === null) {
        avatarUrlCache.delete(key);
      }
    });
    avatarUrlCache.set(key, entry);
  }
  return entry;
};

const url = ref<string | null>(null);

watch(
  () => [props.taskId, props.speakerName, props.hasAvatar] as const,
  async () => {
    const { taskId, speakerName, hasAvatar } = props;
    if (!hasAvatar || !speakerName) {
      url.value = null;
      return;
    }
    const resolved = await resolveAvatarUrl(taskId, speakerName);
    // 异步解析期间 props 可能已切换到另一条消息，丢弃过期结果
    if (props.taskId === taskId && props.speakerName === speakerName) {
      url.value = resolved;
    }
  },
  { immediate: true }
);

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

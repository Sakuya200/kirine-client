<script setup lang="ts">
import { Cog6ToothIcon, PaperAirplaneIcon, StopCircleIcon, TrashIcon } from '@heroicons/vue/24/outline';
import { computed, nextTick, onMounted, ref, watch } from 'vue';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import StreamableAudioPlayer from '@/components/common/StreamableAudioPlayer.vue';
import StreamingConfigDrawer from '@/components/streaming/StreamingConfigDrawer.vue';
import { useModelStore } from '@/stores/models';
import { useStreamingSpeechStore } from '@/stores/streamingSpeech';
import { useUiConfigStore } from '@/stores/uiConfig';
import { useUiStore } from '@/stores/ui';

const store = useStreamingSpeechStore();
const modelStore = useModelStore();
const uiConfigStore = useUiConfigStore();
const uiStore = useUiStore();

const selectedSpeakerId = ref<string | null>(null);
const inputText = ref('');
const messagesContainerRef = ref<HTMLElement | null>(null);

const canSend = computed(() => inputText.value.trim().length > 0 && selectedSpeakerId.value !== null);
const hasSpeakers = computed(() => store.speakers.length > 0);
const selectedSpeakerName = computed(() => store.getSpeaker(selectedSpeakerId.value)?.name ?? '未选择');

// 说话人列表变化时，保持有效选中（无选中时回退首个）
watch(
  () => store.speakers,
  speakers => {
    if (selectedSpeakerId.value !== null && speakers.some(speaker => speaker.id === selectedSpeakerId.value)) {
      return;
    }
    selectedSpeakerId.value = speakers[0]?.id ?? null;
  },
  { immediate: true, deep: true }
);

const scrollToBottom = () => {
  nextTick(() => {
    const el = messagesContainerRef.value;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  });
};

watch(() => store.messages.length, scrollToBottom);

const send = () => {
  if (!canSend.value) {
    if (!hasSpeakers.value) {
      uiStore.notifyWarning('请先在配置抽屉中添加说话人。');
    }
    return;
  }
  store.sendMessage(inputText.value, selectedSpeakerId.value);
  inputText.value = '';
  scrollToBottom();
};

const onTextareaKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    send();
  }
};

const onSpeakerChange = (value: unknown) => {
  selectedSpeakerId.value = (value as string | null) ?? null;
};

const clearMessages = () => {
  store.clearMessages();
  uiStore.notifyInfo('已清空对话。', 2000);
};

onMounted(async () => {
  await Promise.all([uiConfigStore.ensureLoaded(), modelStore.ensureLoaded()]);
});
</script>

<template>
  <div class="flex h-[calc(100vh-3.5rem)] flex-col gap-4">
    <PageHeader title="流式语音" description="选择说话人，输入文本即可实时流式生成语音。" eyebrow="Streaming Speech" />

    <div class="flex items-center justify-between gap-3">
      <p class="text-xs text-stone-500">
        当前说话人：<span class="font-medium text-slate-700">{{ selectedSpeakerName }}</span>
        · 回车发送，Shift+Enter 换行
      </p>
      <div class="flex gap-2">
        <BaseButton
          tone="ghost"
          size="sm"
          :disabled="store.activeTaskId === null"
          @click="store.terminateSession"
        >
          <StopCircleIcon class="h-4 w-4" aria-hidden="true" />
          <span>终止会话</span>
        </BaseButton>
        <BaseButton tone="ghost" size="sm" :disabled="store.messages.length === 0" @click="clearMessages">
          <TrashIcon class="h-4 w-4" aria-hidden="true" />
          <span>清空对话</span>
        </BaseButton>
        <BaseButton tone="ghost" size="sm" @click="store.openDrawer">
          <Cog6ToothIcon class="h-4 w-4" aria-hidden="true" />
          <span>配置</span>
        </BaseButton>
      </div>
    </div>

    <div class="flex min-h-0 flex-1 flex-col overflow-hidden rounded-2xl border border-brand-100/70 bg-white/88 shadow-soft backdrop-blur-sm">
      <div ref="messagesContainerRef" class="flex-1 space-y-4 overflow-y-auto p-5">
        <div v-if="store.messages.length === 0" class="flex h-full items-center justify-center">
          <div class="text-center text-sm text-stone-500">
            <p class="text-base font-medium text-slate-700">开始流式语音对话</p>
            <p class="mt-2">输入文本、选择说话人，回车即可生成实时语音。</p>
            <p v-if="!hasSpeakers" class="mt-2 text-brand-700">尚未配置说话人，请先点击右上角「配置」添加。</p>
          </div>
        </div>

        <template v-else>
          <div v-for="message in store.messages" :key="message.id" class="flex" :class="message.role === 'user' ? 'justify-end' : 'justify-start'">
            <div class="max-w-[78%]">
              <div v-if="message.role === 'user'" class="rounded-2xl rounded-br-md bg-brand-500 px-4 py-2.5 text-sm text-white shadow-soft">
                <p class="whitespace-pre-wrap break-words">{{ message.text }}</p>
              </div>
              <div v-else class="rounded-2xl rounded-bl-md border border-brand-200 bg-white/95 px-4 py-3 shadow-soft">
                <p class="mb-2 text-xs text-stone-500">{{ message.speakerName ? `说话人：${message.speakerName}` : '流式生成中…' }}</p>
                <StreamableAudioPlayer
                  mode="stream"
                  :task-id="message.taskId"
                  :context-id="message.contextId"
                  :speaker-name="message.speakerName ?? ''"
                  :synth-text="message.synthText"
                />
              </div>
            </div>
          </div>
        </template>
      </div>

      <div class="border-t border-brand-100 p-4">
        <div class="mb-3 w-60">
          <BaseListbox
            :model-value="selectedSpeakerId"
            :options="store.speakerOptions"
            label="说话人"
            :placeholder="hasSpeakers ? '选择说话人' : '未配置说话人'"
            :disabled="!hasSpeakers"
            @update:model-value="onSpeakerChange"
          />
        </div>
        <div class="flex items-end gap-2">
          <textarea
            v-model="inputText"
            rows="2"
            class="min-h-[44px] flex-1 resize-none rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 outline-none transition focus:border-brand-400"
            placeholder="输入要说的话，回车发送…"
            @keydown="onTextareaKeydown"
          />
          <BaseButton :disabled="!canSend" @click="send">
            <PaperAirplaneIcon class="h-4 w-4" aria-hidden="true" />
            <span>发送</span>
          </BaseButton>
        </div>
      </div>
    </div>

    <StreamingConfigDrawer :open="store.isDrawerOpen" @close="store.closeDrawer" />
  </div>
</template>

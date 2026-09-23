<script setup lang="ts">
import { Cog6ToothIcon, EyeIcon, EyeSlashIcon, PaperAirplaneIcon, PlayCircleIcon, StopCircleIcon, TrashIcon } from '@heroicons/vue/24/outline';
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { useRoute, useRouter } from 'vue-router';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import StreamingMessageItem from '@/components/streaming/StreamingMessageItem.vue';
import StreamingConfigDrawer from '@/components/streaming/StreamingConfigDrawer.vue';
import WarningConfirmDialog from '@/components/common/WarningConfirmDialog.vue';
import { getHistoryTaskReplayId, HISTORY_TASK_REPLAY_QUERY_KEY } from '@/enums/task';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useTaskDeviceTypeGuard } from '@/hooks/useTaskDeviceTypeGuard';
import { useStreamingSpeechStore } from '@/stores/streamingSpeech';
import { useUiConfigStore } from '@/stores/uiConfig';
import { useUiStore } from '@/stores/ui';
import type { StreamingReplaySnapshot } from '@/types/domain';
import type { StreamingSpeakerConfig } from '@/types/streaming';

const store = useStreamingSpeechStore();
const { t } = useI18n();
const uiConfigStore = useUiConfigStore();
const uiStore = useUiStore();
const route = useRoute();
const router = useRouter();
const {
  dialogOpen: showDeviceMismatchDialog,
  dialogTitle: deviceMismatchDialogTitle,
  dialogMessage: deviceMismatchDialogMessage,
  dialogDetailLines: deviceMismatchDialogDetails,
  ensureMatchedOrConfirmed,
  confirmDialog: confirmDeviceMismatchDialog,
  closeDialog: closeDeviceMismatchDialog
} = useTaskDeviceTypeGuard();

const selectedSpeakerId = ref<string | null>(null);
const inputText = ref('');
const messagesContainerRef = ref<HTMLElement | null>(null);
const isStartSessionRequested = ref(false);
/** 简洁展示模式：隐藏消息内的状态标签与播放/下载按钮，仅保留头像、昵称与文本。 */
const compactMessages = ref(false);

const canSend = computed(
  () => inputText.value.trim().length > 0 && selectedSpeakerId.value !== null && store.activeTaskId !== null && !store.isStartingSession
);
const hasSpeakers = computed(() => store.speakers.length > 0);
const selectedSpeakerName = computed(() => store.getSpeaker(selectedSpeakerId.value)?.name ?? t('streaming.view.notSelected'));

watch(
  () => store.speakers as StreamingSpeakerConfig[],
  speakers => {
    if (selectedSpeakerId.value !== null && speakers.some(speaker => speaker.id === selectedSpeakerId.value)) {
      return;
    }
    selectedSpeakerId.value = speakers[0]?.id ?? null;
  },
  { immediate: true, deep: true }
);

// 进出场动画只用 transform/opacity（不动画 height/margin），元素插入时 scrollHeight 即最终值，
// nextTick 后直接滚到底部即可精确落点。
const scrollToBottom = () => {
  nextTick(() => {
    const el = messagesContainerRef.value;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  });
};

watch(() => store.messages.length, scrollToBottom);

const send = async () => {
  if (!canSend.value) {
    if (!hasSpeakers.value) {
      uiStore.notifyWarning(t('streaming.view.addSpeakerFirst'));
    } else if (store.activeTaskId === null) {
      uiStore.notifyWarning(t('streaming.view.startSessionFirst'));
    }
    return;
  }
  const text = inputText.value;
  inputText.value = '';
  scrollToBottom();
  try {
    await store.sendMessage(text, selectedSpeakerId.value);
  } catch (err) {
    inputText.value = text;
    uiStore.notifyError(err instanceof Error ? err.message : String(err));
  }
};

const startSession = async () => {
  if (isStartSessionRequested.value || store.activeTaskId !== null || store.isStartingSession) {
    return;
  }
  isStartSessionRequested.value = true;
  try {
    if (!store.sessionConfig.baseModel || !store.sessionConfig.modelVersion) {
      await store.startSession();
      return;
    }
    const accepted = await ensureMatchedOrConfirmed({
      baseModel: store.sessionConfig.baseModel,
      modelVersion: store.sessionConfig.modelVersion,
      selectedDevice: store.sessionConfig.device
    });
    if (!accepted) {
      return;
    }
    await store.startSession();
  } catch (err) {
    uiStore.notifyError(err instanceof Error ? err.message : String(err));
  } finally {
    isStartSessionRequested.value = false;
  }
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
  uiStore.notifyInfo(t('streaming.view.cleared'), 2000);
};

const clearReplayTaskId = async () => {
  if (!(HISTORY_TASK_REPLAY_QUERY_KEY in route.query)) {
    return;
  }
  const nextQuery = { ...route.query };
  delete nextQuery[HISTORY_TASK_REPLAY_QUERY_KEY];
  await router.replace({ path: route.path, query: nextQuery });
};

const hydrateReplayTaskFromRoute = async () => {
  const historyId = getHistoryTaskReplayId(route.query[HISTORY_TASK_REPLAY_QUERY_KEY]);

  if (historyId === null) {
    await clearReplayTaskId();
    return;
  }

  try {
    const snapshot = await invoke<StreamingReplaySnapshot>('get_streaming_replay_snapshot', {
      historyId
    });
    store.restoreFromReplaySnapshot(snapshot);

    uiStore.notifySuccess(t('streaming.view.restored'), 2600);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('streaming.view.restoreFailed'), error));
  } finally {
    await clearReplayTaskId();
  }
};

onMounted(async () => {
  await uiConfigStore.ensureLoaded();
  await hydrateReplayTaskFromRoute();
});
</script>

<template>
  <div class="flex h-[calc(100vh-3.5rem)] flex-col gap-4">
    <PageHeader :title="t('streaming.title')" :description="t('streaming.description')" eyebrow="Streaming Speech" />

    <div class="flex items-center justify-between gap-3">
      <p class="text-xs text-stone-500">
        <span class="text-stone-500">{{ t('streaming.view.currentSpeaker') }}</span><span class="font-medium text-slate-700">{{ selectedSpeakerName }}</span>
        {{ t('streaming.view.inputHint') }}
      </p>
      <div class="flex gap-2">
        <BaseButton
          tone="ghost"
          size="sm"
          :disabled="store.activeTaskId !== null || store.isStartingSession || isStartSessionRequested"
          @click="startSession"
        >
          <PlayCircleIcon class="h-4 w-4" aria-hidden="true" />
          <span>{{ store.isStartingSession || isStartSessionRequested ? t('streaming.view.loadingModel') : t('streaming.view.startSession') }}</span>
        </BaseButton>
        <BaseButton tone="ghost" size="sm" :disabled="store.activeTaskId === null" @click="store.terminateSession">
          <StopCircleIcon class="h-4 w-4" aria-hidden="true" />
          <span>{{ t('streaming.view.stopSession') }}</span>
        </BaseButton>
        <BaseButton tone="ghost" size="sm" :disabled="store.messages.length === 0" @click="clearMessages">
          <TrashIcon class="h-4 w-4" aria-hidden="true" />
          <span>{{ t('streaming.view.clearMessages') }}</span>
        </BaseButton>
        <BaseButton
          tone="ghost"
          size="sm"
          :title="compactMessages ? t('streaming.view.showFeaturesTitle') : t('streaming.view.hideFeaturesTitle')"
          :aria-label="compactMessages ? t('streaming.view.showFeaturesTitle') : t('streaming.view.hideFeaturesTitle')"
          @click="compactMessages = !compactMessages"
        >
          <component :is="compactMessages ? EyeIcon : EyeSlashIcon" class="h-4 w-4" aria-hidden="true" />
          <span>{{ compactMessages ? t('streaming.view.showFeatures') : t('streaming.view.hideFeatures') }}</span>
        </BaseButton>
        <BaseButton tone="ghost" size="sm" @click="store.openDrawer">
          <Cog6ToothIcon class="h-4 w-4" aria-hidden="true" />
          <span>{{ t('streaming.view.settings') }}</span>
        </BaseButton>
      </div>
    </div>

    <div class="flex min-h-0 flex-1 flex-col overflow-hidden rounded-2xl border border-brand-100/70 bg-white/88 shadow-soft backdrop-blur-sm">
      <div ref="messagesContainerRef" class="relative flex-1 overflow-y-auto p-5">
        <div v-if="store.messages.length === 0" class="flex h-full items-center justify-center">
          <div class="text-center text-sm text-stone-500">
            <p class="text-base font-medium text-slate-700">{{ t('streaming.view.emptyTitle') }}</p>
            <p class="mt-2">{{ t('streaming.view.emptyHint') }}</p>
            <p v-if="!hasSpeakers" class="mt-2 text-brand-700">{{ t('streaming.view.emptyNoSpeakers') }}</p>
          </div>
        </div>

        <TransitionGroup tag="div" name="message-stack" class="space-y-4">
          <StreamingMessageItem v-for="message in store.messages" :key="message.id" :message="message" :compact="compactMessages" />
        </TransitionGroup>
      </div>

      <div class="border-t border-brand-100 p-4">
        <div class="mb-3 flex items-end justify-between gap-3">
          <div class="w-60">
            <BaseListbox
              :model-value="selectedSpeakerId"
              :options="store.speakerOptions"
              :label="t('streaming.view.speaker')"
              :placeholder="hasSpeakers ? t('streaming.view.speakerPlaceholderReady') : t('streaming.view.speakerPlaceholderNone')"
              :disabled="!hasSpeakers"
              @update:model-value="onSpeakerChange"
            />
          </div>
          <BaseButton class="w-20" :disabled="!canSend" @click="send">
            <PaperAirplaneIcon class="h-4 w-4" aria-hidden="true" />
            <span>{{ t('streaming.view.send') }}</span>
          </BaseButton>
        </div>
        <div>
          <textarea
            v-model="inputText"
            rows="2"
            class="min-h-[96px] w-full resize-none rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 outline-none transition focus:border-brand-400"
            :placeholder="t('streaming.view.inputPlaceholder')"
            @keydown="onTextareaKeydown"
          />
        </div>
      </div>
    </div>

    <StreamingConfigDrawer :open="store.isDrawerOpen" @close="store.closeDrawer" />

    <WarningConfirmDialog
      :open="showDeviceMismatchDialog"
      :title="deviceMismatchDialogTitle"
      :message="deviceMismatchDialogMessage"
      :details="deviceMismatchDialogDetails"
      :confirm-text="t('streaming.view.confirmText')"
      :cancel-text="t('common.cancel')"
      @confirm="confirmDeviceMismatchDialog"
      @cancel="closeDeviceMismatchDialog"
      @close="closeDeviceMismatchDialog"
    />
  </div>
</template>

<style scoped>
.message-stack-enter-active {
  transition: opacity 200ms ease-out, transform 200ms ease-out;
}

.message-stack-leave-active {
  transition: opacity 160ms ease-in, transform 160ms ease-in;
  /* 退场元素脱离文档流，避免回放重建时新旧列表高度叠加导致滚动跳动 */
  position: absolute;
  left: 1.25rem;
  right: 1.25rem;
}

.message-stack-move {
  transition: transform 200ms ease;
}

.message-stack-enter-from {
  opacity: 0;
  transform: translateY(12px);
}

.message-stack-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>

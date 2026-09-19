<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { computed, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { StopCircleIcon } from '@heroicons/vue/24/outline';

import { HISTORY_TASK_REPLAY_QUERY_KEY, HISTORY_TASK_ROUTE_PATH, HISTORY_TASK_TYPE_TEXT_KEY, HistoryTaskType } from '@/enums/task';
import { TaskStatus } from '@/enums/status';
import BaseButton from '@/components/common/BaseButton.vue';
import BaseDialog from '@/components/common/BaseDialog.vue';
import StatusPill from '@/components/common/StatusPill.vue';
import ModelTrainingTaskDetailForm from '@/components/form/ModelTrainingTaskDetailForm.vue';
import StreamingTaskDetailForm from '@/components/form/StreamingTaskDetailForm.vue';
import TextToSpeechTaskDetailForm from '@/components/form/TextToSpeechTaskDetailForm.vue';
import VoiceCloneTaskDetailForm from '@/components/form/VoiceCloneTaskDetailForm.vue';
import VoiceDesignTaskDetailForm from '@/components/form/VoiceDesignTaskDetailForm.vue';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useUiStore } from '@/stores/ui';
import type { HistoryRecord } from '@/types/domain';
import { formatDurationClock } from '@/utils/formatDurationClock';

interface Props {
  open: boolean;
  recordId: number | null;
  reloadToken?: number;
}

const props = withDefaults(defineProps<Props>(), {
  reloadToken: 0
});

const emit = defineEmits<{
  close: [];
  cancel: [historyId: number];
}>();

const router = useRouter();
const { t } = useI18n();
const uiStore = useUiStore();
const record = ref<HistoryRecord | null>(null);
const isLoading = ref(false);

const loadDetailRecord = async () => {
  if (!props.recordId) {
    record.value = null;
    return;
  }

  isLoading.value = true;
  try {
    record.value = await invoke<HistoryRecord>('get_history_record', {
      historyId: props.recordId
    });
  } catch (error) {
    record.value = null;
    uiStore.notifyError(formatErrorMessage(t('history.taskDetail.loadFailed'), error));
  } finally {
    isLoading.value = false;
  }
};

watch(
  () => [props.open, props.recordId, props.reloadToken],
  ([open, recordId]) => {
    if (!open || !recordId) {
      if (!open) {
        record.value = null;
      }
      return;
    }

    void loadDetailRecord();
  },
  { immediate: true }
);

const dialogTitle = computed(() => {
  if (!record.value) {
    return isLoading.value ? t('history.taskDetail.loading') : t('history.taskDetail.title');
  }

  return t('common.taskTypeDetail', { type: t(HISTORY_TASK_TYPE_TEXT_KEY[record.value.taskType]) });
});

const canReplay = computed(() => Boolean(router));
const canCancel = computed(() => Boolean(record.value && [TaskStatus.Pending, TaskStatus.Running].includes(record.value.status)));

const replayTask = async (record: HistoryRecord | null) => {
  if (!record) {
    return;
  }

  emit('close');
  await router.push({
    path: HISTORY_TASK_ROUTE_PATH[record.taskType],
    query: {
      [HISTORY_TASK_REPLAY_QUERY_KEY]: String(record.id)
    }
  });
};

const requestCancel = (record: HistoryRecord | null) => {
  if (!record || !canCancel.value) {
    return;
  }

  emit('cancel', record.id);
};
</script>

<template>
  <BaseDialog :open="open" :title="dialogTitle" panel-class="max-w-4xl" content-class="max-h-[60vh] overflow-y-auto pr-2" @close="emit('close')">
    <div v-if="isLoading" class="rounded-2xl border border-brand-200 bg-brand-50/40 p-4 text-sm text-stone-600">{{ t('history.taskDetail.loadingText') }}</div>
    <div v-if="record" class="space-y-4 text-sm text-slate-600">
      <div class="grid gap-3 md:grid-cols-2">
        <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-xs text-stone-500">{{ t('history.taskDetail.taskId') }}</p>
          <p class="mt-1 font-mono font-semibold text-slate-800">{{ record.id }}</p>
        </article>
        <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-xs text-stone-500">{{ t('history.taskDetail.taskStatus') }}</p>
          <div class="mt-2">
            <StatusPill :status="record.status" />
          </div>
        </article>
        <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-xs text-stone-500">{{ t('history.taskDetail.taskName') }}</p>
          <p class="mt-1 font-semibold text-slate-800">{{ record.title }}</p>
        </article>
        <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-xs text-stone-500">{{ t('history.taskDetail.speaker') }}</p>
          <p class="mt-1 font-semibold text-slate-800">{{ record.speaker }}</p>
        </article>
        <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-xs text-stone-500">{{ t('history.taskDetail.createTime') }}</p>
          <p class="mt-1 font-semibold text-slate-800">{{ record.createTime }}</p>
        </article>
        <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-xs text-stone-500">{{ t('history.taskDetail.modifyTime') }}</p>
          <p class="mt-1 font-semibold text-slate-800">{{ record.modifyTime }}</p>
        </article>
      </div>

      <section class="rounded-2xl border border-brand-200 bg-brand-50/40 p-4">
        <div class="flex items-center justify-between gap-3">
          <p class="text-sm font-semibold text-slate-800">{{ t('history.taskDetail.params') }}</p>
          <span class="text-xs text-stone-500">{{ t('history.taskDetail.duration', { duration: formatDurationClock(record.durationSeconds) }) }}</span>
        </div>
        <div class="mt-4">
          <ModelTrainingTaskDetailForm v-if="record.taskType === HistoryTaskType.ModelTraining" :record="record" />
          <VoiceCloneTaskDetailForm v-else-if="record.taskType === HistoryTaskType.VoiceClone" :record="record" />
          <VoiceDesignTaskDetailForm v-else-if="record.taskType === HistoryTaskType.VoiceDesign" :record="record" />
          <StreamingTaskDetailForm v-else-if="record.taskType === HistoryTaskType.StreamingSpeech" :record="record" />
          <TextToSpeechTaskDetailForm v-else :record="record" />
        </div>
      </section>

      <section class="rounded-2xl border border-brand-200 bg-brand-50/40 p-4">
        <p class="text-sm font-semibold text-slate-800">{{ t('history.taskDetail.logs') }}</p>
        <div class="mt-3 max-h-[22rem] overflow-y-auto rounded-xl bg-white/85 px-3 py-3 text-sm leading-6 text-slate-700">
          <pre v-if="record.taskLog" class="whitespace-pre-wrap break-words font-sans">{{ record.taskLog }}</pre>
          <p v-else class="text-sm text-stone-500">{{ t('history.taskDetail.noLogs') }}</p>
        </div>
      </section>
    </div>
    <template #footer>
      <BaseButton v-if="canCancel" tone="quiet" @click="requestCancel(record)">
        <StopCircleIcon class="h-4 w-4" aria-hidden="true" />
        <span>{{ t('history.taskDetail.cancelTask') }}</span>
      </BaseButton>
      <BaseButton :disabled="!record || !canReplay" @click="replayTask(record)">{{ t('history.taskDetail.replay') }}</BaseButton>
      <BaseButton tone="ghost" @click="emit('close')">{{ t('history.taskDetail.close') }}</BaseButton>
    </template>
  </BaseDialog>
</template>

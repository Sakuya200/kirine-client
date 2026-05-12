<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { computed, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { StopCircleIcon } from '@heroicons/vue/24/outline';

import { HISTORY_TASK_REPLAY_QUERY_KEY, HISTORY_TASK_ROUTE_PATH, HISTORY_TASK_TYPE_TEXT, HistoryTaskType } from '@/enums/task';
import { TaskStatus } from '@/enums/status';
import BaseButton from '@/components/common/BaseButton.vue';
import BaseDialog from '@/components/common/BaseDialog.vue';
import StatusPill from '@/components/common/StatusPill.vue';
import ModelTrainingTaskDetailForm from '@/components/form/ModelTrainingTaskDetailForm.vue';
import TextToSpeechTaskDetailForm from '@/components/form/TextToSpeechTaskDetailForm.vue';
import VoiceCloneTaskDetailForm from '@/components/form/VoiceCloneTaskDetailForm.vue';
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
    uiStore.notifyError(formatErrorMessage('读取任务详情失败，请检查历史记录是否仍然存在', error));
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
    return isLoading.value ? '任务详情加载中' : '任务详情';
  }

  return `${HISTORY_TASK_TYPE_TEXT[record.value.taskType]}详情`;
});

const canReplay = computed(() => Boolean(router));
const canCancel = computed(() =>
  Boolean(
    record.value && record.value.taskType === HistoryTaskType.ModelTraining && [TaskStatus.Pending, TaskStatus.Running].includes(record.value.status)
  )
);

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
    <div v-if="isLoading" class="rounded-2xl border border-brand-200 bg-brand-50/40 p-4 text-sm text-stone-600">正在加载任务详情...</div>
    <div v-if="record" class="space-y-4 text-sm text-slate-600">
      <div class="grid gap-3 md:grid-cols-2">
        <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-xs text-stone-500">任务 ID</p>
          <p class="mt-1 font-mono font-semibold text-slate-800">{{ record.id }}</p>
        </article>
        <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-xs text-stone-500">任务状态</p>
          <div class="mt-2">
            <StatusPill :status="record.status" />
          </div>
        </article>
        <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-xs text-stone-500">任务名称</p>
          <p class="mt-1 font-semibold text-slate-800">{{ record.title }}</p>
        </article>
        <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-xs text-stone-500">说话人</p>
          <p class="mt-1 font-semibold text-slate-800">{{ record.speaker }}</p>
        </article>
        <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-xs text-stone-500">创建时间</p>
          <p class="mt-1 font-semibold text-slate-800">{{ record.createTime }}</p>
        </article>
        <article class="rounded-2xl border border-brand-200 bg-white/80 p-4">
          <p class="text-xs text-stone-500">最近更新时间</p>
          <p class="mt-1 font-semibold text-slate-800">{{ record.modifyTime }}</p>
        </article>
      </div>

      <section class="rounded-2xl border border-brand-200 bg-brand-50/40 p-4">
        <div class="flex items-center justify-between gap-3">
          <p class="text-sm font-semibold text-slate-800">任务参数</p>
          <span class="text-xs text-stone-500">耗时 {{ formatDurationClock(record.durationSeconds) }}</span>
        </div>
        <div class="mt-4">
          <ModelTrainingTaskDetailForm v-if="record.taskType === HistoryTaskType.ModelTraining" :record="record" />
          <VoiceCloneTaskDetailForm v-else-if="record.taskType === HistoryTaskType.VoiceClone" :record="record" />
          <TextToSpeechTaskDetailForm v-else :record="record" />
        </div>
      </section>

      <section class="rounded-2xl border border-brand-200 bg-brand-50/40 p-4">
        <p class="text-sm font-semibold text-slate-800">任务日志</p>
        <div class="mt-3 max-h-[22rem] overflow-y-auto rounded-xl bg-white/85 px-3 py-3 text-sm leading-6 text-slate-700">
          <pre v-if="record.taskLog" class="whitespace-pre-wrap break-words font-sans">{{ record.taskLog }}</pre>
          <p v-else class="text-sm text-stone-500">暂无任务日志。</p>
        </div>
      </section>
    </div>
    <template #footer>
      <BaseButton v-if="canCancel" tone="quiet" @click="requestCancel(record)">
        <StopCircleIcon class="h-4 w-4" aria-hidden="true" />
        <span>终止任务</span>
      </BaseButton>
      <BaseButton :disabled="!record || !canReplay" @click="replayTask(record)">再次执行</BaseButton>
      <BaseButton tone="ghost" @click="emit('close')">关闭</BaseButton>
    </template>
  </BaseDialog>
</template>

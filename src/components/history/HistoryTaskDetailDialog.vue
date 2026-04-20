<script setup lang="ts">
import { computed } from 'vue';
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
import type { HistoryRecord } from '@/types/domain';
import { formatDurationClock } from '@/utils/formatDurationClock';

interface Props {
  open: boolean;
  record: HistoryRecord | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  close: [];
  cancel: [historyId: number];
}>();

const router = useRouter();
const canReplay = computed(() => Boolean(router));
const canCancel = computed(() =>
  Boolean(
    props.record && props.record.taskType === HistoryTaskType.ModelTraining && [TaskStatus.Pending, TaskStatus.Running].includes(props.record.status)
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
  <BaseDialog
    :open="open"
    :title="record ? `${HISTORY_TASK_TYPE_TEXT[record.taskType]}详情` : '任务详情'"
    panel-class="max-w-4xl"
    content-class="max-h-[60vh] overflow-y-auto pr-2"
    @close="emit('close')"
  >
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

      <section v-if="record.errorMessage" class="rounded-2xl border border-rose-200 bg-rose-50/70 p-4">
        <p class="text-sm font-semibold text-rose-900">失败原因</p>
        <div class="mt-3 h-40 overflow-y-auto rounded-xl bg-white/85 px-3 py-3 text-sm leading-6 text-rose-900">
          {{ record.errorMessage }}
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

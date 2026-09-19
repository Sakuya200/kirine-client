<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { ArrowPathIcon, EyeIcon, TrashIcon } from '@heroicons/vue/24/outline';
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseDialog from '@/components/common/BaseDialog.vue';
import BaseLoadingBanner from '@/components/common/BaseLoadingBanner.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import BasePagination from '@/components/common/BasePagination.vue';
import HistoryTaskDetailDialog from '@/components/history/HistoryTaskDetailDialog.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import StatusPill from '@/components/common/StatusPill.vue';
import { STATUS_TEXT_KEY, TaskStatus } from '@/enums/status';
import { HISTORY_TASK_TYPE_TEXT_KEY, HistoryTaskType } from '@/enums/task';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { usePagination } from '@/hooks/usePagination';
import { useUiStore } from '@/stores/ui';
import type { HistoryFilter, HistoryRecordSummary } from '@/types/domain';
import { formatDurationClock } from '@/utils/formatDurationClock';

type TaskTypeFilterValue = 'all' | HistoryTaskType;
type StatusFilterValue = 'all' | TaskStatus;

const { t } = useI18n();

const taskTypeOptions = computed<Array<{ value: TaskTypeFilterValue; label: string }>>(() => [
  { value: 'all', label: t('common.allTaskTypes') },
  { value: HistoryTaskType.ModelTraining, label: t(HISTORY_TASK_TYPE_TEXT_KEY[HistoryTaskType.ModelTraining]) },
  { value: HistoryTaskType.TextToSpeech, label: t(HISTORY_TASK_TYPE_TEXT_KEY[HistoryTaskType.TextToSpeech]) },
  { value: HistoryTaskType.VoiceClone, label: t(HISTORY_TASK_TYPE_TEXT_KEY[HistoryTaskType.VoiceClone]) },
  { value: HistoryTaskType.VoiceDesign, label: t(HISTORY_TASK_TYPE_TEXT_KEY[HistoryTaskType.VoiceDesign]) }
]);

const statusOptions = computed<Array<{ value: StatusFilterValue; label: string }>>(() => [
  { value: 'all', label: t('common.allStatuses') },
  { value: TaskStatus.Pending, label: t(STATUS_TEXT_KEY[TaskStatus.Pending]) },
  { value: TaskStatus.Running, label: t(STATUS_TEXT_KEY[TaskStatus.Running]) },
  { value: TaskStatus.Completed, label: t(STATUS_TEXT_KEY[TaskStatus.Completed]) },
  { value: TaskStatus.Cancelled, label: t(STATUS_TEXT_KEY[TaskStatus.Cancelled]) },
  { value: TaskStatus.Failed, label: t(STATUS_TEXT_KEY[TaskStatus.Failed]) }
]);

const selectedTaskType = ref<TaskTypeFilterValue>('all');
const selectedStatus = ref<StatusFilterValue>('all');
const searchKeyword = ref('');
const selectedRecordId = ref<number | null>(null);
const deleteTargetId = ref<number | null>(null);
const isMutating = ref(false);
const uiStore = useUiStore();

const filter = ref<HistoryFilter>({ keyword: null, taskType: null, status: null });

const {
  items: rows,
  total,
  page,
  pageSize,
  loading: isLoading,
  refresh: loadHistory,
  setPage,
  setPageSize,
  setFilter
} = usePagination<HistoryRecordSummary, HistoryFilter>({
  command: 'list_history_records',
  filter,
  initialPageSize: 10,
  errorLabel: t('history.notice.loadFailed')
});

const deleteTarget = computed(() => rows.value.find(row => row.id === deleteTargetId.value) ?? null);
const historyBusyLabel = computed(() => {
  if (isMutating.value) {
    return t('history.notice.updating');
  }

  if (isLoading.value) {
    return t('history.notice.loading');
  }

  return '';
});

const onKeywordInput = () => {
  setFilter({ keyword: searchKeyword.value.trim() });
};

const onTaskTypeChange = (value: TaskTypeFilterValue) => {
  setFilter({ taskType: value === 'all' ? null : value });
};

const onStatusChange = (value: StatusFilterValue) => {
  setFilter({ status: value === 'all' ? null : value });
};

const requestDelete = (record: HistoryRecordSummary) => {
  deleteTargetId.value = record.id;
};

const closeDeleteDialog = () => {
  deleteTargetId.value = null;
};

const confirmDelete = async () => {
  if (!deleteTarget.value) {
    return;
  }

  const target = deleteTarget.value;
  const removedId = target.id;
  const removedTaskType = target.taskType;
  const removedTitle = target.title;

  isMutating.value = true;

  try {
    const deleted = await invoke<boolean>('delete_history_record', {
      historyId: removedId,
      taskType: removedTaskType
    });

    if (!deleted) {
      uiStore.notifyError(t('history.notice.deleteFailed'));
      return;
    }

    if (selectedRecordId.value === removedId) {
      closeDetail();
    }

    uiStore.notifySuccess(t('history.notice.deleted', { title: removedTitle }), 3200);
    closeDeleteDialog();
    await loadHistory();
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('history.notice.deleteError'), error));
  } finally {
    isMutating.value = false;
  }
};

const openDetail = (record: HistoryRecordSummary) => {
  selectedRecordId.value = record.id;
};

const closeDetail = () => {
  selectedRecordId.value = null;
};

const cancelTask = async (historyId: number) => {
  isMutating.value = true;

  try {
    const accepted = await invoke<boolean>('cancel_history_task', { historyId });
    if (!accepted) {
      uiStore.notifyWarning(t('tts.notice.alreadyCancelling'));
      return;
    }

    await loadHistory();
    uiStore.notifySuccess(t('history.notice.cancelRequested', { taskId: historyId }), 2600);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('tts.notice.cancelFailed'), error));
  } finally {
    isMutating.value = false;
  }
};

onMounted(async () => {
  await loadHistory();
});
</script>

<template>
  <div class="space-y-5">
    <PageHeader
      :title="t('history.title')"
      :description="t('history.description')"
      eyebrow="Task History"
    />

    <BaseLoadingBanner v-if="historyBusyLabel" :label="historyBusyLabel" />

    <PanelCard :title="t('history.panel.title')" :subtitle="t('history.panel.subtitle')">
      <template #actions>
        <BaseButton tone="ghost" :loading="isLoading" @click="loadHistory">
          <ArrowPathIcon v-if="!isLoading" class="h-4 w-4" aria-hidden="true" />
          <span>{{ isLoading ? t('history.panel.refreshing') : t('history.panel.refresh') }}</span>
        </BaseButton>
      </template>

      <div class="mb-4 grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_minmax(0,1fr)]">
        <input
          v-model="searchKeyword"
          class="min-w-0 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 sm:col-span-2 xl:col-span-1"
          :placeholder="t('history.panel.searchPlaceholder')"
          @input="onKeywordInput"
        />
        <BaseListbox :model-value="selectedTaskType" :options="taskTypeOptions" @update:model-value="onTaskTypeChange($event as TaskTypeFilterValue)" />
        <BaseListbox :model-value="selectedStatus" :options="statusOptions" @update:model-value="onStatusChange($event as StatusFilterValue)" />
      </div>

      <div class="overflow-x-auto">
        <table class="w-full min-w-[820px] text-left text-sm">
          <thead>
            <tr class="border-b border-brand-100 text-xs uppercase tracking-wide text-stone-500">
              <th class="pb-2">{{ t('history.table.taskId') }}</th>
              <th class="pb-2">{{ t('history.table.taskName') }}</th>
              <th class="pb-2">{{ t('history.table.type') }}</th>
              <th class="pb-2">{{ t('history.table.speaker') }}</th>
              <th class="pb-2">{{ t('history.table.status') }}</th>
              <th class="pb-2">{{ t('history.table.duration') }}</th>
              <th class="pb-2">{{ t('history.table.createTime') }}</th>
              <th class="pb-2">{{ t('history.table.actions') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in rows" :key="row.id" class="border-b border-brand-50 text-slate-700">
              <td class="py-3 font-mono text-xs">{{ row.id }}</td>
              <td class="py-3">{{ row.title }}</td>
              <td class="py-3">{{ t(HISTORY_TASK_TYPE_TEXT_KEY[row.taskType]) }}</td>
              <td class="py-3">{{ row.speaker }}</td>
              <td class="py-3"><StatusPill :status="row.status" /></td>
              <td class="py-3">{{ formatDurationClock(row.durationSeconds) }}</td>
              <td class="py-3">{{ row.createTime }}</td>
              <td class="py-3">
                <div class="flex flex-wrap gap-2">
                  <BaseButton tone="ghost" size="sm" @click="openDetail(row)">
                    <EyeIcon class="h-4 w-4" aria-hidden="true" />
                    <span>{{ t('history.table.view') }}</span>
                  </BaseButton>
                  <BaseButton tone="quiet" size="sm" :disabled="isMutating" @click="requestDelete(row)">
                    <TrashIcon class="h-4 w-4" aria-hidden="true" />
                    <span>{{ t('history.table.delete') }}</span>
                  </BaseButton>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-if="rows.length === 0" class="mt-4 rounded-2xl border border-dashed border-brand-200 bg-white/85 p-5 text-sm text-stone-500">
        {{ isLoading ? t('history.panel.loading') : t('history.panel.empty') }}
      </div>

      <div v-if="rows.length > 0" class="mt-4">
        <BasePagination
          :current-page="page"
          :page-size="pageSize"
          :total-items="total"
          :disabled="isMutating"
          :loading="isLoading"
          @update:current-page="setPage"
          @update:page-size="setPageSize"
        />
      </div>
    </PanelCard>

    <HistoryTaskDetailDialog :open="selectedRecordId !== null" :record-id="selectedRecordId" @close="closeDetail" @cancel="cancelTask" />

    <BaseDialog :open="deleteTarget !== null" :title="t('history.deleteDialog.title')" @close="closeDeleteDialog">
      <p class="text-sm text-slate-600">
        <template v-if="deleteTarget">{{ t('history.deleteDialog.confirm', { title: deleteTarget.title }) }}</template>
        <template v-else>{{ t('history.deleteDialog.notFound') }}</template>
      </p>
      <template #footer>
        <BaseButton tone="ghost" @click="closeDeleteDialog">
          <span>{{ t('common.cancel') }}</span>
        </BaseButton>
        <BaseButton tone="quiet" :loading="isMutating" :disabled="!deleteTarget || isMutating" @click="confirmDelete">
          <span>{{ isMutating ? t('history.deleteDialog.deleting') : t('history.deleteDialog.confirmDelete') }}</span>
        </BaseButton>
      </template>
    </BaseDialog>
  </div>
</template>

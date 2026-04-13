<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { ArrowPathIcon, EyeIcon, TrashIcon } from '@heroicons/vue/24/outline';
import { computed, onMounted, ref } from 'vue';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseDialog from '@/components/common/BaseDialog.vue';
import BaseLoadingBanner from '@/components/common/BaseLoadingBanner.vue';
import BaseLoadingIndicator from '@/components/common/BaseLoadingIndicator.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import HistoryTaskDetailDialog from '@/components/history/HistoryTaskDetailDialog.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import StatusPill from '@/components/common/StatusPill.vue';
import { TaskStatus } from '@/enums/status';
import { HISTORY_TASK_TYPE_TEXT, HistoryTaskType } from '@/enums/task';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useUiStore } from '@/stores/ui';
import type { HistoryRecord } from '@/types/domain';
import { formatDurationClock } from '@/utils/formatDurationClock';

type TaskTypeFilterValue = 'all' | HistoryTaskType;
type StatusFilterValue = 'all' | TaskStatus;

const taskTypeOptions: Array<{ value: TaskTypeFilterValue; label: string }> = [
  { value: 'all', label: '全部任务类型' },
  { value: HistoryTaskType.ModelTraining, label: HISTORY_TASK_TYPE_TEXT[HistoryTaskType.ModelTraining] },
  { value: HistoryTaskType.TextToSpeech, label: HISTORY_TASK_TYPE_TEXT[HistoryTaskType.TextToSpeech] },
  { value: HistoryTaskType.VoiceClone, label: HISTORY_TASK_TYPE_TEXT[HistoryTaskType.VoiceClone] }
];

const statusOptions: Array<{ value: StatusFilterValue; label: string }> = [
  { value: 'all', label: '全部状态' },
  { value: TaskStatus.Pending, label: '待执行' },
  { value: TaskStatus.Running, label: '执行中' },
  { value: TaskStatus.Completed, label: '已完成' },
  { value: TaskStatus.Failed, label: '失败' }
];

const selectedTaskType = ref<TaskTypeFilterValue>(taskTypeOptions[0].value);
const selectedStatus = ref<StatusFilterValue>(statusOptions[0].value);
const searchKeyword = ref('');
const selectedRecordId = ref<number | null>(null);
const deleteTargetId = ref<number | null>(null);
const rows = ref<HistoryRecord[]>([]);
const isLoading = ref(false);
const isMutating = ref(false);
const uiStore = useUiStore();

const selectedRecord = computed(() => rows.value.find(row => row.id === selectedRecordId.value) ?? null);
const deleteTarget = computed(() => rows.value.find(row => row.id === deleteTargetId.value) ?? null);
const trimmedKeyword = computed(() => searchKeyword.value.trim().toLowerCase());
const filteredRows = computed(() =>
  rows.value.filter(row => {
    const matchesKeyword =
      !trimmedKeyword.value ||
      String(row.id).toLowerCase().includes(trimmedKeyword.value) ||
      row.title.toLowerCase().includes(trimmedKeyword.value) ||
      row.speaker.toLowerCase().includes(trimmedKeyword.value);
    const matchesTaskType = selectedTaskType.value === 'all' || row.taskType === selectedTaskType.value;
    const matchesStatus = selectedStatus.value === 'all' || row.status === selectedStatus.value;

    return matchesKeyword && matchesTaskType && matchesStatus;
  })
);
const historyBusyLabel = computed(() => {
  if (isMutating.value) {
    return '正在更新历史任务，请稍候';
  }

  if (isLoading.value) {
    return '正在加载历史任务列表';
  }

  return '';
});

const loadHistory = async () => {
  isLoading.value = true;

  try {
    rows.value = await invoke<HistoryRecord[]>('list_history_records');
  } catch (error) {
    rows.value = [];
    uiStore.notifyError(formatErrorMessage('读取历史任务失败，请检查本地数据库或 Rust 后端', error));
  } finally {
    isLoading.value = false;
  }
};

const requestDelete = (record: HistoryRecord) => {
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
      uiStore.notifyError('删除历史任务失败。');
      return;
    }

    rows.value = rows.value.filter(row => row.id !== removedId);

    if (selectedRecordId.value === removedId) {
      closeDetail();
    }

    uiStore.notifySuccess(`任务 ${removedTitle} 已删除。`, 3200);
    closeDeleteDialog();
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('删除历史任务失败', error));
  } finally {
    isMutating.value = false;
  }
};

const openDetail = (record: HistoryRecord) => {
  selectedRecordId.value = record.id;
};

const closeDetail = () => {
  selectedRecordId.value = null;
};

onMounted(async () => {
  await loadHistory();
});
</script>

<template>
  <div class="space-y-5">
    <PageHeader title="历史任务" description="统一查看模型训练、文本转语音与声音克隆任务，支持筛选、搜索、详情查看与删除。" eyebrow="Task History" />

    <BaseLoadingBanner v-if="historyBusyLabel" :label="historyBusyLabel" />

    <PanelCard title="任务列表" subtitle="统一展示模型训练、文本转语音与声音克隆任务，数据来自本地数据库。">
      <template #actions>
        <BaseButton tone="ghost" :disabled="isLoading" @click="loadHistory">
          <BaseLoadingIndicator v-if="isLoading" size="sm" tone="muted" />
          <ArrowPathIcon v-else class="h-4 w-4" aria-hidden="true" />
          <span>{{ isLoading ? '刷新中...' : '刷新列表' }}</span>
        </BaseButton>
      </template>

      <div class="mb-4 grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_minmax(0,1fr)]">
        <input
          v-model="searchKeyword"
          class="min-w-0 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 sm:col-span-2 xl:col-span-1"
          placeholder="按任务ID、标题或说话人搜索"
        />
        <BaseListbox v-model="selectedTaskType" :options="taskTypeOptions" />
        <BaseListbox v-model="selectedStatus" :options="statusOptions" />
      </div>

      <div class="overflow-x-auto">
        <table class="w-full min-w-[820px] text-left text-sm">
          <thead>
            <tr class="border-b border-brand-100 text-xs uppercase tracking-wide text-stone-500">
              <th class="pb-2">任务ID</th>
              <th class="pb-2">任务名称</th>
              <th class="pb-2">类型</th>
              <th class="pb-2">说话人</th>
              <th class="pb-2">状态</th>
              <th class="pb-2">耗时</th>
              <th class="pb-2">创建时间</th>
              <th class="pb-2">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in filteredRows" :key="row.id" class="border-b border-brand-50 text-slate-700">
              <td class="py-3 font-mono text-xs">{{ row.id }}</td>
              <td class="py-3">{{ row.title }}</td>
              <td class="py-3">{{ HISTORY_TASK_TYPE_TEXT[row.taskType] }}</td>
              <td class="py-3">{{ row.speaker }}</td>
              <td class="py-3"><StatusPill :status="row.status" /></td>
              <td class="py-3">{{ formatDurationClock(row.durationSeconds) }}</td>
              <td class="py-3">{{ row.createTime }}</td>
              <td class="py-3">
                <div class="flex flex-wrap gap-2">
                  <BaseButton tone="ghost" size="sm" @click="openDetail(row)">
                    <EyeIcon class="h-4 w-4" aria-hidden="true" />
                    <span>查看</span>
                  </BaseButton>
                  <BaseButton tone="quiet" size="sm" :disabled="isMutating" @click="requestDelete(row)">
                    <TrashIcon class="h-4 w-4" aria-hidden="true" />
                    <span>删除</span>
                  </BaseButton>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-if="filteredRows.length === 0" class="mt-4 rounded-2xl border border-dashed border-brand-200 bg-white/85 p-5 text-sm text-stone-500">
        {{ isLoading ? '正在加载历史任务...' : '当前筛选条件下没有匹配的历史任务。' }}
      </div>
    </PanelCard>

    <HistoryTaskDetailDialog :open="selectedRecord !== null" :record="selectedRecord" @close="closeDetail" />

    <BaseDialog :open="deleteTarget !== null" title="删除历史任务" @close="closeDeleteDialog">
      <p class="text-sm text-slate-600">
        <template v-if="deleteTarget">将删除任务“{{ deleteTarget.title }}”，该操作会同步逻辑删除关联详情记录。</template>
        <template v-else>未找到要删除的历史任务。</template>
      </p>
      <template #footer>
        <BaseButton tone="ghost" @click="closeDeleteDialog">
          <span>取消</span>
        </BaseButton>
        <BaseButton tone="quiet" :disabled="!deleteTarget || isMutating" @click="confirmDelete">
          <BaseLoadingIndicator v-if="isMutating" size="sm" tone="muted" />
          <span>{{ isMutating ? '删除中...' : '确认删除' }}</span>
        </BaseButton>
      </template>
    </BaseDialog>
  </div>
</template>

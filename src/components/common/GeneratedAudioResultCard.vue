<script setup lang="ts">
import { EyeIcon } from '@heroicons/vue/24/outline';
import { computed, ref, useSlots } from 'vue';

import AudioResultPlayer from '@/components/common/AudioResultPlayer.vue';
import BaseButton from '@/components/common/BaseButton.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import StatusPill from '@/components/common/StatusPill.vue';
import HistoryTaskDetailDialog from '@/components/history/HistoryTaskDetailDialog.vue';
import { TaskStatus } from '@/enums/status';

interface AudioAssetPayload {
  fileName: string;
  contentType: string;
  bytes: number[];
}

interface GeneratedAudioResultCardTask {
  taskId: number;
  fileName: string;
  status: TaskStatus;
}

interface Props {
  result: GeneratedAudioResultCardTask | null;
  metaText: string;
  emptyText: string;
  title?: string;
  subtitle?: string;
  loadAudioAsset: (taskId: number) => Promise<AudioAssetPayload>;
  downloadAudio?: (taskId: number) => Promise<boolean>;
}

const props = withDefaults(defineProps<Props>(), {
  title: '生成结果',
  subtitle: '展示最近一次任务的返回结果和输出文件信息'
});

const slots = useSlots();
const detailRecordId = ref<number | null>(null);
const detailReloadToken = ref(0);
const isDetailLoading = ref(false);

const canViewDetail = computed(() => Boolean(props.result));
const hasActionsSlot = computed(() => Boolean(slots.actions));

const openDetail = async () => {
  if (!props.result) {
    return;
  }

  isDetailLoading.value = true;
  detailRecordId.value = props.result.taskId;
  detailReloadToken.value += 1;
  isDetailLoading.value = false;
};

const closeDetail = () => {
  detailRecordId.value = null;
};

const refreshDetailRecord = async () => {
  if (detailRecordId.value === null) {
    return;
  }

  detailReloadToken.value += 1;
};

defineExpose({
  refreshDetailRecord
});
</script>

<template>
  <PanelCard :title="title" :subtitle="subtitle">
    <div v-if="result" class="surface-grid rounded-2xl border border-brand-200 bg-white/82 p-4">
      <div class="flex items-start justify-between gap-3">
        <div>
          <p class="text-sm font-medium text-slate-700">{{ result.fileName }}</p>
          <p class="mt-1 text-xs text-stone-500">{{ metaText }}</p>
        </div>
        <div class="flex items-center gap-2">
          <BaseButton v-if="canViewDetail" tone="ghost" size="sm" :loading="isDetailLoading" @click="openDetail">
            <EyeIcon v-if="!isDetailLoading" class="h-4 w-4" aria-hidden="true" />
            <span>{{ isDetailLoading ? '载入中...' : '查看详情' }}</span>
          </BaseButton>
          <StatusPill :status="result.status" />
        </div>
      </div>

      <div class="mt-3">
        <AudioResultPlayer
          :task="result"
          :load-audio-asset="loadAudioAsset"
          :download-audio="downloadAudio"
          download-label="下载"
          download-tone="ghost"
        />
      </div>

      <div class="mt-3 rounded-2xl border border-brand-200 bg-white/80 p-3 text-xs text-stone-600">
        <slot name="details">
          <p>任务 ID：{{ result.taskId }}</p>
        </slot>
      </div>

      <div v-if="hasActionsSlot" class="mt-4 flex flex-wrap gap-2">
        <slot name="actions" />
      </div>
    </div>

    <div v-else class="rounded-2xl border border-dashed border-brand-200 bg-white/82 p-5 text-sm text-stone-500">
      {{ emptyText }}
    </div>

    <HistoryTaskDetailDialog :open="detailRecordId !== null" :record-id="detailRecordId" :reload-token="detailReloadToken" @close="closeDetail" />
  </PanelCard>
</template>

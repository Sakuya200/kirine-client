<script setup lang="ts">
import BaseButton from '@/components/common/BaseButton.vue';
import StatusPill from '@/components/common/StatusPill.vue';
import type { TaskStatus } from '@/enums/status';

export interface RecentTaskListItem {
  taskId: number;
  title: string;
  subtitle: string;
  status: TaskStatus;
}

interface Props {
  items: RecentTaskListItem[];
  emptyText: string;
  actionLabel?: string;
  selectedTaskId?: number | null;
}

withDefaults(defineProps<Props>(), {
  actionLabel: '查看',
  selectedTaskId: null
});

const emit = defineEmits<{
  select: [taskId: number];
}>();
</script>

<template>
  <div class="space-y-3">
    <article
      v-for="item in items"
      :key="item.taskId"
      class="flex flex-wrap items-center justify-between gap-3 rounded-2xl border p-4 transition-colors"
      :class="item.taskId === selectedTaskId ? 'border-brand-400 bg-brand-50/70' : 'border-brand-200 bg-white/85'"
    >
      <div class="min-w-0 flex-1">
        <p class="truncate text-sm font-semibold text-slate-800">{{ item.title }}</p>
        <p class="mt-1 truncate text-xs text-stone-500">{{ item.subtitle }}</p>
      </div>
      <div class="flex flex-wrap items-center gap-2">
        <StatusPill :status="item.status" />
        <BaseButton tone="ghost" size="sm" @click="emit('select', item.taskId)">
          <span>{{ actionLabel }}</span>
        </BaseButton>
      </div>
    </article>

    <div v-if="items.length === 0" class="rounded-2xl border border-dashed border-brand-200 bg-white/80 p-5 text-sm text-stone-500">
      {{ emptyText }}
    </div>
  </div>
</template>

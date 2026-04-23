<script setup lang="ts">
import { ArrowLeftIcon } from '@heroicons/vue/24/outline';

import { HISTORY_TASK_ROUTE_PATH, HistoryTaskType } from '@/enums/task';
</script>

<template>
  <div class="flex min-h-[60vh] items-center justify-center">
    <div class="rounded-2xl border border-brand-100 bg-white/88 p-8 text-center shadow-soft">
      <p class="text-sm uppercase tracking-widest text-stone-500">404</p>
      <h1 class="mt-2 text-2xl font-semibold text-slate-900">页面不存在</h1>
      <p class="mt-2 text-sm text-slate-600">请从左侧导航返回有效页面。</p>
      <RouterLink
        :to="HISTORY_TASK_ROUTE_PATH[HistoryTaskType.ModelTraining]"
        class="mt-4 inline-flex items-center gap-2 rounded-xl bg-brand-600 px-4 py-2 text-sm font-semibold text-white hover:bg-brand-700"
      >
        <ArrowLeftIcon class="h-4 w-4" aria-hidden="true" />
        <span>回到模型微调</span>
      </RouterLink>
    </div>
  </div>
</template>

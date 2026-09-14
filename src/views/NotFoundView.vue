<script setup lang="ts">
import { useI18n } from 'vue-i18n';

import { ArrowLeftIcon } from '@heroicons/vue/24/outline';

import { HISTORY_TASK_ROUTE_PATH, HistoryTaskType } from '@/enums/task';

const { t } = useI18n();
</script>

<template>
  <div class="flex min-h-[60vh] items-center justify-center">
    <div class="rounded-2xl border border-brand-100 bg-white/88 p-8 text-center shadow-soft">
      <p class="text-sm uppercase tracking-widest text-stone-500">404</p>
      <h1 class="mt-2 text-2xl font-semibold text-slate-900">{{ t('common.notFound.title') }}</h1>
      <p class="mt-2 text-sm text-slate-600">{{ t('common.notFound.description') }}</p>
      <RouterLink
        :to="HISTORY_TASK_ROUTE_PATH[HistoryTaskType.ModelTraining]"
        class="mt-4 inline-flex items-center gap-2 rounded-xl bg-brand-600 px-4 py-2 text-sm font-semibold text-white hover:bg-brand-700"
      >
        <ArrowLeftIcon class="h-4 w-4" aria-hidden="true" />
        <span>{{ t('common.notFound.backToTraining') }}</span>
      </RouterLink>
    </div>
  </div>
</template>

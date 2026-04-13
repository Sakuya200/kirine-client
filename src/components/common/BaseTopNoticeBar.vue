<script setup lang="ts">
import { CheckCircleIcon, ExclamationCircleIcon, ExclamationTriangleIcon, InformationCircleIcon, XMarkIcon } from '@heroicons/vue/24/outline';
import type { Component } from 'vue';

import { useUiStore } from '@/stores/ui';

const uiStore = useUiStore();

const toneClassMap = {
  success: 'border-emerald-200/90 bg-emerald-50/95 text-emerald-900 shadow-[0_16px_34px_rgba(16,185,129,0.16)]',
  error: 'border-rose-200/90 bg-rose-50/95 text-rose-900 shadow-[0_16px_34px_rgba(244,63,94,0.16)]',
  info: 'border-brand-200/90 bg-white/96 text-slate-800 shadow-[0_18px_36px_rgba(186,116,56,0.14)]',
  warning: 'border-amber-200/90 bg-amber-50/95 text-amber-900 shadow-[0_16px_34px_rgba(245,158,11,0.16)]'
} as const;

const iconClassMap = {
  success: 'bg-inherit text-emerald-600',
  error: 'bg-inherit text-rose-600',
  info: 'bg-inherit text-brand-600',
  warning: 'bg-inherit text-amber-600'
} as const;

const iconMap: Record<'success' | 'error' | 'info' | 'warning', Component> = {
  success: CheckCircleIcon,
  error: ExclamationCircleIcon,
  info: InformationCircleIcon,
  warning: ExclamationTriangleIcon
};
</script>

<template>
  <TransitionGroup name="notice-stack" tag="div" class="pointer-events-none flex flex-col items-center gap-3">
    <div
      v-for="notice in uiStore.notices"
      :key="notice.id"
      class="pointer-events-auto w-fit max-w-full overflow-hidden rounded-2xl border backdrop-blur"
      :class="toneClassMap[notice.tone]"
    >
      <div class="flex items-center gap-3 px-4 py-3 md:px-5">
        <div class="shrink-0 rounded-xl p-2" :class="iconClassMap[notice.tone]">
          <component :is="iconMap[notice.tone]" class="h-5 w-5" aria-hidden="true" />
        </div>
        <p class="min-w-0 max-w-[min(72vw,42rem)] text-sm leading-6">{{ notice.message }}</p>
        <button
          type="button"
          class="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-inherit text-stone-500 transition hover:bg-black/5 hover:text-slate-800"
          @click="uiStore.removeNotice(notice.id)"
        >
          <XMarkIcon class="h-4 w-4" aria-hidden="true" />
        </button>
      </div>
    </div>
  </TransitionGroup>
</template>

<style scoped>
.notice-stack-enter-active,
.notice-stack-leave-active,
.notice-stack-move {
  transition:
    opacity 220ms ease,
    transform 220ms ease;
}

.notice-stack-enter-from,
.notice-stack-leave-to {
  opacity: 0;
  transform: translateY(-10px) scale(0.98);
}
</style>

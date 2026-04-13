<script setup lang="ts">
import {
  ChevronDoubleLeftIcon,
  ChevronDoubleRightIcon,
  ClockIcon,
  Cog6ToothIcon,
  MicrophoneIcon,
  MusicalNoteIcon,
  SpeakerWaveIcon,
  UserGroupIcon
} from '@heroicons/vue/24/outline';
import type { Component } from 'vue';
import { RouterLink, RouterView, useRoute } from 'vue-router';

import BaseTopNoticeBar from '@/components/common/BaseTopNoticeBar.vue';
import { HistoryTaskType } from '@/enums/task';
import { appRoutes } from '@/routers';
import { useUiStore } from '@/stores/ui';

const route = useRoute();
const uiStore = useUiStore();

const menuItems = appRoutes.filter(item => item.name && item.name !== 'not-found');

const navIcons: Record<string, Component> = {
  [HistoryTaskType.ModelTraining]: MicrophoneIcon,
  [HistoryTaskType.TextToSpeech]: SpeakerWaveIcon,
  [HistoryTaskType.VoiceClone]: MusicalNoteIcon,
  speakers: UserGroupIcon,
  history: ClockIcon,
  settings: Cog6ToothIcon
};
</script>

<template>
  <div class="min-h-screen bg-[#fffbf7]">
    <aside
      class="fixed inset-y-0 left-0 z-30 overflow-y-auto overflow-x-hidden border-r border-[#8b5a36]/35 bg-[#fff6ef] py-5 text-stone-800 transition-all duration-300"
      :class="[uiStore.sidebarWidthClass, uiStore.sidebarCollapsed ? 'px-1.5' : 'px-4']"
    >
      <div class="pointer-events-none absolute -top-20 left-16 h-52 w-52 rounded-full bg-brand-300/24 blur-3xl" />
      <div class="pointer-events-none absolute bottom-8 -left-10 h-44 w-44 rounded-full bg-orange-200/20 blur-3xl" />
      <div
        class="relative mb-7 transition-all duration-300"
        :class="
          uiStore.sidebarCollapsed
            ? 'flex items-center justify-center p-0'
            : 'flex items-start justify-between gap-3 rounded-2xl border border-brand-200/80 bg-white/80 p-3 shadow-[0_14px_28px_rgba(186,116,56,0.08)]'
        "
      >
        <div v-if="!uiStore.sidebarCollapsed" class="min-w-0 flex-1">
          <p class="text-[11px] uppercase tracking-[0.26em] text-brand-500">Kirine</p>
          <p class="text-sm font-semibold text-slate-900">Audio Workbench</p>
        </div>

        <button
          class="inline-flex h-9 w-9 items-center justify-center text-xs text-brand-700 transition hover:text-brand-900"
          @click="uiStore.toggleSidebar"
        >
          <ChevronDoubleRightIcon v-if="uiStore.sidebarCollapsed" class="h-4 w-4" aria-hidden="true" />
          <ChevronDoubleLeftIcon v-else class="h-4 w-4" aria-hidden="true" />
        </button>
      </div>

      <nav class="relative space-y-2">
        <RouterLink
          v-for="item in menuItems"
          :key="String(item.name)"
          :to="item.path"
          class="group text-sm font-medium transition-all duration-200"
          :class="[
            uiStore.sidebarCollapsed
              ? 'flex h-11 w-full items-center justify-center px-0 py-0'
              : 'flex items-center gap-3 rounded-xl border px-3 py-2',
            uiStore.sidebarCollapsed
              ? route.path === item.path
                ? 'text-brand-600'
                : 'text-stone-500 hover:text-brand-700'
              : route.path === item.path
                ? 'bg-brand-500 text-white shadow-soft'
                : 'text-stone-600 hover:bg-white hover:text-slate-900'
          ]"
        >
          <span
            class="inline-flex items-center justify-center rounded-lg text-xs font-semibold"
            :class="[
              uiStore.sidebarCollapsed ? 'h-9 w-9 rounded-none' : 'h-7 w-7',
              uiStore.sidebarCollapsed
                ? route.path === item.path
                  ? 'bg-transparent text-brand-600'
                  : 'bg-transparent text-current'
                : route.path === item.path
                  ? 'bg-white/20 text-white'
                  : 'bg-brand-50 text-brand-700 group-hover:bg-brand-100'
            ]"
          >
            <component :is="navIcons[String(item.name)]" class="h-4 w-4" aria-hidden="true" />
          </span>
          <span :class="uiStore.sidebarCollapsed ? 'hidden' : 'inline'">{{ item.meta?.title }}</span>
        </RouterLink>
      </nav>
    </aside>

    <main
      class="surface-grid min-h-screen p-5 transition-all duration-300 md:p-7"
      :style="{ marginLeft: `${uiStore.sidebarWidth}px`, width: `calc(100% - ${uiStore.sidebarWidth}px)` }"
    >
      <div
        class="pointer-events-none fixed top-5 z-40 px-5 transition-all duration-300 md:top-7 md:px-7"
        :style="{ left: `${uiStore.sidebarWidth}px`, width: `calc(100% - ${uiStore.sidebarWidth}px)` }"
      >
        <div class="mx-auto max-w-7xl">
          <BaseTopNoticeBar />
        </div>
      </div>

      <div class="mx-auto max-w-7xl">
        <div class="page-enter">
          <RouterView />
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { RouterLink, useRoute } from 'vue-router';

import BaseLoadingIndicator from '@/components/common/BaseLoadingIndicator.vue';

interface Props {
  label: string;
}

defineProps<Props>();

const route = useRoute();
const showHistoryLink = computed(() => route.path !== '/history');
</script>

<template>
  <section class="rounded-2xl border border-brand-100/80 bg-white/88 px-5 py-4 shadow-soft backdrop-blur-sm">
    <div class="flex min-h-16 flex-col items-center justify-center gap-2 text-center">
      <BaseLoadingIndicator size="lg" :label="label" />
      <p class="max-w-2xl text-xs leading-5 text-stone-500">
        <template v-if="showHistoryLink">
          可前往
          <RouterLink to="/history" class="font-medium text-brand-700 underline decoration-brand-300 underline-offset-2 hover:text-brand-800">
            历史任务
          </RouterLink>
          页面随时查询执行结果。
        </template>
        <template v-else>当前页面会持续展示任务状态与执行结果。</template>
      </p>
    </div>
  </section>
</template>

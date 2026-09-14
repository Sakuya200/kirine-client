<script setup lang="ts">
import { RouterLink } from 'vue-router';
import { useI18n } from 'vue-i18n';

import BaseLoadingIndicator from '@/components/common/BaseLoadingIndicator.vue';

interface Props {
  label: string;
  showHistoryLink?: boolean;
}

withDefaults(defineProps<Props>(), {
  showHistoryLink: true
});

const { t } = useI18n();
</script>

<template>
  <section class="rounded-2xl border border-brand-100/80 bg-white/88 px-5 py-4 shadow-soft backdrop-blur-sm">
    <div class="flex min-h-16 flex-col items-center justify-center gap-2 text-center">
      <BaseLoadingIndicator size="lg" :label="label" />
      <p class="max-w-2xl text-xs leading-5 text-stone-500">
        <template v-if="showHistoryLink">
          {{ t('common.loadingBanner.historyHintPrefix') }}
          <RouterLink to="/history" class="font-medium text-brand-700 underline decoration-brand-300 underline-offset-2 hover:text-brand-800">
            {{ t('history.title') }}
          </RouterLink>
          {{ t('common.loadingBanner.historyHintSuffix') }}
        </template>
        <template v-else>{{ t('common.loadingBanner.waitHint') }}</template>
      </p>
    </div>
  </section>
</template>

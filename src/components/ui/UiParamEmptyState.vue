<script setup lang="ts">
import { useI18n } from 'vue-i18n';
interface Props {
  title?: string;
  description?: string;
}

const { t } = useI18n();

defineProps<Props>();
</script>

<template>
  <div class="rounded-2xl border border-dashed border-brand-200 bg-brand-50/30 px-4 py-5 text-sm text-stone-600">
    <p class="font-medium text-slate-800">{{ title ?? t('common.ui.emptyParamsTitle') }}</p>
    <p class="mt-1 leading-6">{{ description ?? t('common.ui.emptyParamsDesc') }}</p>
  </div>
</template>

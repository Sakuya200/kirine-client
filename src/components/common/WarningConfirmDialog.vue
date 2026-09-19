<script setup lang="ts">
import { useI18n } from 'vue-i18n';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseDialog from '@/components/common/BaseDialog.vue';

interface Props {
  open: boolean;
  title: string;
  message: string;
  detailLines?: string[];
  confirmLabel?: string;
  cancelLabel?: string;
  loading?: boolean;
}

const { t } = useI18n();

withDefaults(defineProps<Props>(), {
  detailLines: () => [],
  confirmLabel: undefined,
  cancelLabel: undefined,
  loading: false
});

const emit = defineEmits<{
  close: [];
  confirm: [];
}>();
</script>

<template>
  <BaseDialog :open="open" :title="title" panel-class="max-w-xl" @close="emit('close')">
    <div class="space-y-3 text-sm leading-6 text-slate-700">
      <p>{{ message }}</p>
      <div v-if="detailLines.length" class="rounded-2xl border border-amber-200 bg-amber-50/80 px-4 py-3 text-xs text-amber-900">
        <p v-for="line in detailLines" :key="line">{{ line }}</p>
      </div>
    </div>

    <template #footer>
      <BaseButton tone="ghost" :disabled="loading" @click="emit('close')">
        <span>{{ cancelLabel ?? t('common.cancel') }}</span>
      </BaseButton>
      <BaseButton tone="solid" :loading="loading" @click="emit('confirm')">
        <span>{{ confirmLabel ?? t('common.ui.continue') }}</span>
      </BaseButton>
    </template>
  </BaseDialog>
</template>

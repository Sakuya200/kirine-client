<script setup lang="ts">
import { STATUS_STYLES, STATUS_TEXT } from '@/enums/status';
import type { TaskStatus } from '@/enums/status';

interface Props {
  status: TaskStatus;
}

const props = defineProps<Props>();
</script>

<template>
  <span class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-semibold" :class="STATUS_STYLES[props.status]">
    <span class="h-1.5 w-1.5 rounded-full bg-current" />
    {{ STATUS_TEXT[props.status] }}
  </span>
</template>

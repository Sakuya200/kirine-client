<script setup lang="ts">
interface Props {
  label?: string;
  size?: 'sm' | 'md' | 'lg';
  tone?: 'brand' | 'muted';
}

withDefaults(defineProps<Props>(), {
  label: '',
  size: 'md',
  tone: 'brand'
});
</script>

<template>
  <div class="inline-flex items-center justify-center gap-3 align-middle" :class="tone === 'brand' ? 'text-brand-700' : 'text-stone-500'">
    <span
      class="inline-block shrink-0 animate-spin rounded-full border-2 border-current border-t-transparent"
      :class="size === 'sm' ? 'h-3.5 w-3.5' : size === 'lg' ? 'h-7 w-7 border-[3px]' : 'h-5 w-5'"
      aria-hidden="true"
    />
    <span v-if="label" class="inline-flex min-h-7 items-center text-sm font-semibold leading-none">
      {{ label }}
    </span>
  </div>
</template>

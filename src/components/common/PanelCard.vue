<script setup lang="ts">
interface Props {
  title: string;
  subtitle?: string;
}

defineProps<Props>();
</script>

<template>
  <section class="relative overflow-visible rounded-2xl border border-brand-100/70 bg-white/88 p-5 shadow-soft backdrop-blur-sm">
    <div class="pointer-events-none absolute inset-x-5 top-0 h-px bg-gradient-to-r from-transparent via-brand-200 to-transparent" />
    <div class="mb-4 flex items-start justify-between gap-3">
      <div>
        <h2 class="text-base font-semibold tracking-tight text-slate-900">{{ title }}</h2>
        <p v-if="subtitle" class="mt-1 text-xs text-stone-500">{{ subtitle }}</p>
      </div>
      <slot name="actions" />
    </div>
    <slot />
  </section>
</template>

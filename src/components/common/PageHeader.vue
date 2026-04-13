<script setup lang="ts">
interface Props {
  title: string;
  description: string;
  eyebrow?: string;
}

defineProps<Props>();
</script>

<template>
  <header class="relative overflow-hidden rounded-2xl border border-brand-100/70 bg-white/86 p-5 shadow-soft backdrop-blur">
    <div class="absolute inset-x-0 top-0 h-16 bg-gradient-to-r from-brand-100/80 via-brand-100/50 to-transparent" />
    <div class="relative">
      <p
        class="mb-1 inline-flex rounded-full border border-brand-200 bg-brand-50 px-2.5 py-1 text-[11px] font-semibold uppercase tracking-[0.18em] text-brand-700"
      >
        {{ eyebrow ?? 'Workspace Panel' }}
      </p>
      <h1 class="text-2xl font-semibold tracking-tight text-slate-900">{{ title }}</h1>
      <p class="mt-2 max-w-3xl text-sm text-slate-600">{{ description }}</p>
    </div>
  </header>
</template>

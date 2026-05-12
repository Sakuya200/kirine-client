<script setup lang="ts">
interface Props {
  tone?: 'solid' | 'ghost' | 'quiet';
  size?: 'sm' | 'md' | 'lg';
  type?: 'button' | 'submit' | 'reset';
  block?: boolean;
  disabled?: boolean;
  loading?: boolean;
}

withDefaults(defineProps<Props>(), {
  tone: 'solid',
  size: 'md',
  block: false,
  disabled: false,
  loading: false,
  type: 'button'
});
</script>

<template>
  <button
    :type="type"
    :disabled="disabled || loading"
    :aria-busy="loading"
    class="inline-flex items-center justify-center gap-1 rounded-xl font-semibold transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-200 focus-visible:ring-offset-2"
    :class="[
      tone === 'solid'
        ? 'border border-brand-500 bg-brand-500 text-white shadow-[0_8px_18px_rgba(216,115,39,0.22)] hover:-translate-y-0.5 hover:bg-brand-600'
        : tone === 'ghost'
          ? 'border border-brand-200 bg-white/90 text-brand-700 hover:-translate-y-0.5 hover:border-brand-300 hover:bg-brand-50'
          : 'border border-transparent bg-brand-100/75 text-stone-700 hover:bg-brand-200/80',
      size === 'sm' ? 'px-2.5 py-1.5 text-xs' : size === 'lg' ? 'px-4 py-2.5 text-sm' : 'px-3.5 py-2 text-sm',
      block ? 'w-full' : '',
      disabled || loading ? 'cursor-not-allowed opacity-55 hover:translate-y-0 hover:bg-inherit' : ''
    ]"
  >
    <span v-if="loading" class="base-button-spinner" aria-hidden="true" />
    <slot />
  </button>
</template>

<style scoped>
.base-button-spinner {
  width: 0.95rem;
  height: 0.95rem;
  border-radius: 9999px;
  border: 2px solid currentColor;
  border-right-color: transparent;
  animation: base-button-spin 0.7s linear infinite;
}

@keyframes base-button-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
